//! Tests der Echtzeit-Anbindung ohne Audiogerät: Kanal-Zuordnung, Split-Cue,
//! Rückgabe alter Tracks, Schnappschüsse und kein Speicher im Audio-Pfad.

use dw_audio::{create, Routing};
use dw_core::{AudioBuf, BeatGrid, Command, DeckId};
use dw_engine::Track;
use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::Arc;

thread_local! {
    static TRACKING: Cell<bool> = const { Cell::new(false) };
    static COUNT: Cell<usize> = const { Cell::new(0) };
}

struct Counting;

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        if TRACKING.with(|t| t.get()) {
            COUNT.with(|c| c.set(c.get() + 1));
        }
        System.alloc(layout)
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        if TRACKING.with(|t| t.get()) {
            COUNT.with(|c| c.set(c.get() + 1));
        }
        System.dealloc(ptr, layout)
    }
}

#[global_allocator]
static ALLOC: Counting = Counting;

const SR: u32 = 48_000;

/// Konstantes Signal: links `l`, rechts `r`.
fn dc_track(l: f32, r: f32, seconds: f64) -> Arc<Track> {
    let frames = (seconds * SR as f64) as usize;
    let data: Vec<f32> = (0..frames).flat_map(|_| [l, r]).collect();
    Arc::new(Track { audio: AudioBuf::new(SR, data), grid: BeatGrid::constant(120.0, 0.0, 0), gain_db: 0.0 })
}

/// Mittelwert je Kanal über den hinteren Teil eines Puffers.
fn means(buf: &[f32], channels: usize) -> Vec<f32> {
    let frames = buf.len() / channels;
    (0..channels).map(|c| (frames / 2..frames).map(|i| buf[i * channels + c]).sum::<f32>() / (frames - frames / 2) as f32).collect()
}

#[test]
fn master_and_headphones_go_to_their_channel_pairs() {
    let (mut ctl, mut proc) = create(SR);
    ctl.set_routing(Routing { master: (0, 1), cue: Some((2, 3)), split_cue: false });
    // A läuft auf dem Master; B ist nur vorgehört (Fader zu).
    ctl.load(DeckId::A, dc_track(0.2, 0.1, 5.0));
    ctl.load(DeckId::B, dc_track(-0.3, -0.4, 5.0));
    ctl.send(Command::Fader { deck: DeckId::B, value: 0.0 });
    ctl.send(Command::Cue { deck: DeckId::B, on: true });
    ctl.send(Command::Play { deck: DeckId::A, on: true });
    ctl.send(Command::Play { deck: DeckId::B, on: true });
    let mut out = vec![9.0f32; 4800 * 6];
    proc.process(&mut out, 6, SR as f64);
    proc.process(&mut out, 6, SR as f64);
    let m = means(&out, 6);
    // Der EQ lässt Gleichanteil unverändert durch (Tiefband), deshalb sind die Werte direkt vergleichbar.
    for (got, want) in m.iter().zip([0.2, 0.1, -0.3, -0.4, 0.0, 0.0]) {
        assert!((got - want).abs() < 2e-3, "Kanäle {m:?}");
    }
}

#[test]
fn split_cue_puts_master_left_and_cue_right() {
    let (mut ctl, mut proc) = create(SR);
    ctl.set_routing(Routing { master: (0, 1), cue: None, split_cue: true });
    ctl.load(DeckId::A, dc_track(0.2, 0.4, 5.0)); // Master, mono = 0,3
    ctl.load(DeckId::B, dc_track(-0.1, -0.3, 5.0)); // Cue, mono = −0,2
    ctl.send(Command::Fader { deck: DeckId::B, value: 0.0 });
    ctl.send(Command::Cue { deck: DeckId::B, on: true });
    ctl.send(Command::Play { deck: DeckId::A, on: true });
    ctl.send(Command::Play { deck: DeckId::B, on: true });
    let mut out = vec![0.0f32; 9600 * 2];
    proc.process(&mut out, 2, SR as f64);
    let m = means(&out, 2);
    assert!((m[0] - 0.3).abs() < 2e-3 && (m[1] + 0.2).abs() < 2e-3, "Kanäle {m:?}");
}

#[test]
fn replaced_tracks_come_back_to_the_control_thread() {
    let (mut ctl, mut proc) = create(SR);
    let first = dc_track(0.1, 0.1, 1.0);
    ctl.load(DeckId::A, first.clone());
    let mut out = vec![0.0f32; 512 * 2];
    proc.process(&mut out, 2, SR as f64);
    assert_eq!(Arc::strong_count(&first), 2);
    ctl.load(DeckId::A, dc_track(0.2, 0.2, 1.0));
    proc.process(&mut out, 2, SR as f64);
    // Der alte Track liegt jetzt im Rückweg und wird im Steuerungs-Thread freigegeben.
    assert_eq!(ctl.collect_garbage(), 1);
    assert_eq!(Arc::strong_count(&first), 1);
}

#[test]
fn snapshots_report_state_and_peaks() {
    let (mut ctl, mut proc) = create(SR);
    ctl.load(DeckId::A, dc_track(0.5, 0.25, 5.0));
    ctl.send(Command::Play { deck: DeckId::A, on: true });
    let mut out = vec![0.0f32; 256 * 2];
    for _ in 0..20 {
        proc.process(&mut out, 2, SR as f64);
    }
    // Der Einschwingvorgang des EQ beim Start schwingt über; für den Pegelvergleich zählt der Teil danach.
    let _ = ctl.snapshot();
    for _ in 0..20 {
        proc.process(&mut out, 2, SR as f64);
    }
    let s = ctl.snapshot().clone();
    assert!(s.decks[0].playing && s.decks[0].loaded);
    assert!(s.decks[0].pos > 0.1 && s.decks[0].pos < 0.3, "Position {}", s.decks[0].pos);
    assert!((s.decks[0].peak[0] - 0.5).abs() < 0.01 && (s.decks[0].peak[1] - 0.25).abs() < 0.01);
    assert_eq!(s.master, Some(DeckId::A));
    assert!(ctl.dsp_load() < 1.0);
    assert_eq!(ctl.xruns(), 0);
}

#[test]
fn audio_side_does_not_allocate() {
    let (mut ctl, mut proc) = create(SR);
    ctl.load(DeckId::A, dc_track(0.1, 0.1, 30.0));
    ctl.load(DeckId::B, dc_track(0.2, 0.2, 30.0));
    ctl.send(Command::Play { deck: DeckId::A, on: true });
    for i in 0..200 {
        ctl.send_at(1.0 + i as f64 * 0.1, Command::Filter { deck: DeckId::A, pos: (i % 10) as f32 / 10.0 });
    }
    // Ein Wechsel des Tracks im laufenden Betrieb: Der alte darf nicht im Audio-Thread freigegeben werden.
    ctl.load(DeckId::B, dc_track(0.3, 0.3, 30.0));
    ctl.send(Command::Play { deck: DeckId::B, on: true });
    let mut out = vec![0.0f32; 256 * 4];

    TRACKING.with(|t| t.set(true));
    for _ in 0..2000 {
        proc.process(&mut out, 4, SR as f64);
    }
    TRACKING.with(|t| t.set(false));

    let count = COUNT.with(|c| c.get());
    assert_eq!(count, 0, "{count} Speicheranforderungen oder -freigaben im Audio-Thread");
    assert_eq!(ctl.collect_garbage(), 1);
}

/// REC-01: Der Mitschnitt-Abgriff liefert genau das Master-Signal, das an das Gerät geht – lückenlos
/// über viele Puffer, nur solange er eingeschaltet ist, und ohne Speicher im Audio-Pfad. Liest
/// niemand, gehen Frames verloren und werden gezählt, statt den Audio-Thread aufzuhalten.
#[test]
fn master_tap_delivers_exactly_what_goes_to_the_device() {
    let (mut ctl, mut proc) = create(SR);
    let mut tap = ctl.take_master_tap().expect("Abgriff");
    assert!(ctl.take_master_tap().is_none(), "den Abgriff gibt es nur einmal");
    assert_eq!(tap.sample_rate(), SR);
    // Rampe statt Gleichsignal, damit Lücken und Vertauschungen auffallen.
    let frames = SR as usize * 20;
    let data: Vec<f32> = (0..frames).flat_map(|i| [0.3 * ((i % 4801) as f32 / 4801.0), -0.2 * ((i % 3001) as f32 / 3001.0)]).collect();
    ctl.load(DeckId::A, Arc::new(Track { audio: AudioBuf::new(SR, data), grid: BeatGrid::constant(120.0, 0.0, 0), gain_db: 0.0 }));
    ctl.send(Command::Play { deck: DeckId::A, on: true });
    let mut out = vec![0.0f32; 512 * 2];
    proc.process(&mut out, 2, SR as f64);

    // Ausgeschaltet kommt nichts an.
    let mut got = Vec::new();
    assert_eq!(tap.read(&mut got), 0);

    // Eingeschaltet: 200 Puffer verschiedener Größe, zwischendurch gelesen; kein Speicher im Audio-Pfad.
    tap.set_active(true);
    let mut sent = Vec::new();
    TRACKING.with(|t| t.set(true));
    COUNT.with(|c| c.set(0));
    for i in 0..200 {
        let n = [64, 256, 480, 512, 37][i % 5];
        proc.process(&mut out[..n * 2], 2, SR as f64);
        TRACKING.with(|t| t.set(false));
        sent.extend_from_slice(&out[..n * 2]);
        if i % 7 == 0 {
            tap.read(&mut got);
        }
        TRACKING.with(|t| t.set(true));
    }
    TRACKING.with(|t| t.set(false));
    assert_eq!(COUNT.with(|c| c.get()), 0, "Speicher im Audio-Pfad angefordert oder freigegeben");
    tap.read(&mut got);
    assert_eq!(got.len(), sent.len());
    assert!(got == sent, "Abgriff weicht vom Ausgang ab");
    assert!(sent.iter().any(|v| v.abs() > 0.05), "es lief kein Ton");
    assert_eq!(tap.dropped_frames(), 0);

    // Liest niemand, läuft der Abgriff voll (4 s): Der Rest wird gezählt, der Ausgang läuft weiter.
    for _ in 0..(SR as usize * 5 / 512) {
        proc.process(&mut out, 2, SR as f64);
    }
    assert!(out.iter().any(|v| v.abs() > 0.05), "Ausgang steht");
    let dropped = tap.dropped_frames() as usize;
    got.clear();
    tap.read(&mut got);
    assert_eq!(got.len(), SR as usize * 2 * 4, "Abgriff fasst 4 s");
    assert!(dropped > SR as usize / 2 && dropped < SR as usize * 2, "verloren: {dropped} Frames");

    // Wieder einschalten verwirft Altes und beginnt bei null.
    tap.set_active(false);
    proc.process(&mut out, 2, SR as f64);
    tap.set_active(true);
    assert_eq!(tap.dropped_frames(), 0);
    proc.process(&mut out, 2, SR as f64);
    got.clear();
    tap.read(&mut got);
    assert!(got == out, "nach dem Wiedereinschalten kommt genau der neue Puffer");
}
