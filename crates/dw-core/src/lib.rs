//! Grundtypen von Deckwerk: Audio-Puffer, Beatgrid, Tonart, Analyseergebnis,
//! Befehle und Zustands-Schnappschuss (Spezifikation 7.4).

pub mod analysis;
pub mod audio;
pub mod command;
pub mod grid;
pub mod key;

pub use analysis::{TrackAnalysis, Waveform, WaveformLevel, ANALYSIS_VERSION};
pub use audio::AudioBuf;
pub use command::{Band, Command, DeckId, DeckState, MixerState, Snapshot, SyncMode, XfAssign, XfCurve};
pub use grid::{BeatGrid, GridAnchor};
pub use key::{harmonic_compat, Key};

/// Umrechnung Dezibel → linearer Faktor. `-inf` ergibt 0.
pub fn db_to_lin(db: f32) -> f32 {
    if db == f32::NEG_INFINITY {
        0.0
    } else {
        10f32.powf(db / 20.0)
    }
}

/// Umrechnung linearer Faktor → Dezibel. 0 ergibt `-inf`.
pub fn lin_to_db(lin: f32) -> f32 {
    if lin <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * lin.log10()
    }
}
