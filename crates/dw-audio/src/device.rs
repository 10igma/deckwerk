//! Ausgabe über die Audio-Schnittstellen des Systems (ENG-01 bis ENG-03): WASAPI und ASIO
//! unter Windows, CoreAudio unter macOS, ALSA und JACK unter Linux (PipeWire über dessen
//! ALSA-/JACK-Schicht).

use crate::host::{Controller, Processor};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

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

/// Listet die Ausgabegeräte aller verfügbaren Schnittstellen.
pub fn list_devices() -> Vec<DeviceInfo> {
    let mut out = Vec::new();
    for host_id in cpal::available_hosts() {
        let Ok(host) = cpal::host_from_id(host_id) else { continue };
        let default_name = host.default_output_device().and_then(|d| d.name().ok());
        let Ok(devices) = host.output_devices() else { continue };
        for device in devices {
            let Ok(name) = device.name() else { continue };
            let Ok(configs) = device.supported_output_configs() else { continue };
            let configs: Vec<_> = configs.collect();
            if configs.is_empty() {
                continue;
            }
            let max_channels = configs.iter().map(|c| c.channels()).max().unwrap_or(0);
            let sample_rates = RATES
                .into_iter()
                .filter(|&r| configs.iter().any(|c| c.min_sample_rate().0 <= r && r <= c.max_sample_rate().0))
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

/// Sinnvolle Voreinstellung für das Standardgerät: 48 kHz, wenn das Gerät sie kann, sonst seine
/// eigene Abtastrate; die Kanalzahl des Geräts (mindestens 2, höchstens 4).
pub fn default_config() -> Result<OutputConfig, DeviceError> {
    let device = cpal::default_host().default_output_device().ok_or(DeviceError::NoDevice)?;
    let default = device.default_output_config().map_err(|e| DeviceError::Backend(e.to_string()))?;
    let supports_48k = device.supported_output_configs().is_ok_and(|mut configs| {
        configs.any(|c| c.sample_format() == cpal::SampleFormat::F32 && c.min_sample_rate().0 <= 48_000 && 48_000 <= c.max_sample_rate().0)
    });
    Ok(OutputConfig {
        device: None,
        sample_rate: if supports_48k { 48_000 } else { default.sample_rate().0 },
        buffer_frames: None,
        channels: default.channels().clamp(2, 4),
    })
}

/// Laufende Ausgabe. Beim Verwerfen endet sie.
pub struct OutputStream {
    _stream: cpal::Stream,
    pub sample_rate: u32,
    pub channels: u16,
    pub buffer_frames: Option<u32>,
}

/// Startet die Ausgabe. Der `Processor` wandert in den Audio-Thread.
pub fn start(config: &OutputConfig, controller: &Controller, mut processor: Processor) -> Result<OutputStream, DeviceError> {
    let device = match &config.device {
        None => cpal::default_host().default_output_device().ok_or(DeviceError::NoDevice)?,
        Some(wanted) => cpal::available_hosts()
            .into_iter()
            .filter_map(|id| cpal::host_from_id(id).ok())
            .filter_map(|h| h.output_devices().ok())
            .flatten()
            .find(|d| d.name().is_ok_and(|n| &n == wanted))
            .ok_or_else(|| DeviceError::NotFound(wanted.clone()))?,
    };
    let stream_config = cpal::StreamConfig {
        channels: config.channels,
        sample_rate: cpal::SampleRate(config.sample_rate),
        buffer_size: match config.buffer_frames {
            Some(n) => cpal::BufferSize::Fixed(n),
            None => cpal::BufferSize::Default,
        },
    };
    let channels = config.channels as usize;
    let sample_rate = config.sample_rate as f64;
    let on_error = controller.error_counter();
    let stream = device
        .build_output_stream(
            &stream_config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| processor.process(data, channels, sample_rate),
            move |_err| on_error(),
            None,
        )
        .map_err(|e| match e {
            cpal::BuildStreamError::StreamConfigNotSupported => {
                DeviceError::Unsupported(format!("{} Hz, {} Kanäle", config.sample_rate, config.channels))
            }
            other => DeviceError::Backend(other.to_string()),
        })?;
    stream.play().map_err(|e| DeviceError::Backend(e.to_string()))?;
    Ok(OutputStream { _stream: stream, sample_rate: config.sample_rate, channels: config.channels, buffer_frames: config.buffer_frames })
}
