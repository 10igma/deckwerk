//! Mitschnitt des Master-Signals (REC-01): Ein eigener Thread liest den Abgriff des Audio-Threads
//! und schreibt laufend eine WAV-Datei. Einmal je Sekunde stehen die Längen im Dateikopf, damit
//! die Datei auch nach einem Absturz lesbar ist.

use dw_audio::MasterTap;
use dw_decode::WavWriter;
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Bittiefe der Aufnahme.
const BITS: u16 = 24;
/// So oft wird der Abgriff geleert.
const POLL: Duration = Duration::from_millis(20);
/// So oft werden die Längen in den Dateikopf geschrieben.
const SYNC_EVERY: Duration = Duration::from_secs(1);

/// Stand eines Mitschnitts.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct RecordingInfo {
    pub path: String,
    /// Läuft die Aufnahme noch?
    pub active: bool,
    /// Bisher geschriebene Länge in Sekunden.
    pub seconds: f64,
    /// Frames, die verloren gingen, weil das Schreiben nicht nachkam (0 = lückenlos).
    pub dropped_frames: u32,
    /// Grund, falls die Aufnahme von selbst abgebrochen ist (z. B. Datenträger voll).
    pub error: Option<String>,
}

enum Request {
    Start(PathBuf, Sender<Result<RecordingInfo, String>>),
    Stop(Sender<Result<RecordingInfo, String>>),
}

/// Steuerung des Mitschnitt-Threads.
pub struct Recorder {
    tx: Mutex<Sender<Request>>,
    status: Arc<Mutex<Option<RecordingInfo>>>,
}

struct Active {
    writer: WavWriter,
    path: PathBuf,
    synced: Instant,
}

impl Recorder {
    /// Startet den Thread, der den Abgriff liest. Er endet, wenn der `Recorder` verschwindet.
    pub fn spawn(mut tap: MasterTap) -> Recorder {
        let (tx, rx) = mpsc::channel::<Request>();
        let status: Arc<Mutex<Option<RecordingInfo>>> = Arc::new(Mutex::new(None));
        let shared = status.clone();
        std::thread::Builder::new()
            .name("deckwerk-mitschnitt".into())
            .spawn(move || {
                let rate = tap.sample_rate() as f64;
                let mut active: Option<Active> = None;
                let mut buf: Vec<f32> = Vec::with_capacity(tap.sample_rate() as usize * 2);
                let info = |a: &Active, tap: &MasterTap, on: bool, error: Option<String>| RecordingInfo {
                    path: a.path.to_string_lossy().to_string(),
                    active: on,
                    seconds: a.writer.frames() as f64 / rate,
                    dropped_frames: tap.dropped_frames(),
                    error,
                };
                loop {
                    let request = rx.recv_timeout(POLL);
                    // Erst leeren, dann die Anforderung: So landet beim Stoppen alles in der Datei.
                    let mut failed: Option<String> = None;
                    if let Some(a) = active.as_mut() {
                        buf.clear();
                        tap.read(&mut buf);
                        let mut result = if buf.is_empty() { Ok(()) } else { a.writer.write(&buf) };
                        if result.is_ok() && a.synced.elapsed() >= SYNC_EVERY {
                            result = a.writer.sync();
                            a.synced = Instant::now();
                            *shared.lock().unwrap() = Some(info(a, &tap, true, None));
                        }
                        if let Err(e) = result {
                            failed = Some(format!("{}: {e}", a.path.display()));
                        }
                    }
                    if let Some(reason) = failed {
                        // Schreiben gescheitert: Aufnahme beenden, den Grund stehen lassen.
                        tap.set_active(false);
                        if let Some(mut a) = active.take() {
                            let _ = a.writer.sync();
                            *shared.lock().unwrap() = Some(info(&a, &tap, false, Some(reason)));
                        }
                    }
                    match request {
                        Ok(Request::Start(path, reply)) => {
                            let result = if active.is_some() {
                                Err("Es läuft schon eine Aufnahme.".to_string())
                            } else {
                                match WavWriter::create(&path, tap.sample_rate(), BITS) {
                                    Ok(writer) => {
                                        tap.set_active(true);
                                        let a = Active { writer, path, synced: Instant::now() };
                                        let started = info(&a, &tap, true, None);
                                        *shared.lock().unwrap() = Some(started.clone());
                                        active = Some(a);
                                        Ok(started)
                                    }
                                    Err(e) => Err(format!("{}: {e}", path.display())),
                                }
                            };
                            let _ = reply.send(result);
                        }
                        Ok(Request::Stop(reply)) => {
                            let result = match active.take() {
                                Some(mut a) => {
                                    tap.set_active(false);
                                    // Der Audio-Thread schreibt den laufenden Puffer noch zu Ende.
                                    std::thread::sleep(Duration::from_millis(40));
                                    buf.clear();
                                    tap.read(&mut buf);
                                    let written = a.writer.write(&buf).and_then(|_| a.writer.sync());
                                    let done = info(&a, &tap, false, written.err().map(|e| e.to_string()));
                                    *shared.lock().unwrap() = None;
                                    Ok(done)
                                }
                                None => Err("Es läuft keine Aufnahme.".to_string()),
                            };
                            let _ = reply.send(result);
                        }
                        Err(RecvTimeoutError::Timeout) => {}
                        Err(RecvTimeoutError::Disconnected) => {
                            if let Some(mut a) = active.take() {
                                tap.set_active(false);
                                let _ = a.writer.sync();
                            }
                            break;
                        }
                    }
                }
            })
            .expect("Mitschnitt-Thread");
        Recorder { tx: Mutex::new(tx), status }
    }

    fn ask(&self, make: impl FnOnce(Sender<Result<RecordingInfo, String>>) -> Request) -> Result<RecordingInfo, String> {
        let (reply, answer) = mpsc::channel();
        self.tx.lock().unwrap().send(make(reply)).map_err(|_| "Der Mitschnitt-Thread läuft nicht mehr.".to_string())?;
        answer.recv_timeout(Duration::from_secs(5)).map_err(|_| "Der Mitschnitt-Thread antwortet nicht.".to_string())?
    }

    /// Beginnt eine Aufnahme in `path` (WAV, 24 bit, Abtastrate der Engine).
    pub fn start(&self, path: &Path) -> Result<RecordingInfo, String> {
        self.ask(|reply| Request::Start(path.to_path_buf(), reply))
    }

    /// Beendet die Aufnahme und schließt die Datei ab.
    pub fn stop(&self) -> Result<RecordingInfo, String> {
        self.ask(Request::Stop)
    }

    /// Stand der laufenden Aufnahme (höchstens eine Sekunde alt) oder der zuletzt von selbst
    /// abgebrochenen; `None`, wenn keine läuft.
    pub fn status(&self) -> Option<RecordingInfo> {
        self.status.lock().unwrap().clone()
    }
}
