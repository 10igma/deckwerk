//! Ein vorbereiteter Übergang im laufenden Betrieb (TRANS-05, AUTO-08): A läuft als Master, B steht
//! irgendwo; der Übergang wird eingeplant und von der Engine gefahren. Gemessen wird am Ton:
//! TRANS-02 (Taktgrenzen), TRANS-03 (Bass-Regel), DECK-08 (Sync) – für jede eingebaute Vorlage.

use dw_analysis::{analyze, AnalysisConfig};
use dw_automix::{plan_live, LiveDeck, LivePlan, Roles, Transition, When};
use dw_core::{AudioBuf, Command, DeckId, Key, TrackAnalysis};
use dw_dsp::{Biquad, Coeffs};
use dw_engine::{Engine, Track};
use dw_testgen::{techno_track, SectionKind, SectionSpec, TrackSpec};
use std::sync::Arc;

use DeckId::{A, B};

const SR: u32 = 48_000;
const ROLES: Roles = Roles { out: A, inn: B };

/// Track, der durchgehend Kick und Bass spielt: der härteste Fall für die Bass-Regel.
fn full_track(bpm: f64, key: Key, bars: u32, first_beat: f64, bar_offset: u8, seed: u64) -> (Arc<Track>, TrackAnalysis) {
    let mut spec = TrackSpec::new(bpm, key, vec![SectionSpec::new(SectionKind::Main, bars)], seed);
    spec.sample_rate = SR;
    spec.first_beat_sec = first_beat;
    spec.bar_offset = bar_offset;
    let (audio, _) = techno_track(&spec);
    let analysis = analyze(&audio, &AnalysisConfig::default());
    let track = Arc::new(Track { audio, grid: analysis.grid.clone(), gain_db: analysis.auto_gain_db(-14.0) });
    (track, analysis)
}

/// Pegel im Band unter 120 Hz je Takt (in dB), für die Takte, die ganz in `from..to` liegen.
fn low_band_per_bar(audio: &[f32], from: usize, to: usize, bar: usize) -> Vec<f64> {
    let c = Coeffs::lowpass(SR as f64, 120.0, std::f64::consts::FRAC_1_SQRT_2);
    let (mut f1, mut f2) = (Biquad::new(c), Biquad::new(c));
    let low: Vec<f64> = audio.chunks_exact(2).map(|f| f2.process(f1.process(0.5 * (f[0] + f[1]) as f64))).collect();
    low[from..to].chunks_exact(bar).map(|c| 10.0 * (c.iter().map(|v| v * v).sum::<f64>() / bar as f64).max(1e-12).log10()).collect()
}

fn max(v: &[f64]) -> f64 {
    v.iter().cloned().fold(f64::MIN, f64::max)
}

/// Rendert, bis die Clock `until` erreicht; hängt den Ton an `out` an und meldet den größten
/// Versatz zwischen den Rastern, solange beide Decks laufen.
fn run_until(e: &mut Engine, until: f64, out: &mut Vec<f32>, marks: &mut [(f64, usize)]) -> f64 {
    let mut cue = vec![0.0f32; 256 * 2];
    let mut worst = 0.0f64;
    while e.clock_beats() < until {
        let before = e.clock_beats();
        let at = out.len();
        out.resize(at + 256 * 2, 0.0);
        e.process(&mut out[at..], &mut cue);
        for (beat, frame) in marks.iter_mut() {
            if before < *beat && e.clock_beats() >= *beat {
                *frame = at / 2;
            }
        }
        let s = e.snapshot();
        if s.decks[0].playing && s.decks[1].playing {
            worst = worst.max(s.decks[1].phase.abs()).max(s.decks[0].phase.abs());
        }
    }
    worst
}

struct Live {
    engine: Engine,
    plan: LivePlan,
    out: Vec<f32>,
    grid_b: dw_core::BeatGrid,
    bar: usize,
    /// Beat von B, an dem es einsteigt.
    entry: f64,
}

/// A läuft als Master mitten im Track, B steht neben einem Taktanfang; der Übergang wird eingeplant.
fn prepare(transition: &Transition, align_bars: u32) -> Live {
    let (a, an_a) = full_track(132.0, Key::from_camelot(8, 'A').unwrap(), 112, 0.21, 0, 11);
    // B ist etwas schneller und beginnt mit einem Auftakt: Die Eins liegt auf Beat 2.
    let (b, an_b) = full_track(134.5, Key::from_camelot(9, 'A').unwrap(), 112, 0.05, 2, 12);
    let mut e = Engine::new(SR);
    e.load(A, a);
    e.load(B, b);
    // B steht irgendwo im neunten Takt, nicht auf einem Taktanfang.
    e.apply(Command::Seek { deck: B, pos: an_b.grid.sec_at(34.3) });
    e.apply(Command::Play { deck: A, on: true });
    let bar = (4.0 * 60.0 / 132.0 * SR as f64).round() as usize;
    let mut out = Vec::new();
    // Gut neun Takte von A allein, dann drückt jemand „Übergang".
    let pressed = e.clock_beats() + 37.3;
    run_until(&mut e, pressed, &mut out, &mut []);

    let s = e.snapshot();
    assert_eq!(s.master, Some(A));
    let deck = |i: usize, an: &TrackAnalysis| (s.decks[i].pos, s.decks[i].duration, s.decks[i].track_bpm, an.grid.clone());
    let (da, db) = (deck(0, &an_a), deck(1, &an_b));
    let plan = plan_live(
        transition,
        ROLES,
        s.clock_beats,
        LiveDeck { grid: &da.3, playing: true, pos: da.0, duration: da.1, track_bpm: da.2 },
        LiveDeck { grid: &db.3, playing: false, pos: db.0, duration: db.1, track_bpm: db.2 },
        When::Next { align_bars },
    )
    .unwrap();
    // TRANS-02: T0 liegt auf einem Taktanfang von A im gewählten Raster, der Einstieg auf einem von B.
    let a_at_t0 = da.3.beat_at(da.0) + (plan.t0 - s.clock_beats);
    assert!((a_at_t0 - a_at_t0.round()).abs() < 1e-6 && da.3.is_downbeat(a_at_t0.round() as i64), "T0 bei Beat {a_at_t0} von A");
    let first_a = da.3.next_downbeat(da.3.beat_at(0.0) - 1e-6);
    assert_eq!((a_at_t0.round() as i64 - first_a).rem_euclid(4 * align_bars as i64), 0);
    // Der Einstieg ist der Taktanfang von B, der seiner Position am nächsten liegt (höchstens zwei Beats).
    let entry = db.3.beat_at(plan.in_pos);
    assert!((entry - entry.round()).abs() < 1e-6 && db.3.is_downbeat(entry.round() as i64), "Einstieg bei Beat {entry}");
    assert!((entry - 34.3).abs() <= 2.0 + 1e-6, "Einstieg bei Beat {entry}, B stand bei 34,3");

    for cmd in &plan.now {
        e.apply(*cmd);
    }
    for (beat, cmd) in &plan.scheduled {
        assert!(e.schedule(*beat, *cmd));
    }
    Live { engine: e, plan, out, grid_b: an_b.grid, bar, entry }
}

#[test]
fn prepared_transitions_play_live_and_follow_the_rules() {
    for transition in Transition::templates(16) {
        let name = transition.name.clone();
        let l = transition.beats();
        let Live { engine: mut e, plan, mut out, grid_b, bar, entry } = prepare(&transition, 4);

        let mut marks = [(plan.t0, 0usize), (plan.t0 + l, 0usize)];
        // Bis kurz nach T0: B läuft, im Takt, ab seinem Einstieg.
        run_until(&mut e, plan.t0 + 0.5, &mut out, &mut marks);
        let s = e.snapshot();
        assert!(s.decks[1].playing, "{name}: B läuft nach T0 nicht");
        assert!((grid_b.beat_at(s.decks[1].pos) - (entry + s.clock_beats - plan.t0)).abs() < 1e-3, "{name}: B steht bei Beat {}", grid_b.beat_at(s.decks[1].pos));
        assert!((s.decks[1].bpm - 132.0).abs() < 1e-3, "{name}: B läuft mit {} BPM", s.decks[1].bpm);

        // Durch den Übergang und acht Takte darüber hinaus.
        let worst = run_until(&mut e, plan.end + 32.0, &mut out, &mut marks);
        assert!(worst < 1e-3, "{name}: Beat-Versatz {worst} (DECK-08)");
        let s = e.snapshot();
        // Danach: A steht mit Reglern in Ruhestellung, B läuft offen und ist Master.
        assert!(!s.decks[0].playing && s.decks[1].playing, "{name}");
        assert_eq!((s.decks[0].eq_db, s.decks[0].fader, s.decks[0].filter), ([0.0; 3], 1.0, 0.0), "{name}: A nicht zurückgestellt");
        assert_eq!((s.decks[1].eq_db, s.decks[1].fader), ([0.0; 3], 1.0), "{name}: B nicht offen");
        assert_eq!(s.master, Some(B), "{name}");

        // TRANS-03: Kein Takt im Übergang ist im Bass mehr als 3 dB lauter als A oder B allein.
        let (t0, end) = (marks[0].1, marks[1].1);
        assert!((end as i64 - t0 as i64 - 16 * bar as i64).abs() < 512, "{name}: Länge {} Frames", end - t0);
        let solo_a = max(&low_band_per_bar(&out, t0 - 8 * bar, t0, bar));
        let solo_b = max(&low_band_per_bar(&out, end + bar, end + 7 * bar, bar));
        let during = low_band_per_bar(&out, t0, end, bar);
        let reference = solo_a.max(solo_b);
        let lowest = during.iter().cloned().fold(f64::MAX, f64::min);
        println!("{name}: Bass allein A {solo_a:.1} dB, B {solo_b:.1} dB; im Übergang {:.1} … {:.1} dB", lowest, max(&during));
        assert!(max(&during) <= reference + 3.0, "{name}: Bass im Übergang {:.1} dB über dem Einzelpegel", max(&during) - reference);
        assert!(lowest >= reference - 6.0, "{name}: Bass-Loch im Übergang: {during:.1?}");
        // Master bleibt unter der Decke des Limiters.
        assert!(AudioBuf::new(SR, out).peak() <= 10f32.powf(-0.3 / 20.0) + 1e-6, "{name}");
    }
}

/// Abbrechen verwirft den Rest: Die Regler bleiben stehen, wo sie sind, und beide Decks laufen weiter.
#[test]
fn cancelling_leaves_the_controls_where_they_are() {
    let transition = Transition::bass_swap(16);
    let Live { engine: mut e, plan, mut out, .. } = prepare(&transition, 1);
    // Ein Viertel des Übergangs: B ist offen, sein Bass noch aus, die Mitten unterwegs.
    run_until(&mut e, plan.t0 + 16.0, &mut out, &mut []);
    e.apply(Command::CancelScheduled);
    let at_cancel = e.snapshot();
    assert_eq!(at_cancel.decks[1].fader, 1.0);
    assert_eq!(at_cancel.decks[1].eq_db[0], f32::NEG_INFINITY);
    assert!(at_cancel.decks[1].eq_db[1] < -1.0 && at_cancel.decks[1].eq_db[1] > -6.0, "Mitten von B bei {}", at_cancel.decks[1].eq_db[1]);

    run_until(&mut e, plan.end + 8.0, &mut out, &mut []);
    let later = e.snapshot();
    assert!(later.decks[0].playing && later.decks[1].playing);
    for i in 0..2 {
        assert_eq!((later.decks[i].eq_db, later.decks[i].fader), (at_cancel.decks[i].eq_db, at_cancel.decks[i].fader), "Deck {i} hat sich nach dem Abbruch bewegt");
    }
}

/// Ende vorgegeben (TRANS-05): Der Übergang endet auf dem gewählten Taktanfang von A, und das
/// stehende Deck B hat dann genau die Stelle erreicht, an der es stand. Auch aus dem Stand am
/// eigenen Anfang – dann startet B vor seinem Anfang und läuft bis dorthin stumm.
#[test]
fn an_end_anchored_transition_lands_on_the_mark() {
    for (b_beat, label) in [(130.3, "mitten im Track"), (2.2, "am Anfang")] {
        let (a, an_a) = full_track(132.0, Key::from_camelot(8, 'A').unwrap(), 112, 0.21, 0, 11);
        let (b, an_b) = full_track(134.5, Key::from_camelot(9, 'A').unwrap(), 112, 0.05, 2, 12);
        let mut e = Engine::new(SR);
        e.load(A, a);
        e.load(B, b);
        e.apply(Command::Seek { deck: B, pos: an_b.grid.sec_at(b_beat) });
        e.apply(Command::Play { deck: A, on: true });
        let mut out = Vec::new();
        let pressed = e.clock_beats() + 21.7;
        run_until(&mut e, pressed, &mut out, &mut []);

        let s = e.snapshot();
        let transition = Transition::bass_swap(16);
        let plan = plan_live(
            &transition,
            ROLES,
            s.clock_beats,
            LiveDeck { grid: &an_a.grid, playing: true, pos: s.decks[0].pos, duration: s.decks[0].duration, track_bpm: s.decks[0].track_bpm },
            LiveDeck { grid: &an_b.grid, playing: false, pos: s.decks[1].pos, duration: s.decks[1].duration, track_bpm: s.decks[1].track_bpm },
            When::EndIn { beats: 100.0 },
        )
        .unwrap();
        // Das Ende liegt auf dem Taktanfang von A, der 100 Beats voraus am nächsten ist.
        let end_clock = plan.t0 + transition.beats();
        let a_at_end = an_a.grid.beat_at(s.decks[0].pos) + (end_clock - s.clock_beats);
        assert!((a_at_end - a_at_end.round()).abs() < 1e-6 && an_a.grid.is_downbeat(a_at_end.round() as i64), "{label}: Ende bei Beat {a_at_end} von A");
        assert!((end_clock - s.clock_beats - 100.0).abs() <= 2.0 + 1e-6, "{label}: Ende in {} Beats", end_clock - s.clock_beats);
        // Marke von B: der Taktanfang neben seiner Stelle.
        let mark = an_b.grid.beat_at(plan.in_pos) + transition.beats();
        assert!((mark - mark.round()).abs() < 1e-6 && an_b.grid.is_downbeat(mark.round() as i64) && (mark - b_beat).abs() <= 2.0 + 1e-6, "{label}: Marke bei Beat {mark}");

        for cmd in &plan.now {
            e.apply(*cmd);
        }
        for (beat, cmd) in &plan.scheduled {
            assert!(e.schedule(*beat, *cmd));
        }
        // Vor T0 steht B, am Start liegt es um die Länge vor der Marke.
        run_until(&mut e, plan.t0 - 0.5, &mut out, &mut []);
        let s = e.snapshot();
        assert!(!s.decks[1].playing, "{label}: B läuft vor T0");
        assert!((an_b.grid.beat_at(s.decks[1].pos) - (mark - transition.beats())).abs() < 1e-6, "{label}");
        if b_beat < 10.0 {
            assert!(s.decks[1].pos < 0.0, "{label}: B steht bei {} s, erwartet vor dem Anfang", s.decks[1].pos);
        }
        // Am Ende hat B die Marke erreicht, auf 1/1000 Beat, und läuft im Takt.
        let worst = run_until(&mut e, end_clock, &mut out, &mut []);
        let s = e.snapshot();
        let reached = an_b.grid.beat_at(s.decks[1].pos) - (s.clock_beats - end_clock);
        println!("{label}: B am Ende bei Beat {reached:.4}, Marke {mark}; größter Versatz {worst:.5}");
        assert!((reached - mark).abs() < 1e-3, "{label}: B am Ende bei Beat {reached}, Marke {mark}");
        assert!(worst < 1e-3 && s.decks[1].phase.abs() < 1e-3, "{label}: Versatz {worst}");
    }
}

/// Klick-Spur: ein kurzer Impuls auf jedem Beat, nur auf einem Kanal (0 = links, 1 = rechts).
fn click_track(bpm: f64, first_beat: f64, beats: usize, channel: usize) -> (Arc<Track>, dw_core::BeatGrid) {
    let frames = ((first_beat + (beats as f64 + 2.0) * 60.0 / bpm) * SR as f64) as usize;
    let mut data = vec![0.0f32; frames * 2];
    for b in 0..beats {
        let centre = ((first_beat + b as f64 * 60.0 / bpm) * SR as f64).round() as usize;
        for k in 0..9usize {
            let i = centre + k;
            if i >= 4 && i - 4 < frames {
                data[(i - 4) * 2 + channel] = 0.8 * (0.5 + 0.5 * (std::f32::consts::PI * (k as f32 - 4.0) / 5.0).cos());
            }
        }
    }
    let grid = dw_core::BeatGrid::constant(bpm, first_beat, 0);
    (Arc::new(Track { audio: AudioBuf::new(SR, data), grid: grid.clone(), gain_db: 0.0 }), grid)
}

/// Stellen der Klicks (Maximum je Anschlag) auf einem Kanal.
fn clicks(out: &[f32], channel: usize, from: usize, to: usize) -> Vec<usize> {
    let mut found = Vec::new();
    let mut i = from;
    while i < to {
        if out[i * 2 + channel].abs() > 0.05 {
            let end = (i + 400).min(to);
            let peak = (i..end).max_by(|&a, &b| out[a * 2 + channel].abs().total_cmp(&out[b * 2 + channel].abs())).unwrap();
            found.push(peak);
            i = peak + 2000;
        } else {
            i += 1;
        }
    }
    found
}

/// DECK-08 am Ton, nicht nur an den Rastern: Im eingeplanten Übergang liegen die Anschläge beider
/// Decks übereinander (≤ 1 ms) – vor dem Bass-Tausch, danach und für beide Arten, ihn zu legen.
#[test]
fn clicks_of_both_decks_coincide_during_a_live_transition() {
    for when in [When::Next { align_bars: 4 }, When::EndIn { beats: 120.0 }] {
        let (a, grid_a) = click_track(132.0, 0.21, 700, 0);
        let (b, grid_b) = click_track(134.5, 0.05, 700, 1);
        let mut e = Engine::new(SR);
        e.load(A, a);
        e.load(B, b);
        e.apply(Command::Seek { deck: B, pos: grid_b.sec_at(131.3) });
        e.apply(Command::Play { deck: A, on: true });
        let mut out = Vec::new();
        let pressed = e.clock_beats() + 37.3;
        run_until(&mut e, pressed, &mut out, &mut []);
        let s = e.snapshot();
        let transition = Transition::bass_swap(16);
        let plan = plan_live(
            &transition,
            ROLES,
            s.clock_beats,
            LiveDeck { grid: &grid_a, playing: true, pos: s.decks[0].pos, duration: s.decks[0].duration, track_bpm: s.decks[0].track_bpm },
            LiveDeck { grid: &grid_b, playing: false, pos: s.decks[1].pos, duration: s.decks[1].duration, track_bpm: s.decks[1].track_bpm },
            when,
        )
        .unwrap();
        for cmd in &plan.now {
            e.apply(*cmd);
        }
        for (beat, cmd) in &plan.scheduled {
            assert!(e.schedule(*beat, *cmd));
        }
        let mut marks = [(plan.t0, 0usize), (plan.t0 + transition.beats(), 0usize)];
        run_until(&mut e, plan.t0 + transition.beats() + 4.0, &mut out, &mut marks);
        // Im Übergang, sobald B offen ist und solange A noch offen ist: jeder Klick von B auf einem von A.
        let beat = (60.0 / 132.0 * SR as f64) as usize;
        let (from, to) = (marks[0].1 + 10 * beat, marks[1].1 - 18 * beat);
        let (left, right) = (clicks(&out, 0, from, to), clicks(&out, 1, from, to));
        assert!(left.len() >= 30 && right.len() >= 30, "{when:?}: {} / {} Klicks", left.len(), right.len());
        let worst = right.iter().map(|&r| left.iter().map(|&l| (l as i64 - r as i64).abs()).min().unwrap()).max().unwrap();
        let worst_ms = worst as f64 / SR as f64 * 1000.0;
        println!("{when:?}: größter Abstand zwischen den Klicks {worst_ms:.3} ms");
        assert!(worst_ms <= 1.0, "{when:?}: Klicks von B liegen {worst_ms:.1} ms neben denen von A");
    }
}
