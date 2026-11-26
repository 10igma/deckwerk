//! Echtzeit-Anbindung der Engine (Spezifikation 7.3, 4.12): Befehle laufen Ã¼ber
//! Ringpuffer ohne Sperren in den Audio-Thread, der Zustand kommt als Schnappschuss zurÃ¼ck.

#[cfg(feature = "device")]
pub mod device;
mod host;

pub use host::{create, Controller, ErrorCounter, MasterTap, Processor, Routing};
