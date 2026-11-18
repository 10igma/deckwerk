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
