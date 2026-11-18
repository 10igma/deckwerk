//! Start der Audio-Ausgabe. Das Ausgabe-Objekt des Systems darf seinen Thread nicht
//! verlassen, deshalb besitzt ein eigener Thread es für die ganze Laufzeit.

use dw_audio::device;
use dw_audio::{create, Controller, Routing};
use serde::Serialize;
use std::sync::mpsc;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, Serialize)]
pub struct AudioInfo {
    /// `false`: Es gibt kein nutzbares Ausgabegerät; die Engine läuft stumm im Zeittakt weiter.
    pub has_device: bool,
    pub sample_rate: u32,
    pub channels: u16,
    /// Puffergröße in Frames; 0 = Vorgabe des Geräts.
    pub buffer_size: u32,
    /// Meldung, falls das Gerät nicht geöffnet werden konnte.
    pub error: Option<String>,
}

const FALLBACK_RATE: u32 = 48_000;
const FALLBACK_BLOCK: usize = 480;

/// Startet die Ausgabe auf dem Standardgerät und gibt die Steuerung zurück.
pub fn start() -> (Controller, AudioInfo) {
    let (tx, rx) = mpsc::channel();
    std::thread::Builder::new()
        .name("deckwerk-audio".into())
        .spawn(move || {
            let error = match device::default_config() {
                Ok(config) => {
                    let (mut ctl, processor) = create(config.sample_rate);
                    // Ab vier Kanälen liegt der Kopfhörer auf dem zweiten Paar (ENG-02).
                    ctl.set_routing(Routing { master: (0, 1), cue: (config.channels >= 4).then_some((2, 3)), split_cue: false });
                    match device::start(&config, &ctl, processor) {
                        Ok(stream) => {
                            let info = AudioInfo {
                                has_device: true,
                                sample_rate: stream.sample_rate,
                                channels: stream.channels,
                                buffer_size: stream.buffer_frames.unwrap_or(0),
                                error: None,
                            };
                            let _ = tx.send((ctl, info));
                            // Der Thread hält die Ausgabe am Leben.
                            loop {
                                std::thread::park();
                            }
                        }
                        Err(e) => e.to_string(),
                    }
                }
                Err(e) => e.to_string(),
            };
            // Ohne Gerät: Engine im Zeittakt weiterrechnen, damit die Anwendung bedienbar bleibt.
            run_without_device(tx, Some(error));
        })
        .expect("Audio-Thread konnte nicht gestartet werden");
    rx.recv().expect("Audio-Thread hat sich nicht gemeldet")
}

fn run_without_device(tx: mpsc::Sender<(Controller, AudioInfo)>, error: Option<String>) {
    let (ctl, mut processor) = create(FALLBACK_RATE);
    let info = AudioInfo { has_device: false, sample_rate: FALLBACK_RATE, channels: 2, buffer_size: FALLBACK_BLOCK as u32, error };
    let _ = tx.send((ctl, info));
    let mut out = vec![0.0f32; FALLBACK_BLOCK * 2];
    let block = Duration::from_secs_f64(FALLBACK_BLOCK as f64 / FALLBACK_RATE as f64);
    let mut next = Instant::now();
    loop {
        processor.process(&mut out, 2, FALLBACK_RATE as f64);
        next += block;
        let now = Instant::now();
        if next > now {
            std::thread::sleep(next - now);
        } else if now - next > Duration::from_millis(200) {
            // Nach einer langen Pause (Rechner im Ruhezustand) nicht aufholen.
            next = now;
        }
    }
}
