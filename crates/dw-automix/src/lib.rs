//! Automatische Übergänge (Spezifikation 5.10). Bisher umgesetzt: T1, der lange Blend
//! mit Bass-Tausch, samt Tempo-Angleichung (T10) für zwei Tracks. Bewertung, Planung
//! ganzer Sets und die übrigen Techniken folgen in Stufe 5.

mod blend;

pub use blend::{plan_blend, render_blend, BlendPlan, BlendReport, PlanError};
