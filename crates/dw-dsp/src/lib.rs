//! Bausteine der Signalverarbeitung: Glättung, Biquad, Isolator-EQ, Filter-Regler,
//! Limiter und Sinc-Interpolation (Spezifikation 5.13, 5.14).

pub mod biquad;
pub mod filter;
pub mod interp;
pub mod isolator;
pub mod limiter;
pub mod resample;
pub mod smooth;

pub use biquad::{Biquad, Coeffs};
pub use filter::FilterKnob;
pub use interp::sample_stereo;
pub use isolator::Isolator;
pub use limiter::Limiter;
pub use smooth::Smoother;
