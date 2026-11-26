//! Ausgabe über die Audio-Schnittstellen des Systems (ENG-01 bis ENG-03): WASAPI und ASIO
//! unter Windows, CoreAudio unter macOS, ALSA und JACK unter Linux (PipeWire über dessen
//! ALSA-/JACK-Schicht).

use crate::host::{ErrorCounter, Processor};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::time::Duration;

#[derive(Debug, thiserror::Error)]
pub enum DeviceError {
    #[error("kein Ausgabegerät gefunden")]
    NoDevice,
    #[error("Gerät „{0}“ nicht gefunden")]
    NotFound(String),
    #[error("Gerät unterstützt die gewünschte Einstellung nicht: {0}")]
    Unsupported(String),
    #[error("Audio-Schnittstelle meldet: {0}")]
    Backend(String),
}

/// Beschreibung eines Ausgabegeräts für die Einstellungen.
#[derive(Clone, Debug, PartialEq)]
pub struct DeviceInfo {
    /// Name der Schnittstelle, z. B. „ALSA“, „WASAPI“, „CoreAudio“.
    pub host: String,
    pub name: String,
    pub is_default: bool,
    pub max_channels: u16,
    pub sample_rates: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutputConfig {
    /// `None` = Standardgerät des Systems.
    pub device: Option<String>,
    pub sample_rate: u32,
    /// Puffergröße in Frames; `None` = Vorgabe des Geräts.
    pub buffer_frames: Option<u32>,
    pub channels: u16,
}

const RATES: [u32; 4] = [44_100, 48_000, 88_200, 96_000];

/// Name des Geräts, wie ihn das System anzeigt.
fn device_name(device: &cpal::Device) -> Option<String> {
    device.description().ok().map(|d| d.name().to_string())
}

/// Listet die Ausgabegeräte aller verfügbaren Schnittstellen.
pub fn list_devices() -> Vec<DeviceInfo> {
    let mut out = Vec::new();
    for host_id in cpal::available_hosts() {
        let Ok(host) = cpal::host_from_id(host_id) else { continue };
        let default_name = host.default_output_device().and_then(|d| device_name(&d));
        let Ok(devices) = host.output_devices() else { continue };
        for device in devices {
            let Some(name) = device_name(&device) else { continue };
            let Ok(configs) = device.supported_output_configs() else { continue };
            let configs: Vec<_> = configs.collect();
            if configs.is_empty() {
                continue;
            }
            let max_channels = configs.iter().map(|c| c.channels()).max().unwrap_or(0);
            let sample_rates = RATES
                .into_iter()
                .filter(|&r| configs.iter().any(|c| c.min_sample_rate() <= r && r <= c.max_sample_rate()))
                .collect();
            out.push(DeviceInfo {
                host: host_id.name().to_string(),
                is_default: default_name.as_deref() == Some(name.as_str()),
                name,
                max_channels,
                sample_rates,
            });
        }
    }
    out
}

/// Kanalzahlen für die Ausgabe (ENG-02) in der Reihenfolge, in der sie versucht werden: die
/// Vorgabe des Geräts, auf 2 bis 4 begrenzt, sofern das Gerät diese Zahl anbietet; dann die
/// Vorgabe selbst; zuletzt Stereo. WASAPI im geteilten Modus nimmt neben Stereo nur die
/// Kanalzahl des Geräts an; eine feste Begrenzung auf 4 scheitert dort an jedem 7.1-Gerät.
pub fn channel_candidates(default_channels: u16, offered: &[u16]) -> Vec<u16> {
    let mut out = Vec::with_capacity(3);
    let wanted = default_channels.clamp(2, 4);
    if offered.contains(&wanted) {
        out.push(wanted);
    }
    for channels in [default_channels, 2] {
        if !out.contains(&channels) {
            out.push(channels);
        }
    }
    out
}

/// Öffnet die Ausgabe probeweise mit stummem Signal und schließt sie wieder. Die Angaben des
/// Geräts über seine Formate reichen nicht: Unter WASAPI meldet ein 7.1-Gerät 8 Kanäle als
/// Vorgabe und kann genau diese beim Öffnen ablehnen.
fn opens(device: &cpal::Device, channels: u16, sample_rate: u32) -> bool {
    let config = cpal::StreamConfig { channels, sample_rate, buffer_size: cpal::BufferSize::Default };
    device.build_output_stream(config, |data: &mut [f32], _: &cpal::OutputCallbackInfo| data.fill(0.0), |_err| {}, None).is_ok()
}

/// Sucht ein Gerät nach seinem Namen; `None` = Standardgerät des Systems.
fn find_device(name: Option<&str>) -> Result<cpal::Device, DeviceError> {
    match name {
        None => cpal::default_host().default_output_device().ok_or(DeviceError::NoDevice),
        Some(wanted) => cpal::available_hosts()
            .into_iter()
            .filter_map(|id| cpal::host_from_id(id).ok())
            .filter_map(|h| h.output_devices().ok())
            .flatten()
            .find(|d| device_name(d).as_deref() == Some(wanted))
            .ok_or_else(|| DeviceError::NotFound(wanted.to_string())),
    }
}

/// Sinnvolle Voreinstellung für ein Gerät (`None` = Standardgerät des Systems). Abtastrate:
/// `fixed_rate`, falls vorgegeben (die Engine läuft bereits und behält ihre Rate); sonst
/// 48 kHz, wenn das Gerät sie kann, sonst seine eigene. Kanalzahl: die erste aus
/// `channel_candidates`, mit der sich das Gerät tatsächlich öffnen lässt.
pub fn config_for(device_name: Option<&str>, fixed_rate: Option<u32>) -> Result<OutputConfig, DeviceError> {
    let device = find_device(device_name)?;
    let default = device.default_output_config().map_err(|e| DeviceError::Backend(e.to_string()))?;
    let configs: Vec<_> = device
        .supported_output_configs()
        .map(|configs| configs.filter(|c| c.sample_format() == cpal::SampleFormat::F32).collect())
        .unwrap_or_default();
    let covers = |c: &cpal::SupportedStreamConfigRange, rate: u32| c.min_sample_rate() <= rate && rate <= c.max_sample_rate();
    let sample_rate = match fixed_rate {
        Some(rate) => rate,
        None if configs.iter().any(|c| covers(c, 48_000)) => 48_000,
        None => default.sample_rate(),
    };
    let offered: Vec<u16> = configs.iter().filter(|c| covers(c, sample_rate)).map(|c| c.channels()).collect();
    let candidates = channel_candidates(default.channels(), &offered);
    // Probiert wird nur, wenn es eine Wahl gibt; ein Stereo-Gerät wird wie bisher direkt geöffnet.
    // Lässt sich keine Kanalzahl öffnen, meldet `start` den Fehler für die erste.
    let channels = if candidates.len() == 1 {
        candidates[0]
    } else {
        candidates.iter().copied().find(|&c| opens(&device, c, sample_rate)).unwrap_or(candidates[0])
    };
    Ok(OutputConfig { device: device_name.map(str::to_string), sample_rate, buffer_frames: None, channels })
}

/// Voreinstellung für das Standardgerät des Systems.
pub fn default_config() -> Result<OutputConfig, DeviceError> {
    config_for(None, None)
}

/// Hält den `Processor` im Audio-Thread und gibt ihn zurück, sobald die Ausgabe endet. So
/// überlebt die Engine mit ihren geladenen Decks einen Gerätewechsel.
struct Lease {
    processor: Option<Processor>,
    back: mpsc::SyncSender<Processor>,
}

impl Drop for Lease {
    fn drop(&mut self) {
        if let Some(processor) = self.processor.take() {
            // Der Rückweg fasst genau einen Eintrag; `try_send` fordert keinen Speicher an.
            let _ = self.back.try_send(processor);
        }
    }
}

/// So lange wartet `stop` darauf, dass die Schnittstelle den `Processor` loslässt.
const RELEASE_TIMEOUT: Duration = Duration::from_secs(2);

/// Laufende Ausgabe. Beim Verwerfen endet sie; `stop` gibt zusätzlich den `Processor` zurück.
pub struct OutputStream {
    stream: Option<cpal::Stream>,
    back: mpsc::Receiver<Processor>,
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_frames: Option<u32>,
}

impl OutputStream {
    /// Beendet die Ausgabe und gibt den `Processor` zurück. `None`, falls die Schnittstelle
    /// ihn nicht innerhalb von zwei Sekunden loslässt.
    pub fn stop(mut self) -> Option<Processor> {
        drop(self.stream.take());
        self.back.recv_timeout(RELEASE_TIMEOUT).ok()
    }
}

/// Fehler beim Start. Der `Processor` kommt zurück, damit ein anderes Gerät versucht werden kann.
pub struct StartError {
    pub error: DeviceError,
    pub processor: Option<Processor>,
}

/// Startet die Ausgabe. Der `Processor` wandert in den Audio-Thread.
pub fn start(config: &OutputConfig, errors: ErrorCounter, processor: Processor) -> Result<OutputStream, StartError> {
    let device = match find_device(config.device.as_deref()) {
        Ok(device) => device,
        Err(error) => return Err(StartError { error, processor: Some(processor) }),
    };
    let stream_config = cpal::StreamConfig {
        channels: config.channels,
        sample_rate: config.sample_rate,
        buffer_size: match config.buffer_frames {
            Some(n) => cpal::BufferSize::Fixed(n),
            None => cpal::BufferSize::Default,
        },
    };
    let channels = config.channels as usize;
    let sample_rate = config.sample_rate as f64;
    let (back_tx, back) = mpsc::sync_channel(1);
    let mut lease = Lease { processor: Some(processor), back: back_tx };
    let built = device.build_output_stream(
        stream_config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| match lease.processor.as_mut() {
            Some(processor) => processor.process(data, channels, sample_rate),
            None => data.fill(0.0),
        },
        move |_err| errors.count(),
        None,
    );
    let stream = match built {
        Ok(stream) => stream,
        Err(e) => {
            let error = match e.kind() {
                cpal::ErrorKind::UnsupportedConfig => {
                    DeviceError::Unsupported(format!("{} Hz, {} Kanäle", config.sample_rate, config.channels))
                }
                _ => DeviceError::Backend(e.to_string()),
            };
            return Err(StartError { error, processor: back.recv_timeout(RELEASE_TIMEOUT).ok() });
        }
    };
    if let Err(e) = stream.play() {
        drop(stream);
        return Err(StartError { error: DeviceError::Backend(e.to_string()), processor: back.recv_timeout(RELEASE_TIMEOUT).ok() });
    }
    Ok(OutputStream { stream: Some(stream), back, sample_rate: config.sample_rate, channels: config.channels, buffer_frames: config.buffer_frames })
}

#[cfg(test)]
mod tests {
    use super::{channel_candidates, Lease};
    use std::sync::mpsc;

    /// Endet die Ausgabe, kommt der `Processor` über den Rückweg zurück und nimmt danach
    /// weiter Befehle an; ein Gerätewechsel verliert die Engine also nicht.
    #[test]
    fn lease_returns_the_processor_when_the_output_ends() {
        let (mut ctl, processor) = crate::create(48_000);
        let (tx, rx) = mpsc::sync_channel(1);
        drop(Lease { processor: Some(processor), back: tx });
        let mut processor = rx.try_recv().expect("Processor kommt zurück");
        assert!(ctl.send(dw_core::Command::MasterVolume { db: -6.0 }));
        let mut out = vec![0.0f32; 2 * 4800];
        for _ in 0..10 {
            processor.process(&mut out, 2, 48_000.0);
        }
        assert_eq!(ctl.snapshot().mixer.master_db, -6.0);
    }

    /// ENG-02: Ein 7.1-Gerät unter WASAPI meldet 8 Kanäle und bietet sonst nichts an. Die
    /// frühere feste Begrenzung auf 4 ließ die Ausgabe scheitern („48000 Hz, 4 Kanäle“);
    /// versucht werden die 8 Kanäle des Geräts und danach Stereo.
    #[test]
    fn surround_device_tries_its_own_channel_count_then_stereo() {
        assert_eq!(channel_candidates(8, &[8]), [8, 2]);
        assert_eq!(channel_candidates(8, &[]), [8, 2]);
        assert_eq!(channel_candidates(6, &[6]), [6, 2]);
    }

    #[test]
    fn channel_count_is_limited_to_four_where_the_device_offers_it() {
        assert_eq!(channel_candidates(8, &[2, 4, 8]), [4, 8, 2]);
        assert_eq!(channel_candidates(4, &[2, 4]), [4, 2]);
    }

    #[test]
    fn stereo_and_mono_devices() {
        assert_eq!(channel_candidates(2, &[2]), [2]);
        assert_eq!(channel_candidates(1, &[1, 2]), [2, 1]);
        assert_eq!(channel_candidates(1, &[1]), [1, 2]);
    }
}