//! Automatische Übergänge (Spezifikation 5.10). Bisher umgesetzt: T1, der lange Blend
//! mit Bass-Tausch, samt Tempo-Angleichung (T10) für zwei Tracks. Bewertung, Planung
//! ganzer Sets und die übrigen Techniken folgen in Stufe 5.
//!
//! Ein Übergang ist ein Datenobjekt mit Reglerverläufen ([`Transition`]); dieselbe Beschreibung
//! wird offline gerendert und live abgespielt.

mod blend;
mod live;
mod transition;

pub use blend::{plan_blend, render_blend, BlendPlan, BlendReport, PlanError};
pub use live::{plan_live, LiveDeck, LiveError, LivePlan, When, LEAD_BEATS, MAX_TEMPO_GAP};
pub use transition::{Curve, Lane, Param, Point, Roles, Side, Transition, STEPS_PER_BEAT};
