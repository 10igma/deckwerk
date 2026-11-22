//! Einen vorbereiteten Übergang im laufenden Betrieb einplanen (TRANS-05, AUTO-08): Aus dem Stand
//! der Decks wird der Beginn T0 an der nächsten passenden Taktgrenze bestimmt und die Automation
//! in vorgemerkte Befehle für die Engine übersetzt.

use crate::transition::{Roles, Transition};
use dw_core::{BeatGrid, Command, SyncMode};
use serde::{Deserialize, Serialize};

/// Größter Tempo-Abstand zwischen den Tracks, bis zu dem ein Übergang im Takt eingeplant wird.
pub const MAX_TEMPO_GAP: f64 = 0.08;
/// Vorlauf in Beats zwischen „jetzt" und dem ersten Befehl des Übergangs: Zeit für den Weg der
/// Befehle zur Engine und dafür, dass der Start nicht überraschend kommt.
pub const LEAD_BEATS: f64 = 2.0;

/// Wann der Übergang liegen soll.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum When {
    /// Beginn an der nächsten Taktgrenze des laufenden Tracks im Raster von `align_bars` Takten.
    /// Ein stehendes neues Deck steigt an seiner jetzigen Stelle ein.
    Next { align_bars: u32 },
    /// Ende in `beats` Clock-Beats ab jetzt (auf den nächstgelegenen Taktanfang des laufenden Tracks
    /// gerückt); der Beginn wird um die Länge zurückgerechnet. Ein stehendes neues Deck erreicht seine
    /// jetzige Stelle genau am Ende – es startet entsprechend früher, notfalls vor seinem Anfang
    /// und bis dorthin stumm.
    EndIn { beats: f64 },
}

/// Stand eines Decks, wie die Planung ihn braucht.
#[derive(Clone, Copy, Debug)]
pub struct LiveDeck<'a> {
    pub grid: &'a BeatGrid,
    /// Läuft das Deck schon? Beim kommenden Deck: Dann entfallen Einstieg und Start.
    pub playing: bool,
    /// Abspielposition in Sekunden.
    pub pos: f64,
    /// Länge des Tracks in Sekunden.
    pub duration: f64,
    /// Tempo des Tracks an dieser Stelle (ohne Regler).
    pub track_bpm: f64,
}

#[derive(Debug, thiserror::Error, PartialEq)]
pub enum LiveError {
    #[error("Tempo-Abstand {0:.1} % ist für einen Übergang im Takt zu groß (höchstens {1:.0} %)")]
    TempoGap(f64, f64),
    #[error("Der laufende Track ist zu kurz: Bis zum Ende bleiben {0:.0} Takte, der Übergang braucht {1}")]
    OutTooShort(f64, u32),
    #[error("Der neue Track ist ab seinem Einstieg zu kurz für einen Übergang über {0} Takte")]
    InTooShort(u32),
    #[error("Bis zum gewählten Ende sind es {0:.0} Beats; der Übergang über {1} Takte braucht {2:.0}. Das Ende weiter nach hinten legen oder eine kürzere Länge wählen")]
    TooLate(f64, u32, f64),
    #[error("{0}")]
    Invalid(String),
}

/// Ein eingeplanter Übergang.
#[derive(Clone, Debug, PartialEq)]
pub struct LivePlan {
    /// Beginn T0 in Clock-Beats.
    pub t0: f64,
    /// Clock-Beat, nach dem der Übergang nichts mehr tut.
    pub end: f64,
    /// Stelle, an der das kommende Deck bei T0 steht, in Sekunden (negativ = vor seinem Anfang).
    /// Läuft es schon, seine Position bei T0.
    pub in_pos: f64,
    /// Sofort auszuführen (Einstieg setzen, Sync einschalten).
    pub now: Vec<Command>,
    /// Vorzumerken: (Clock-Beat, Befehl), aufsteigend.
    pub scheduled: Vec<(f64, Command)>,
}

/// Erster Taktanfang bei oder nach dem Trackanfang.
fn first_downbeat(grid: &BeatGrid) -> i64 {
    grid.next_downbeat(grid.beat_at(0.0) - 1e-6)
}

/// Taktanfang, der dem Beat `beat` am nächsten liegt.
fn nearest_downbeat(grid: &BeatGrid, beat: f64) -> i64 {
    let after = grid.next_downbeat(beat - 1e-6);
    let before = after - 4;
    if (beat - before as f64) < (after as f64 - beat) {
        before
    } else {
        after
    }
}

/// Plant `transition` vom laufenden Deck `out` zum Deck `inn`, das steht oder schon mitläuft.
///
/// Voraussetzung, die der Aufrufer prüft: Das laufende Deck gibt den Takt der Clock vor (es ist
/// Master) oder hängt im Beat-Sync an ihr – dann laufen seine Beats und die der Clock im Gleichschritt
/// und `clock_beats` lässt sich über seine Position in die Zukunft rechnen. Dasselbe gilt für ein
/// schon laufendes neues Deck.
///
/// Der erste Befehl des Übergangs liegt immer mindestens [`LEAD_BEATS`] nach „jetzt".
pub fn plan_live(transition: &Transition, roles: Roles, clock_beats: f64, out: LiveDeck, inn: LiveDeck, when: When) -> Result<LivePlan, LiveError> {
    transition.validate().map_err(LiveError::Invalid)?;
    if roles.out == roles.inn {
        return Err(LiveError::Invalid("Übergang braucht zwei verschiedene Decks".into()));
    }
    let gap = (inn.track_bpm / out.track_bpm - 1.0).abs();
    if !gap.is_finite() || gap > MAX_TEMPO_GAP {
        return Err(LiveError::TempoGap(gap * 100.0, MAX_TEMPO_GAP * 100.0));
    }
    let l = transition.beats();

    // T0 im Raster des laufenden Tracks.
    let out_beat = out.grid.beat_at(out.pos);
    let earliest = out_beat + LEAD_BEATS - transition.first_beat();
    let boundary = match when {
        When::Next { align_bars } => {
            let align = align_bars.max(1) as i64;
            let first = first_downbeat(out.grid);
            let bar = (out.grid.next_downbeat(earliest) - first).div_euclid(4);
            first + (bar + (align - bar.rem_euclid(align)) % align) * 4
        }
        When::EndIn { beats } => {
            if !beats.is_finite() {
                return Err(LiveError::Invalid("Ende des Übergangs ist keine Zahl".into()));
            }
            let end = nearest_downbeat(out.grid, out_beat + beats);
            let start = end - l as i64;
            if (start as f64) < earliest {
                return Err(LiveError::TooLate((end as f64 - out_beat).max(0.0), transition.bars, l + earliest - out_beat));
            }
            start
        }
    };
    let left_bars = (out.grid.beat_at(out.duration) - boundary as f64) / 4.0;
    if left_bars < transition.bars as f64 {
        return Err(LiveError::OutTooShort(left_bars.max(0.0), transition.bars));
    }
    let t0 = clock_beats + (boundary as f64 - out_beat);

    let in_beat = inn.grid.beat_at(inn.pos);
    let in_end = inn.grid.beat_at(inn.duration);
    if inn.playing {
        // Das neue Deck läuft schon mit: Es wird nur noch gefahren, nicht mehr gestellt oder gestartet.
        let at_t0 = in_beat + (t0 - clock_beats);
        if in_end - at_t0 < l + 4.0 {
            return Err(LiveError::InTooShort(transition.bars));
        }
        let running = Transition { start_in: None, ..transition.clone() };
        return Ok(LivePlan { t0, end: t0 + running.last_beat(), in_pos: inn.grid.sec_at(at_t0), now: Vec::new(), scheduled: running.commands(t0, roles) });
    }

    // Das neue Deck steht. Seine Stelle rückt auf den nächstgelegenen Taktanfang; je nach Vorgabe ist
    // das der Einstieg bei T0 oder die Stelle, die es am Ende erreicht haben soll.
    let mut mark = nearest_downbeat(inn.grid, in_beat);
    let entry = match when {
        When::Next { .. } => {
            if inn.grid.sec_at(mark as f64) < 0.0 {
                mark += 4;
            }
            mark as f64
        }
        // Im Beat-Sync legt das Deck je Clock-Beat einen eigenen Beat zurück.
        When::EndIn { .. } => mark as f64 - l,
    };
    if in_end - entry < l + 4.0 {
        return Err(LiveError::InTooShort(transition.bars));
    }
    let in_pos = inn.grid.sec_at(entry);
    Ok(LivePlan {
        t0,
        end: t0 + transition.last_beat(),
        in_pos,
        now: vec![Command::Seek { deck: roles.inn, pos: in_pos }, Command::Sync { deck: roles.inn, mode: SyncMode::Beat }],
        scheduled: transition.commands(t0, roles),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use dw_core::DeckId::{A, B};

    const ROLES: Roles = Roles { out: A, inn: B };

    fn deck(grid: &BeatGrid, pos: f64, bpm: f64) -> LiveDeck<'_> {
        LiveDeck { grid, playing: false, pos, duration: 600.0, track_bpm: bpm }
    }

    /// TRANS-05: T0 liegt auf der nächsten Grenze im gewählten Taktraster des laufenden Tracks, nie
    /// näher als der Vorlauf; die Clock-Zeit ergibt sich aus dem Abstand in Beats.
    #[test]
    fn start_is_quantised_to_the_bar_grid_of_the_playing_track() {
        // A: 120 BPM, erster Beat bei 0,25 s, die Eins liegt auf Beat 1. B: 126 BPM.
        let ga = BeatGrid::constant(120.0, 0.25, 1);
        let gb = BeatGrid::constant(126.0, 0.1, 0);
        let t = Transition::bass_swap(16);
        // A steht bei Beat 37,5 (Takt 9 + 2,5 Beats, gezählt ab der ersten Eins auf Beat 1).
        let pos_a = ga.sec_at(37.5);
        let clock = 1000.25;
        for (align, want_boundary) in [(1u32, 41i64), (4, 49), (8, 65)] {
            let plan = plan_live(&t, ROLES, clock, deck(&ga, pos_a, 120.0), deck(&gb, 30.0, 126.0), When::Next { align_bars: align }).unwrap();
            assert!((plan.t0 - (clock + want_boundary as f64 - 37.5)).abs() < 1e-9, "Raster {align}: T0 {}", plan.t0);
            assert!(ga.is_downbeat(want_boundary) && (want_boundary - 1).rem_euclid(4 * align as i64) == 0);
            // Der erste vorgemerkte Befehl liegt einen Beat vor T0 und mindestens den Vorlauf nach „jetzt".
            let first = plan.scheduled.first().unwrap().0;
            assert!((first - (plan.t0 - 1.0)).abs() < 1e-9 && first >= clock + LEAD_BEATS - 1e-9);
            assert!((plan.end - (plan.t0 + 64.0 + 1.0)).abs() < 1e-9);
        }
        // Kurz vor einer Taktgrenze reicht der Vorlauf nicht mehr: Es wird die übernächste.
        let plan = plan_live(&t, ROLES, clock, deck(&ga, ga.sec_at(39.5), 120.0), deck(&gb, 30.0, 126.0), When::Next { align_bars: 1 }).unwrap();
        assert!((plan.t0 - (clock + 45.0 - 39.5)).abs() < 1e-9);
    }

    /// Der Einstieg des neuen Decks rückt auf den nächstgelegenen Taktanfang; das Deck bekommt Beat-Sync.
    #[test]
    fn entry_snaps_to_the_nearest_downbeat() {
        let ga = BeatGrid::constant(120.0, 0.0, 0);
        let gb = BeatGrid::constant(126.0, 0.1, 2);
        let t = Transition::fade(8);
        for (beat, want) in [(18.4, 18i64), (19.9, 18), (20.1, 22), (22.0, 22), (0.3, 2)] {
            let plan = plan_live(&t, ROLES, 0.0, deck(&ga, 10.0, 120.0), deck(&gb, gb.sec_at(beat), 126.0), When::Next { align_bars: 1 }).unwrap();
            assert!((plan.in_pos - gb.sec_at(want as f64)).abs() < 1e-9, "Beat {beat}: Einstieg bei Beat {}", gb.beat_at(plan.in_pos));
            assert_eq!(plan.now, vec![Command::Seek { deck: B, pos: plan.in_pos }, Command::Sync { deck: B, mode: SyncMode::Beat }]);
        }
    }

    #[test]
    fn rejects_what_cannot_be_played() {
        let ga = BeatGrid::constant(120.0, 0.0, 0);
        let gb = BeatGrid::constant(140.0, 0.0, 0);
        let t = Transition::bass_swap(32);
        let err = plan_live(&t, ROLES, 0.0, deck(&ga, 10.0, 120.0), deck(&gb, 10.0, 140.0), When::Next { align_bars: 1 }).unwrap_err();
        assert!(matches!(err, LiveError::TempoGap(g, m) if (g - 16.67).abs() < 0.01 && m == 8.0), "{err:?}");
        // A kurz vor dem Ende: 600 s = 300 Takte, ab Takt 280 bleiben keine 32 mehr.
        let gb = BeatGrid::constant(122.0, 0.0, 0);
        let err = plan_live(&t, ROLES, 0.0, deck(&ga, ga.sec_at(280.0 * 4.0), 120.0), deck(&gb, 10.0, 122.0), When::Next { align_bars: 1 }).unwrap_err();
        assert!(matches!(err, LiveError::OutTooShort(left, 32) if left < 32.0), "{err:?}");
        let short = LiveDeck { grid: &gb, playing: false, pos: 10.0, duration: 60.0, track_bpm: 122.0 };
        assert_eq!(plan_live(&t, ROLES, 0.0, deck(&ga, 10.0, 120.0), short, When::Next { align_bars: 1 }).unwrap_err(), LiveError::InTooShort(32));
        assert!(matches!(plan_live(&t, Roles { out: A, inn: A }, 0.0, deck(&ga, 10.0, 120.0), deck(&gb, 10.0, 122.0), When::Next { align_bars: 1 }), Err(LiveError::Invalid(_))));
    }
/// Ende vorgegeben: Der Beginn wird um die Länge zurückgerechnet, das stehende Deck erreicht seine
    /// Stelle genau am Ende – auch wenn es dafür vor seinem Anfang starten muss.
    #[test]
    fn end_anchor_computes_the_start_backwards() {
        let ga = BeatGrid::constant(120.0, 0.25, 1);
        let gb = BeatGrid::constant(126.0, 0.1, 2);
        let t = Transition::bass_swap(16);
        let l = 64.0;
        let clock = 500.0;
        // A bei Beat 37,5; das Ende soll 131 Beats voraus liegen → Beat 168,5, nächster Taktanfang 169.
        let a = deck(&ga, ga.sec_at(37.5), 120.0);
        // B steht bei Beat 66,4 (Taktanfänge 62, 66, 70): Es soll am Ende bei Beat 66 sein.
        let plan = plan_live(&t, ROLES, clock, a, deck(&gb, gb.sec_at(66.4), 126.0), When::EndIn { beats: 131.0 }).unwrap();
        assert!((plan.t0 - (clock + 169.0 - l - 37.5)).abs() < 1e-9, "T0 {}", plan.t0);
        assert!((gb.beat_at(plan.in_pos) - (66.0 - l)).abs() < 1e-9, "B startet bei Beat {}", gb.beat_at(plan.in_pos));
        assert_eq!(plan.now[0], Command::Seek { deck: B, pos: plan.in_pos });
        // Steht B am eigenen Anfang, liegt der Start davor: Vorlauf, den der Track nicht hat.
        let plan = plan_live(&t, ROLES, clock, a, deck(&gb, gb.sec_at(2.0), 126.0), When::EndIn { beats: 131.0 }).unwrap();
        assert!(plan.in_pos < 0.0 && (gb.beat_at(plan.in_pos) - (2.0 - l)).abs() < 1e-9, "B startet bei {} s", plan.in_pos);
        // Liegt das Ende zu nah, sagt die Absage, wie viele Beats fehlen.
        let err = plan_live(&t, ROLES, clock, a, deck(&gb, 30.0, 126.0), When::EndIn { beats: 60.0 }).unwrap_err();
        assert!(matches!(err, LiveError::TooLate(have, 16, need) if (have - 59.5).abs() < 1e-9 && (need - 67.0).abs() < 1e-9), "{err:?}");
    }

    /// Läuft das neue Deck schon mit, wird es weder gestellt noch gestartet; gefahren werden nur die Regler.
    #[test]
    fn a_deck_that_already_runs_is_only_automated() {
        let ga = BeatGrid::constant(120.0, 0.0, 0);
        let gb = BeatGrid::constant(126.0, 0.1, 2);
        let t = Transition::bass_swap(8);
        let running = LiveDeck { grid: &gb, playing: true, pos: gb.sec_at(50.25), duration: 600.0, track_bpm: 126.0 };
        for when in [When::Next { align_bars: 4 }, When::EndIn { beats: 80.0 }] {
            let plan = plan_live(&t, ROLES, 10.0, deck(&ga, ga.sec_at(21.0), 120.0), running, when).unwrap();
            assert!(plan.now.is_empty(), "{when:?}");
            assert!(!plan.scheduled.iter().any(|(_, c)| matches!(c, Command::Play { deck: B, .. } | Command::Seek { .. })), "{when:?}");
            assert!(plan.scheduled.iter().any(|(_, c)| *c == Command::Play { deck: A, on: false }));
            // Seine Stelle bei T0 ergibt sich aus dem Gleichschritt mit der Clock.
            assert!((gb.beat_at(plan.in_pos) - (50.25 + plan.t0 - 10.0)).abs() < 1e-9);
        }
    }

    #[test]
    fn when_uses_the_documented_json_format() {
        assert_eq!(serde_json::to_string(&When::Next { align_bars: 4 }).unwrap(), r#"{"type":"next","align_bars":4}"#);
        assert_eq!(serde_json::from_str::<When>(r#"{"type":"end_in","beats":128}"#).unwrap(), When::EndIn { beats: 128.0 });
    }
}
