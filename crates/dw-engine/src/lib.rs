//! Audio-Engine von Deckwerk (Spezifikation 7.3–7.7): vier Decks, Mixer, Master-Clock
//! und Sync. Dieselbe Engine läuft in Echtzeit und offline (ENG-07).
//!
//! `Engine::process` fordert keinen Speicher an und sperrt nichts (ENG-06).

mod deck;
mod engine;
mod strip;

pub use engine::{render, Engine, Track};

/// Interne Blockgröße in Frames. Regler werden pro Sample geglättet,
/// Tempo und Sync pro Block berechnet.
pub const BLOCK: usize = 64;
