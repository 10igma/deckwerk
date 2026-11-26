//! T1 – Langer Blend mit Bass-Tausch, als Folge zeitgenauer Engine-Befehle.

use crate::transition::{Roles, Transition};
use dw_core::{AudioBuf, Command, DeckId, SyncMode, TrackAnalysis};
use dw_engine::{Engine, Track};
use std::sync::Arc;

/// Takte je Phrase, auf die Ein- und Ausstieg gerastet werden.
const PHRASE_BARS: i64 = 8;
/// Größte Tempoänderung je Takt beim Angleichen (T10).
const MAX_TEMPO_CHANGE_PER_BAR: f64 = 0.0025;

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum PlanError {
    #[error("Tempo-Abstand {0:.1} % ist für einen Blend zu groß (höchstens {1:.0} %)")]
    TempoGap(f64, f64),
    #[error("Track {0} ist zu kurz für einen Übergang über {1} Takte")]
    TooShort(char, u32),
    #[error("Übergangslänge muss 16, 32 oder 64 Takte sein")]
    BadLength,
}

/// Plan eines Übergangs von A nach B. Positionen in Sekunden im jeweiligen Track.
#[derive(Clone, Debug, PartialEq)]
pub struct BlendPlan {
    /// Hier beginnt die Wiedergabe von A (ein Downbeat).
    pub a_start: f64,
    /// Übergangsbeginn T0 in A (Phrasengrenze).
    pub a_out: f64,
    /// Einstieg von B (Phrasengrenze).
    pub b_in: f64,
    /// Länge L des Übergangs in Takten.
    pub bars: u32,
    /// Takte von A allein vor dem Übergang.
    pub lead_bars: u32,
    /// Takte von B allein nach dem Übergang.
    pub tail_bars: u32,
    pub bpm_a: f64,
    pub bpm_b: f64,
    /// Tempo-Verlauf: Beginn und Ende in Takten relativ zu T0. Gleich = Sprung am Tauschpunkt.
    pub tempo_ramp: (f64, f64),
}

impl BlendPlan {
    /// Tauschpunkt S in Takten ab T0.
    pub fn swap_bar(&self) -> u32 {
        self.bars / 2
    }

    /// Tempo-Abstand in Prozent.
    pub fn tempo_gap_percent(&self) -> f64 {
        (self.bpm_b / self.bpm_a - 1.0).abs() * 100.0
    }
}

/// Erster Downbeat bei oder nach dem Trackanfang.
fn first_downbeat(a: &TrackAnalysis) -> i64 {
    a.grid.next_downbeat(a.grid.beat_at(0.0) - 1e-6)
}

/// Plant T1. `lead_bars` und `tail_bars` werden auf ganze Phrasen gerundet.
pub fn plan_blend(a: &TrackAnalysis, b: &TrackAnalysis, bars: u32, lead_bars: u32, tail_bars: u32) -> Result<BlendPlan, PlanError> {
    if ![16, 32, 64].contains(&bars) {
        return Err(PlanError::BadLength);
    }
    let gap = (b.bpm / a.bpm - 1.0).abs();
    let max_gap = if bars >= 32 { 0.08 } else { 0.06 };
    if gap > max_gap {
        return Err(PlanError::TempoGap(gap * 100.0, max_gap * 100.0));
    }
    let l = bars as i64;

    // A: letzte Phrasengrenze, nach der noch der ganze Übergang (plus zwei Takte Reserve) Platz hat.
    let a_first = first_downbeat(a);
    let a_total_bars = ((a.grid.beat_at(a.duration) - a_first as f64) / 4.0).floor() as i64;
    let out_bar = ((a_total_bars - l - 2).div_euclid(PHRASE_BARS)) * PHRASE_BARS;
    if out_bar < 0 {
        return Err(PlanError::TooShort('A', bars));
    }
    let lead = (lead_bars as i64 / PHRASE_BARS * PHRASE_BARS).min(out_bar);
    let a_out_beat = a_first + out_bar * 4;
    let a_start_beat = a_out_beat - lead * 4;

    // B: Einstieg am ersten Downbeat; der Track muss über Übergang und Nachlauf reichen.
    let b_first = first_downbeat(b);
    let b_total_bars = ((b.grid.beat_at(b.duration) - b_first as f64) / 4.0).floor() as i64;
    if b_total_bars < l + 2 {
        return Err(PlanError::TooShort('B', bars));
    }
    let tail = (tail_bars as i64 / PHRASE_BARS * PHRASE_BARS).min(b_total_bars - l - 1).max(0);

    // T10: Tempo-Verlauf.
    let s = bars as f64 / 2.0;
    let tempo_ramp = if gap <= 0.005 {
        (s, s)
    } else {
        let needed = gap / MAX_TEMPO_CHANGE_PER_BAR;
        let extra = ((needed - bars as f64).max(0.0) / 2.0).min(16.0);
        ((-extra).max(-(lead as f64)), (bars as f64 + extra).min((bars as i64 + tail) as f64))
    };

    Ok(BlendPlan {
        a_start: a.grid.sec_at(a_start_beat as f64),
        a_out: a.grid.sec_at(a_out_beat as f64),
        b_in: b.grid.sec_at(b_first as f64),
        bars,
        lead_bars: lead as u32,
        tail_bars: tail as u32,
        bpm_a: a.bpm,
        bpm_b: b.bpm,
        tempo_ramp,
    })
}

/// Messwerte eines gerenderten Übergangs.
#[derive(Clone, Debug, PartialEq)]
pub struct BlendReport {
    /// Frame im Ergebnis, an dem der Übergang beginnt (T0), der Tauschpunkt (S) liegt und er endet.
    pub t0_frame: usize,
    pub swap_frame: usize,
    pub end_frame: usize,
    /// Größter Versatz zwischen den Beat-Rastern beider Decks während des Übergangs, in Beats.
    pub max_phase_error: f64,
    /// Stärkste Pegelreduktion des Limiters in dB (≤ 0).
    pub max_limiter_reduction_db: f32,
}

/// Rendert den Übergang: A allein, Übergang nach der Vorlage T1, B allein.
pub fn render_blend(a: Arc<Track>, b: Arc<Track>, plan: &BlendPlan) -> (AudioBuf, BlendReport) {
    let sr = a.audio.sample_rate;
    assert_eq!(sr, b.audio.sample_rate);
    let mut e = Engine::new(sr);
    use DeckId::{A, B};

    // Freie Clock als Taktgeber, beide Decks hängen mit Beat-Sync daran.
    e.apply(Command::SetMaster { deck: None });
    e.apply(Command::ClockTempo { bpm: plan.bpm_a });
    e.load(A, a);
    e.load(B, b);
    e.apply(Command::Seek { deck: A, pos: plan.a_start });
    e.apply(Command::Seek { deck: B, pos: plan.b_in });
    e.apply(Command::Sync { deck: A, mode: SyncMode::Beat });
    e.apply(Command::Sync { deck: B, mode: SyncMode::Beat });

    let l = plan.bars as f64 * 4.0;
    let s = plan.swap_bar() as f64 * 4.0;
    // Die Clock beginnt bei 0; ganze Beats, damit die Decks ohne Phasensprung einrasten.
    let start = 4.0;
    let t0 = start + plan.lead_bars as f64 * 4.0;
    let end = t0 + l + plan.tail_bars as f64 * 4.0;

    e.schedule(start, Command::Play { deck: A, on: true });

    // --- Vorlage T1 (5.10) als Automation: dieselbe Beschreibung, die auch live abgespielt wird ---
    for (beat, cmd) in Transition::bass_swap(plan.bars).commands(t0, Roles { out: A, inn: B }) {
        e.schedule(beat, cmd);
    }

    // --- T10: Tempo-Verlauf ---
    let (r0, r1) = (t0 + plan.tempo_ramp.0 * 4.0, t0 + plan.tempo_ramp.1 * 4.0);
    if r1 > r0 {
        let beats = (r1 - r0).round() as u32;
        for i in 0..=beats {
            let t = i as f64 / beats as f64;
            e.schedule(r0 + i as f64, Command::ClockTempo { bpm: plan.bpm_a + (plan.bpm_b - plan.bpm_a) * t });
        }
    } else {
        e.schedule(r0, Command::ClockTempo { bpm: plan.bpm_b });
    }

    // Rendern, bis die Clock das Ende erreicht.
    let mut out: Vec<f32> = Vec::new();
    let mut cue = vec![0.0f32; 512 * 2];
    let mut report = BlendReport { t0_frame: 0, swap_frame: 0, end_frame: 0, max_phase_error: 0.0, max_limiter_reduction_db: 0.0 };
    let mut skip = 0usize;
    let mut started = false;
    while e.clock_beats() < end {
        let before = e.clock_beats();
        let at = out.len();
        out.resize(at + 512 * 2, 0.0);
        e.process(&mut out[at..], &mut cue);
        let now = e.clock_beats();
        let frame = at / 2;
        if !started && now >= start {
            // Alles vor dem Start von A ist Stille und wird abgeschnitten.
            started = true;
            skip = frame;
        }
        for (mark, slot) in [(t0, &mut report.t0_frame), (t0 + s, &mut report.swap_frame), (t0 + l, &mut report.end_frame)] {
            if before < mark && now >= mark {
                *slot = frame;
            }
        }
        let snap = e.snapshot();
        report.max_limiter_reduction_db = report.max_limiter_reduction_db.min(snap.mixer.limiter_gr_db);
        if now > t0 + 1.0 && now < t0 + l - 1.0 && snap.decks[0].playing && snap.decks[1].playing {
            let d = snap.decks[0].beat - snap.decks[1].beat;
            report.max_phase_error = report.max_phase_error.max((d - d.round()).abs());
        }
    }
    out.drain(..skip * 2);
    report.t0_frame = report.t0_frame.saturating_sub(skip);
    report.swap_frame = report.swap_frame.saturating_sub(skip);
    report.end_frame = report.end_frame.saturating_sub(skip);
    (AudioBuf::new(sr, out), report)
}
