//! Pegel in JSON: −∞ dB (Kill, Stille) wird als `null` übertragen. serde_json schreibt
//! nicht-endliche Zahlen von sich aus als `null`; diese Funktionen lesen sie wieder ein.

use serde::{Deserialize, Deserializer};

/// Für ein einzelnes Feld: `#[serde(deserialize_with = "db_json::value")]`.
pub fn value<'de, D: Deserializer<'de>>(d: D) -> Result<f32, D::Error> {
    Ok(Option::<f32>::deserialize(d)?.unwrap_or(f32::NEG_INFINITY))
}

/// Für die drei EQ-Bänder: `#[serde(deserialize_with = "db_json::triple")]`.
pub fn triple<'de, D: Deserializer<'de>>(d: D) -> Result<[f32; 3], D::Error> {
    let v = <[Option<f32>; 3]>::deserialize(d)?;
    Ok(v.map(|x| x.unwrap_or(f32::NEG_INFINITY)))
}
