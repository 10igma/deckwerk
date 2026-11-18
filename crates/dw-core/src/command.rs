//! Befehlsschicht (Spezifikation 7.4): Jede Bedienhandlung ist ein `Command`.
//! Maus, Tastatur, Automation und Autopilot erzeugen dieselben Befehle.
//! Der Zustand fließt als `Snapshot` zurück an die Oberfläche.

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
    /// Auf eine Position springen (Sekunden im Track).
    Seek { deck: DeckId, pos: f64 },
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
    Gain { deck: DeckId, db: f32 },
    /// EQ-Band in dB; `-inf` = Kill, Maximum +6.
    Eq { deck: DeckId, band: Band, db: f32 },
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
    MasterVolume { db: f32 },
    Limiter { on: bool },
    /// Kopfhörer: 0 = nur Cue, 1 = nur Master.
    CueMix { value: f32 },
    /// Kopfhörer-Lautstärke in dB.
    CueVolume { db: f32 },
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
    pub gain_db: f32,
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
    pub master_db: f32,
    pub limiter_on: bool,
    /// Pegelreduktion des Limiters in dB (≤ 0).
    pub limiter_gr_db: f32,
    pub master_peak: [f32; 2],
    pub cue_mix: f32,
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
