//! Kanalzug eines Decks: Gain → EQ → Filter → Pegelmessung → Cue-Abgriff → Fader/Crossfader.

use dw_core::{db_to_lin, XfAssign};
use dw_dsp::{FilterKnob, Isolator, Smoother};

pub(crate) struct Strip {
    pub user_gain_db: f32,
    pub auto_gain_db: f32,
    gain: Smoother,
    eq: Isolator,
    pub eq_db: [f32; 3],
    filter: FilterKnob,
    pub filter_pos: f32,
    pub fader_value: f32,
    /// Fader × Crossfader-Anteil, geglättet.
    out_gain: Smoother,
    pub cue_on: bool,
    cue_gain: Smoother,
    pub xf_assign: XfAssign,
    pub peak: [f32; 2],
}

impl Strip {
    pub fn new(sr: f64) -> Self {
        Strip {
            user_gain_db: 0.0,
            auto_gain_db: 0.0,
            gain: Smoother::new(1.0, sr, 5.0),
            eq: Isolator::new(sr),
            eq_db: [0.0; 3],
            filter: FilterKnob::new(sr),
            filter_pos: 0.0,
            fader_value: 1.0,
            out_gain: Smoother::new(1.0, sr, 5.0),
            cue_on: false,
            cue_gain: Smoother::new(0.0, sr, 5.0),
            xf_assign: XfAssign::Thru,
            peak: [0.0; 2],
        }
    }

    pub fn update_gain(&mut self) {
        self.gain.set(db_to_lin(self.user_gain_db.clamp(f32::NEG_INFINITY, 12.0) + self.auto_gain_db));
    }

    pub fn set_eq(&mut self, band: usize, db: f32) {
        let db = if db.is_nan() { 0.0 } else { db.min(6.0) };
        self.eq_db[band] = db;
        self.eq.set_gain(band, db_to_lin(db));
    }

    pub fn set_filter(&mut self, pos: f32) {
        self.filter_pos = pos.clamp(-1.0, 1.0);
        self.filter.set_pos(self.filter_pos);
    }

    pub fn set_cue(&mut self, on: bool) {
        self.cue_on = on;
        self.cue_gain.set(if on { 1.0 } else { 0.0 });
    }

    /// `xf_gain`: Anteil des Crossfaders für diesen Kanal (0 … 1).
    pub fn set_output(&mut self, xf_gain: f32) {
        // Standard-Kurve des Kanalfaders: quadratisch (MIX-05).
        let v = self.fader_value.clamp(0.0, 1.0);
        self.out_gain.set(v * v * xf_gain);
    }

    /// Verarbeitet `buf` (verschachtelt) und mischt auf Master- und Cue-Summe.
    pub fn process(&mut self, buf: &mut [f32], master: &mut [f32], cue: &mut [f32]) {
        for frame in buf.chunks_exact_mut(2) {
            let g = self.gain.next();
            frame[0] *= g;
            frame[1] *= g;
        }
        self.eq.process(buf);
        self.filter.process(buf);
        for ((frame, m), c) in buf.chunks_exact(2).zip(master.chunks_exact_mut(2)).zip(cue.chunks_exact_mut(2)) {
            self.peak[0] = self.peak[0].max(frame[0].abs());
            self.peak[1] = self.peak[1].max(frame[1].abs());
            let cg = self.cue_gain.next();
            c[0] += frame[0] * cg;
            c[1] += frame[1] * cg;
            let og = self.out_gain.next();
            m[0] += frame[0] * og;
            m[1] += frame[1] * og;
        }
    }

    /// Hält die Glättungen in Bewegung, wenn das Deck stumm ist.
    pub fn idle(&mut self, frames: usize) {
        for _ in 0..frames {
            self.gain.next();
            self.cue_gain.next();
            self.out_gain.next();
        }
    }
}
