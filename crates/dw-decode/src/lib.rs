//! Dekodierung von Audiodateien (BIB-02): MP3, WAV, AIFF, FLAC, AAC/M4A, ALAC,
//! OGG Vorbis und Opus. Ergebnis ist immer Stereo in 32-bit-Gleitkomma.

mod mp4;

use dw_core::AudioBuf;
use std::fs::File;
use std::path::Path;
use symphonia::core::audio::SampleBuffer;
use symphonia::core::codecs::{CodecRegistry, DecoderOptions, CODEC_TYPE_AAC, CODEC_TYPE_NULL};
use symphonia::core::errors::Error as SymError;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

#[derive(Debug, thiserror::Error)]
pub enum DecodeError {
    #[error("Datei kann nicht geöffnet werden: {0}")]
    Io(#[from] std::io::Error),
    #[error("Format wird nicht erkannt oder nicht unterstützt: {0}")]
    Unsupported(String),
    #[error("Datei enthält keine Audiospur")]
    NoAudioTrack,
    #[error("Datei ist beschädigt: {0}")]
    Corrupt(String),
}

/// Ergebnis der Dekodierung in der Abtastrate der Datei.
pub struct Decoded {
    pub audio: AudioBuf,
    /// Kanalzahl der Datei (Mono wird auf beide Kanäle gelegt, bei mehr als zwei Kanälen zählen die ersten beiden).
    pub source_channels: usize,
    pub bits_per_sample: Option<u32>,
    /// Pakete, die wegen Fehlern übersprungen wurden.
    pub skipped_packets: usize,
}

fn codecs() -> &'static CodecRegistry {
    use std::sync::OnceLock;
    static REGISTRY: OnceLock<CodecRegistry> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut registry = CodecRegistry::new();
        symphonia::default::register_enabled_codecs(&mut registry);
        registry.register_all::<symphonia_adapter_libopus::OpusDecoder>();
        registry
    })
}

/// Dekodiert eine Datei vollständig. Encoder-Vor- und Nachlauf (MP3, AAC) wird abgeschnitten (5.1).
pub fn decode_file(path: &Path) -> Result<Decoded, DecodeError> {
    let file = File::open(path)?;
    let mss = MediaSourceStream::new(Box::new(file), Default::default());
    let mut hint = Hint::new();
    if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
        hint.with_extension(ext);
    }
    let format_opts = FormatOptions { enable_gapless: true, ..Default::default() };
    let probed = symphonia::default::get_probe()
        .format(&hint, mss, &format_opts, &MetadataOptions::default())
        .map_err(|e| DecodeError::Unsupported(e.to_string()))?;
    let mut format = probed.format;
    let track = format
        .tracks()
        .iter()
        .find(|t| t.codec_params.codec != CODEC_TYPE_NULL)
        .ok_or(DecodeError::NoAudioTrack)?;
    let track_id = track.id;
    let bits_per_sample = track.codec_params.bits_per_sample;
    // AAC in MP4: Der Demuxer meldet den Encoder-Vorlauf nicht; er steht in der Schnittliste der Datei.
    let needs_edit_list = track.codec_params.codec == CODEC_TYPE_AAC && track.codec_params.delay.is_none();
    let expected_frames = track.codec_params.n_frames.unwrap_or(0) as usize;
    let mut decoder = codecs()
        .make(&track.codec_params, &DecoderOptions::default())
        .map_err(|e| DecodeError::Unsupported(e.to_string()))?;

    let mut data: Vec<f32> = Vec::with_capacity(expected_frames * 2);
    let mut sample_buf: Option<SampleBuffer<f32>> = None;
    let mut sample_rate = track.codec_params.sample_rate.unwrap_or(0);
    let mut channels = 0usize;
    let mut skipped = 0usize;

    loop {
        let packet = match format.next_packet() {
            Ok(p) => p,
            Err(SymError::IoError(e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(SymError::ResetRequired) => break,
            Err(e) => {
                if data.is_empty() {
                    return Err(DecodeError::Corrupt(e.to_string()));
                }
                break;
            }
        };
        if packet.track_id() != track_id {
            continue;
        }
        match decoder.decode(&packet) {
            Ok(decoded) => {
                let spec = *decoded.spec();
                sample_rate = spec.rate;
                channels = spec.channels.count();
                let buf = sample_buf.get_or_insert_with(|| SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
                if buf.capacity() < decoded.capacity() * channels {
                    *buf = SampleBuffer::<f32>::new(decoded.capacity() as u64, spec);
                }
                buf.copy_interleaved_ref(decoded);
                let samples = buf.samples();
                match channels {
                    0 => {}
                    1 => {
                        for &s in samples {
                            data.push(s);
                            data.push(s);
                        }
                    }
                    2 => data.extend_from_slice(samples),
                    n => {
                        for frame in samples.chunks_exact(n) {
                            data.push(frame[0]);
                            data.push(frame[1]);
                        }
                    }
                }
            }
            // Einzelne beschädigte Pakete überspringen, statt den ganzen Track zu verwerfen.
            Err(SymError::DecodeError(_)) | Err(SymError::IoError(_)) => skipped += 1,
            Err(e) => return Err(DecodeError::Corrupt(e.to_string())),
        }
    }

    if needs_edit_list {
        if let Ok(Some(edit)) = mp4::audio_edit_info(path) {
            let skip = (edit.delay as usize * 2).min(data.len());
            data.drain(..skip);
            if let Some(valid) = edit.valid {
                data.truncate(valid as usize * 2);
            }
        }
    }

    if data.is_empty() || sample_rate == 0 {
        return Err(DecodeError::Corrupt("keine Audiodaten dekodiert".into()));
    }
    Ok(Decoded { audio: AudioBuf::new(sample_rate, data), source_channels: channels, bits_per_sample, skipped_packets: skipped })
}

/// Wandelt einen Puffer in die Ziel-Abtastrate (unverändert, wenn sie schon passt).
pub fn resample(audio: &AudioBuf, target_rate: u32) -> AudioBuf {
    if audio.sample_rate == target_rate {
        return audio.clone();
    }
    AudioBuf::new(target_rate, dw_dsp::resample::resample_stereo(&audio.data, audio.sample_rate, target_rate))
}

/// Dekodiert eine Datei und liefert sie in der Abtastrate der Engine.
pub fn load_for_engine(path: &Path, engine_rate: u32) -> Result<AudioBuf, DecodeError> {
    let decoded = decode_file(path)?;
    Ok(if decoded.audio.sample_rate == engine_rate { decoded.audio } else { resample(&decoded.audio, engine_rate) })
}
