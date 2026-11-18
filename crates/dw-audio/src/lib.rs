//! Echtzeit-Anbindung der Engine (Spezifikation 7.3, 4.12): Befehle laufen über
//! Ringpuffer ohne Sperren in den Audio-Thread, der Zustand kommt als Schnappschuss zurück.

#[cfg(feature = "device")]
pub mod device;
mod host;

pub use host::{create, Controller, Processor, Routing};
