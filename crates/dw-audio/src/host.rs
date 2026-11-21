//! Geräteunabhängiger Teil: `Controller` (Steuerungs-Thread) und `Processor` (Audio-Thread).

use dw_core::{Command, DeckId, Snapshot};
use dw_engine::{Engine, Track};
use rtrb::{Consumer, Producer, RingBuffer};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::Instant;

const MESSAGE_CAPACITY: usize = 4096;
const GARBAGE_CAPACITY: usize = 64;
const SNAPSHOT_CAPACITY: usize = 8;
/// Größter Geräte-Puffer in Frames, für den Arbeitsspeicher bereitsteht (ENG-03: bis 2 048).
const MAX_FRAMES: usize = 8192;

enum Message {
    Now(Command),
    At(f64, Command),
    Load(DeckId, Arc<Track>),
    Routing(Routing),
}

/// Zuordnung der Engine-Ausgänge zu den Kanälen des Geräts (ENG-02, MIX-10).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Routing {
    /// Kanalpaar für den Master (nullbasiert).
    pub master: (usize, usize),
    /// Kanalpaar für den Kopfhörer; `None` = kein eigener Kopfhörer-Ausgang.
    pub cue: Option<(usize, usize)>,
    /// Split-Cue: Master mono auf dem linken, Kopfhörer mono auf dem rechten Master-Kanal.
    pub split_cue: bool,
}

impl Default for Routing {
    fn default() -> Self {
        Routing { master: (0, 1), cue: None, split_cue: false }
    }
}

#[derive(Default)]
struct Shared {
    /// DSP-Last des letzten Puffers in Promille der Pufferzeit.
    load: AtomicU32,
    /// Höchste DSP-Last seit dem letzten Abruf.
    load_peak: AtomicU32,
    /// Puffer, deren Berechnung länger dauerte als ihre Spielzeit.
    overruns: AtomicU32,
    /// Vom Gerät gemeldete Aussetzer und Fehler.
    device_errors: AtomicU32,
}

/// Seite des Steuerungs-Threads.
pub struct Controller {
    tx: Producer<Message>,
    garbage: Consumer<Arc<Track>>,
    snapshots: Consumer<Snapshot>,
    last: Snapshot,
    shared: Arc<Shared>,
    sample_rate: u32,
}

/// Seite des Audio-Threads. `process` fordert keinen Speicher an und sperrt nichts.
pub struct Processor {
    engine: Engine,
    rx: Consumer<Message>,
    garbage: Producer<Arc<Track>>,
    /// Alte Tracks, die noch nicht zurückgegeben werden konnten.
    stash: [Option<Arc<Track>>; 4],
    snapshots: Producer<Snapshot>,
    frames_since_snapshot: usize,
    snapshot_interval: usize,
    routing: Routing,
    master: Vec<f32>,
    cue: Vec<f32>,
    shared: Arc<Shared>,
}

/// Erzeugt das Paar aus Steuerung und Audio-Seite für eine Abtastrate.
pub fn create(sample_rate: u32) -> (Controller, Processor) {
    let (tx, rx) = RingBuffer::new(MESSAGE_CAPACITY);
    let (garbage_tx, garbage_rx) = RingBuffer::new(GARBAGE_CAPACITY);
    let (snap_tx, snap_rx) = RingBuffer::new(SNAPSHOT_CAPACITY);
    let shared = Arc::new(Shared::default());
    let controller = Controller {
        tx,
        garbage: garbage_rx,
        snapshots: snap_rx,
        last: Snapshot::default(),
        shared: shared.clone(),
        sample_rate,
    };
    let processor = Processor {
        engine: Engine::new(sample_rate),
        rx,
        garbage: garbage_tx,
        stash: [None, None, None, None],
        snapshots: snap_tx,
        frames_since_snapshot: 0,
        snapshot_interval: (sample_rate / 60) as usize,
        routing: Routing::default(),
        master: vec![0.0; MAX_FRAMES * 2],
        cue: vec![0.0; MAX_FRAMES * 2],
        shared,
    };
    (controller, processor)
}

impl Controller {
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Sendet einen Befehl. `false`, wenn der Puffer voll ist (Audio-Thread steht).
    pub fn send(&mut self, cmd: Command) -> bool {
        self.tx.push(Message::Now(cmd)).is_ok()
    }

    /// Merkt einen Befehl für einen Zeitpunkt in Clock-Beats vor.
    pub fn send_at(&mut self, beat: f64, cmd: Command) -> bool {
        self.tx.push(Message::At(beat, cmd)).is_ok()
    }

    /// Lädt einen Track auf ein Deck. Er muss in der Abtastrate der Engine vorliegen.
    pub fn load(&mut self, deck: DeckId, track: Arc<Track>) -> bool {
        assert_eq!(track.audio.sample_rate, self.sample_rate);
        self.collect_garbage();
        self.tx.push(Message::Load(deck, track)).is_ok()
    }

    pub fn set_routing(&mut self, routing: Routing) -> bool {
        self.tx.push(Message::Routing(routing)).is_ok()
    }

    /// Gibt Tracks frei, die der Audio-Thread nicht mehr braucht. Regelmäßig aufrufen.
    pub fn collect_garbage(&mut self) -> usize {
        let mut n = 0;
        while self.garbage.pop().is_ok() {
            n += 1;
        }
        n
    }

    /// Neuester Zustand der Engine. Spitzenpegel werden über alle seit dem letzten Abruf
    /// eingetroffenen Schnappschüsse zusammengefasst.
    pub fn snapshot(&mut self) -> &Snapshot {
        let mut peaks = [[0.0f32; 2]; 4];
        let mut master_peak = [0.0f32; 2];
        let mut gr = 0.0f32;
        let mut fresh = false;
        while let Ok(s) = self.snapshots.pop() {
            for (i, d) in s.decks.iter().enumerate() {
                peaks[i] = [peaks[i][0].max(d.peak[0]), peaks[i][1].max(d.peak[1])];
            }
            master_peak = [master_peak[0].max(s.mixer.master_peak[0]), master_peak[1].max(s.mixer.master_peak[1])];
            gr = gr.min(s.mixer.limiter_gr_db);
            self.last = s;
            fresh = true;
        }
        if fresh {
            for (i, d) in self.last.decks.iter_mut().enumerate() {
                d.peak = peaks[i];
            }
            self.last.mixer.master_peak = master_peak;
            self.last.mixer.limiter_gr_db = gr;
        }
        self.collect_garbage();
        &self.last
    }

    /// DSP-Last des letzten Puffers, 0 … 1 (ENG-09).
    pub fn dsp_load(&self) -> f32 {
        self.shared.load.load(Ordering::Relaxed) as f32 / 1000.0
    }

    /// Höchste DSP-Last seit dem letzten Aufruf.
    pub fn take_dsp_load_peak(&self) -> f32 {
        self.shared.load_peak.swap(0, Ordering::Relaxed) as f32 / 1000.0
    }

    /// Zahl der Aussetzer: überlange Berechnungen plus vom Gerät gemeldete Fehler.
    pub fn xruns(&self) -> u32 {
        self.shared.overruns.load(Ordering::Relaxed) + self.shared.device_errors.load(Ordering::Relaxed)
    }

    /// Zähler für Fehler, die das Gerät meldet; er geht in `xruns` ein. Wer die Ausgabe
    /// öffnet, reicht ihn an `device::start` weiter, auch bei einem späteren Gerätewechsel.
    pub fn error_counter(&self) -> ErrorCounter {
        ErrorCounter(self.shared.clone())
    }
}

/// Zählt vom Gerät gemeldete Fehler (siehe `Controller::error_counter`).
#[derive(Clone)]
pub struct ErrorCounter(Arc<Shared>);

impl ErrorCounter {
    pub fn count(&self) {
        self.0.device_errors.fetch_add(1, Ordering::Relaxed);
    }
}

impl Processor {
    fn return_track(&mut self, old: Option<Arc<Track>>) {
        // Freigeben geschieht im Steuerungs-Thread. Ist der Rückweg voll, wird der Track zwischengelagert.
        let Some(mut track) = old else { return };
        for slot in self.stash.iter_mut() {
            if let Some(t) = slot.take() {
                if let Err(rtrb::PushError::Full(t)) = self.garbage.push(t) {
                    *slot = Some(t);
                }
            }
        }
        track = match self.garbage.push(track) {
            Ok(()) => return,
            Err(rtrb::PushError::Full(t)) => t,
        };
        if let Some(slot) = self.stash.iter_mut().find(|s| s.is_none()) {
            *slot = Some(track);
        } else {
            // Äußerster Notfall: lieber Speicher nie freigeben als im Audio-Thread freigeben.
            std::mem::forget(track);
        }
    }

    fn drain_messages(&mut self) {
        while let Ok(msg) = self.rx.pop() {
            match msg {
                Message::Now(cmd) => self.engine.apply(cmd),
                Message::At(beat, cmd) => {
                    self.engine.schedule(beat, cmd);
                }
                Message::Load(deck, track) => {
                    let old = self.engine.load(deck, track);
                    self.return_track(old);
                }
                Message::Routing(r) => self.routing = r,
            }
        }
    }

    /// Füllt einen Geräte-Puffer mit `channels` verschachtelten Kanälen.
    pub fn process(&mut self, out: &mut [f32], channels: usize, sample_rate: f64) {
        let started = Instant::now();
        self.drain_messages();
        let total = out.len() / channels.max(1);
        out.fill(0.0);
        let mut done = 0;
        while done < total {
            let n = (total - done).min(MAX_FRAMES);
            self.engine.process(&mut self.master[..n * 2], &mut self.cue[..n * 2]);
            let r = self.routing;
            for i in 0..n {
                let frame = &mut out[(done + i) * channels..(done + i + 1) * channels];
                let (ml, mr) = (self.master[i * 2], self.master[i * 2 + 1]);
                let (cl, cr) = (self.cue[i * 2], self.cue[i * 2 + 1]);
                if r.split_cue {
                    if let Some(s) = frame.get_mut(r.master.0) {
                        *s = 0.5 * (ml + mr);
                    }
                    if let Some(s) = frame.get_mut(r.master.1) {
                        *s = 0.5 * (cl + cr);
                    }
                } else {
                    if let Some(s) = frame.get_mut(r.master.0) {
                        *s = ml;
                    }
                    if let Some(s) = frame.get_mut(r.master.1) {
                        *s = mr;
                    }
                    if let Some((a, b)) = r.cue {
                        if let Some(s) = frame.get_mut(a) {
                            *s = cl;
                        }
                        if let Some(s) = frame.get_mut(b) {
                            *s = cr;
                        }
                    }
                }
            }
            done += n;
        }

        self.frames_since_snapshot += total;
        if self.frames_since_snapshot >= self.snapshot_interval && !self.snapshots.is_full() {
            self.frames_since_snapshot = 0;
            let _ = self.snapshots.push(self.engine.snapshot());
        }

        let budget = total as f64 / sample_rate;
        let load = if budget > 0.0 { (started.elapsed().as_secs_f64() / budget * 1000.0) as u32 } else { 0 };
        self.shared.load.store(load, Ordering::Relaxed);
        self.shared.load_peak.fetch_max(load, Ordering::Relaxed);
        if load > 1000 {
            self.shared.overruns.fetch_add(1, Ordering::Relaxed);
        }
    }
}
