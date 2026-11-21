//! Audio-Ausgabe der Anwendung. Das Ausgabe-Objekt des Systems darf seinen Thread nicht
//! verlassen, deshalb besitzt ein eigener Thread es für die ganze Laufzeit. Derselbe Thread
//! wechselt auf Wunsch das Gerät (ENG-01); die Engine mit ihren geladenen Decks bleibt dabei
//! erhalten und behält ihre Abtastrate.

use dw_audio::device::{self, OutputStream};
use dw_audio::{create, Controller, ErrorCounter, Processor, Routing};
use dw_core::{Command, DeckId, XfAssign};
use serde::Serialize;
use std::sync::mpsc;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct AudioInfo {
    /// `false`: Es gibt kein nutzbares Ausgabegerät; die Engine läuft stumm im Zeittakt weiter.
    pub has_device: bool,
    /// Gewähltes Gerät; `None` = Standardgerät des Systems.
    pub device: Option<String>,
    pub sample_rate: u32,
    pub channels: u16,
    /// Puffergröße in Frames; 0 = Vorgabe des Geräts.
    pub buffer_size: u32,
    /// Meldung, falls das Gerät nicht geöffnet werden konnte.
    pub error: Option<String>,
}

const FALLBACK_RATE: u32 = 48_000;
const FALLBACK_BLOCK: usize = 480;

struct Request {
    device: Option<String>,
    reply: mpsc::Sender<Result<AudioInfo, (String, AudioInfo)>>,
}

/// Zugang zum Audio-Thread für den Gerätewechsel.
pub struct AudioHandle(Mutex<mpsc::Sender<Request>>);

impl AudioHandle {
    /// Wechselt das Ausgabegerät (`None` = Standardgerät des Systems). Scheitert der Wechsel,
    /// läuft die Ausgabe auf dem bisherigen Gerät weiter; der Fehler trägt die Meldung und den
    /// Stand danach.
    pub fn select(&self, device: Option<String>) -> Result<AudioInfo, (String, AudioInfo)> {
        let (reply, answer) = mpsc::channel();
        self.0.lock().unwrap().send(Request { device, reply }).expect("Audio-Thread läuft nicht mehr");
        answer.recv().expect("Audio-Thread hat nicht geantwortet")
    }
}

/// Kanalzuweisung: Ab vier Kanälen liegt der Kopfhörer auf dem zweiten Paar (ENG-02).
pub fn routing_for(channels: u16) -> Routing {
    Routing { master: (0, 1), cue: (channels >= 4).then_some((2, 3)), split_cue: false }
}

/// Ausgabe und Engine: Entweder hält die laufende Ausgabe den `Processor`, oder er liegt hier
/// und wird im Zeittakt stumm weitergerechnet.
struct Output {
    stream: Option<OutputStream>,
    idle: Option<Processor>,
    info: AudioInfo,
}

impl Output {
    fn open(device: Option<String>, rate: u32, errors: &ErrorCounter, processor: Processor) -> Output {
        let silent = |error: String, idle: Option<Processor>, device: Option<String>| Output {
            stream: None,
            idle,
            info: AudioInfo { has_device: false, device, sample_rate: rate, channels: 2, buffer_size: FALLBACK_BLOCK as u32, error: Some(error) },
        };
        let config = match device::config_for(device.as_deref(), Some(rate)) {
            Ok(config) => config,
            Err(e) => return silent(e.to_string(), Some(processor), device),
        };
        match device::start(&config, errors.clone(), processor) {
            Ok(stream) => {
                let info = AudioInfo {
                    has_device: true,
                    device,
                    sample_rate: stream.sample_rate,
                    channels: stream.channels,
                    buffer_size: stream.buffer_frames.unwrap_or(0),
                    error: None,
                };
                Output { stream: Some(stream), idle: None, info }
            }
            Err(e) => silent(e.error.to_string(), e.processor, device),
        }
    }

    /// Beendet die Ausgabe und gibt den `Processor` heraus.
    fn close(&mut self) -> Option<Processor> {
        match self.stream.take() {
            Some(stream) => stream.stop(),
            None => self.idle.take(),
        }
    }
}

/// Startet die Ausgabe auf dem gewünschten Gerät (`None` = Standardgerät des Systems) und gibt
/// die Steuerung zurück. Lässt sich ein gewünschtes Gerät nicht öffnen, wird das Standardgerät
/// versucht.
pub fn start(device: Option<String>) -> (Controller, AudioInfo, AudioHandle) {
    let (tx, rx) = mpsc::channel();
    let (req_tx, req_rx) = mpsc::channel::<Request>();
    std::thread::Builder::new()
        .name("deckwerk-audio".into())
        .spawn(move || {
            // Die Engine bekommt die Abtastrate des ersten Geräts und behält sie.
            let rate = device::config_for(device.as_deref(), None)
                .or_else(|_| device::default_config())
                .map_or(FALLBACK_RATE, |c| c.sample_rate);
            let (ctl, processor) = create(rate);
            let errors = ctl.error_counter();
            let mut out = Output::open(device.clone(), rate, &errors, processor);
            if !out.info.has_device && device.is_some() {
                if let Some(processor) = out.close() {
                    out = Output::open(None, rate, &errors, processor);
                }
            }
            let _ = tx.send((ctl, out.info.clone()));
            run(out, rate, errors, req_rx);
        })
        .expect("Audio-Thread konnte nicht gestartet werden");
    let (mut ctl, info) = rx.recv().expect("Audio-Thread hat sich nicht gemeldet");
    ctl.set_routing(routing_for(info.channels));
    assign_crossfader(&mut ctl);
    (ctl, info, AudioHandle(Mutex::new(req_tx)))
}

/// Schleife des Audio-Threads: hält die Ausgabe am Leben, rechnet ohne Gerät im Zeittakt
/// weiter und bedient Gerätewechsel.
fn run(mut out: Output, rate: u32, errors: ErrorCounter, requests: mpsc::Receiver<Request>) {
    let mut silence = vec![0.0f32; FALLBACK_BLOCK * 2];
    let block = Duration::from_secs_f64(FALLBACK_BLOCK as f64 / rate as f64);
    let mut next = Instant::now();
    loop {
        let request = match out.idle.as_mut() {
            // Mit Gerät (oder ganz ohne Engine) gibt es hier nichts zu rechnen.
            None => match requests.recv() {
                Ok(request) => Some(request),
                Err(_) => return,
            },
            // Ohne Gerät: Engine im Zeittakt weiterrechnen, damit die Anwendung bedienbar bleibt.
            Some(processor) => {
                processor.process(&mut silence, 2, rate as f64);
                next += block;
                let now = Instant::now();
                if now > next + Duration::from_millis(200) {
                    // Nach einer langen Pause (Rechner im Ruhezustand) nicht aufholen.
                    next = now;
                }
                match requests.recv_timeout(next.saturating_duration_since(now)) {
                    Ok(request) => Some(request),
                    Err(mpsc::RecvTimeoutError::Timeout) => None,
                    Err(mpsc::RecvTimeoutError::Disconnected) => return,
                }
            }
        };
        let Some(request) = request else { continue };
        let previous = out.info.device.clone();
        let Some(processor) = out.close() else {
            out.info = AudioInfo { has_device: false, error: Some("Die Audio-Schnittstelle hat die Engine nicht freigegeben; bitte Deckwerk neu starten.".into()), ..out.info };
            let _ = request.reply.send(Err((out.info.error.clone().unwrap_or_default(), out.info.clone())));
            continue;
        };
        out = Output::open(request.device.clone(), rate, &errors, processor);
        let answer = if out.info.has_device {
            Ok(out.info.clone())
        } else {
            // Zurück zum bisherigen Gerät, damit ein Fehlgriff den Ton nicht kostet.
            let error = out.info.error.clone().unwrap_or_default();
            if request.device != previous {
                if let Some(processor) = out.close() {
                    out = Output::open(previous, rate, &errors, processor);
                }
            }
            Err((error, out.info.clone()))
        };
        next = Instant::now();
        let _ = request.reply.send(answer);
    }
}

/// Grundzuweisung am Crossfader (MIX-06): A und C links, B und D rechts. Die Engine selbst
/// startet mit „durch", damit der Offline-Render ohne Crossfader auskommt; in der Anwendung
/// wäre der Regler so ohne Wirkung.
fn assign_crossfader(ctl: &mut Controller) {
    for deck in DeckId::ALL {
        let assign = if deck.index() % 2 == 0 { XfAssign::Left } else { XfAssign::Right };
        ctl.send(Command::XfAssign { deck, assign });
    }
}
