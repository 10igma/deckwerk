//! Befehlsschicht (Spezifikation 7.4): Jede Bedienhandlung ist ein `Command`.
//! Maus, Tastatur, Automation und Autopilot erzeugen dieselben Befehle.
//! Der Zustand fließt als `Snapshot` zurück an die Oberfläche.

use crate::db_json;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DeckId {
    A,
    B,
    C,
    D,
}

impl DeckId {
    pub const ALL: [DeckId; 4] = [DeckId::A, DeckId::B, DeckId::C, DeckId::D];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn from_index(i: usize) -> Option<DeckId> {
        Self::ALL.get(i).copied()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Band {
    Low,
    Mid,
    High,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SyncMode {
    /// Deck läuft frei.
    #[default]
    Off,
    /// Nur das Tempo folgt der Master-Clock.
    Tempo,
    /// Tempo und Phase folgen der Master-Clock.
    Beat,
}

/// Zuweisung eines Kanals zum Crossfader.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum XfAssign {
    Left,
    Right,
    #[default]
    Thru,
}

/// Kurve des Crossfaders (MIX-06).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum XfCurve {
    /// Gleiche Leistung über den ganzen Weg.
    #[default]
    Smooth,
    /// Volle Lautstärke fast bis zum Anschlag (Cut).
    Sharp,
    /// Crossfader ohne Wirkung.
    Off,
}

/// Bedienbefehl. Werte von Reglern sind in den genannten Einheiten angegeben.
#[derive(Clone, Copy, PartialEq, Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Command {
    /// Wiedergabe starten oder anhalten.
    Play { deck: DeckId, on: bool },
    /// Starten mit Vorlauf (VIS-13): Das Deck springt so weit zurück, dass es seine jetzige Stelle
    /// in `beats` Beats der Clock erreicht, und läuft los. Liegt der Start vor dem Anfang des
    /// Tracks, läuft es bis dorthin stumm. Negative Werte starten entsprechend weiter hinten.
    PlayIn { deck: DeckId, beats: f64 },
    /// Auf eine Position springen (Sekunden im Track).
    Seek { deck: DeckId, pos: f64 },
    /// Sprung im Takt (DECK-20): Ein laufendes Deck landet auf dem Beat-Bruchteil, auf dem es ohne
    /// Sprung gerade wäre – höchstens einen halben Beat neben `pos`. Seine Phase zur Clock bleibt damit
    /// erhalten, gleich wie lange der Befehl unterwegs war. Ein stehendes Deck springt genau auf `pos`.
    SeekSnap { deck: DeckId, pos: f64 },
    /// Tempo-Regler: Abweichung als Faktor, z. B. 0.02 = +2 %.
    Tempo { deck: DeckId, offset: f64 },
    /// Anschieben/Abbremsen: zusätzliche Abweichung, solange gehalten (0 = losgelassen).
    Bend { deck: DeckId, amount: f64 },
    Sync { deck: DeckId, mode: SyncMode },
    /// Master-Deck festlegen. `None` = freie Clock. Schaltet die Automatik aus.
    SetMaster { deck: Option<DeckId> },
    /// Master automatisch wählen: das am längsten laufende Deck (DECK-09).
    MasterAuto,
    /// Tempo der freien Clock in BPM.
    ClockTempo { bpm: f64 },
    /// Kanal-Gain in dB (zusätzlich zum Auto-Gain).
    Gain {
        deck: DeckId,
        #[serde(deserialize_with = "db_json::value")]
        db: f32,
    },
    /// EQ-Band in dB; `-inf` = Kill, Maximum +6.
    Eq {
        deck: DeckId,
        band: Band,
        #[serde(deserialize_with = "db_json::value")]
        db: f32,
    },
    /// Filter-Regler −1 … +1 (0 = aus).
    Filter { deck: DeckId, pos: f32 },
    /// Kanalfader 0 … 1.
    Fader { deck: DeckId, value: f32 },
    /// Vorhören auf dem Kopfhörer.
    Cue { deck: DeckId, on: bool },
    XfAssign { deck: DeckId, assign: XfAssign },
    /// Crossfader −1 (links) … +1 (rechts).
    Crossfader { pos: f32 },
    XfCurve { curve: XfCurve },
    /// Master-Lautstärke in dB.
    MasterVolume {
        #[serde(deserialize_with = "db_json::value")]
        db: f32,
    },
    Limiter { on: bool },
    /// Kopfhörer: 0 = nur Cue, 1 = nur Master.
    CueMix { value: f32 },
    /// Kopfhörer-Lautstärke in dB.
    CueVolume {
        #[serde(deserialize_with = "db_json::value")]
        db: f32,
    },
    /// Verwirft alle vorgemerkten Befehle (einen laufenden Übergang abbrechen, TRANS-05). Die Regler
    /// bleiben stehen, wo sie gerade sind.
    CancelScheduled,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct DeckState {
    pub loaded: bool,
    pub playing: bool,
    /// Position in Sekunden.
    pub pos: f64,
    pub duration: f64,
    /// Beat-Nummer im Beatgrid des Tracks (mit Nachkommaanteil).
    pub beat: f64,
    pub track_bpm: f64,
    /// Aktuell hörbares Tempo.
    pub bpm: f64,
    /// Tempo-Abweichung als Faktor (Regler bzw. Sync).
    pub tempo_offset: f64,
    pub sync: SyncMode,
    pub is_master: bool,
    /// Versatz zur Master-Clock in Beats, −0,5 … +0,5.
    pub phase: f64,
    /// Kanalpegel vor dem Fader, Spitze links/rechts (linear).
    pub peak: [f32; 2],
    #[serde(deserialize_with = "db_json::value")]
    pub gain_db: f32,
    #[serde(deserialize_with = "db_json::triple")]
    pub eq_db: [f32; 3],
    pub filter: f32,
    pub fader: f32,
    pub cue: bool,
    pub xf_assign: XfAssign,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct MixerState {
    pub crossfader: f32,
    pub xf_curve: XfCurve,
    #[serde(deserialize_with = "db_json::value")]
    pub master_db: f32,
    pub limiter_on: bool,
    /// Pegelreduktion des Limiters in dB (≤ 0).
    pub limiter_gr_db: f32,
    pub master_peak: [f32; 2],
    pub cue_mix: f32,
    #[serde(deserialize_with = "db_json::value")]
    pub cue_db: f32,
}

/// Zustand der Engine für die Anzeige.
#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct Snapshot {
    pub clock_bpm: f64,
    pub clock_beats: f64,
    pub master: Option<DeckId>,
    pub master_auto: bool,
    pub decks: [DeckState; 4],
    pub mixer: MixerState,
    /// Bisher verarbeitete Frames.
    pub frames: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_use_the_documented_json_format() {
        let cmd: Command = serde_json::from_str(r#"{"type":"eq","deck":"b","band":"low","db":null}"#).unwrap();
        assert_eq!(cmd, Command::Eq { deck: DeckId::B, band: Band::Low, db: f32::NEG_INFINITY });
        assert_eq!(serde_json::to_string(&cmd).unwrap(), r#"{"type":"eq","deck":"b","band":"low","db":null}"#);
        let cmd: Command = serde_json::from_str(r#"{"type":"sync","deck":"a","mode":"beat"}"#).unwrap();
        assert_eq!(cmd, Command::Sync { deck: DeckId::A, mode: SyncMode::Beat });
        let cmd: Command = serde_json::from_str(r#"{"type":"set_master","deck":null}"#).unwrap();
        assert_eq!(cmd, Command::SetMaster { deck: None });
        assert_eq!(serde_json::from_str::<Command>(r#"{"type":"master_auto"}"#).unwrap(), Command::MasterAuto);
    }

    #[test]
    fn snapshot_with_killed_band_survives_json() {
        let mut s = Snapshot::default();
        s.decks[1].eq_db = [f32::NEG_INFINITY, 0.0, -3.0];
        s.mixer.limiter_gr_db = -1.5;
        let json = serde_json::to_string(&s).unwrap();
        assert!(json.contains("[null,0.0,-3.0]"));
        assert_eq!(serde_json::from_str::<Snapshot>(&json).unwrap(), s);
    }
}
