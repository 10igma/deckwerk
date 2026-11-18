//! Beatgrid: Zuordnung zwischen Zeit im Track (Sekunden) und Beat-Nummer.

use serde::{Deserialize, Serialize};

/// Tempo-Anker: Ab `pos` (Sekunden, liegt auf einem Beat) gilt `bpm`.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct GridAnchor {
    pub pos: f64,
    pub bpm: f64,
}

/// Beatgrid mit einem oder mehreren Tempo-Ankern.
///
/// Beat 0 liegt auf dem ersten Anker. `bar_offset` (0–3) ist die Nummer des ersten Beats,
/// der eine Eins ist: Beat `b` ist ein Downbeat, wenn `(b - bar_offset) % 4 == 0`.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BeatGrid {
    anchors: Vec<GridAnchor>,
    /// Beat-Nummer jedes Ankers (Anker 0 = Beat 0).
    anchor_beats: Vec<f64>,
    pub bar_offset: u8,
}

impl BeatGrid {
    pub fn constant(bpm: f64, first_beat: f64, bar_offset: u8) -> Self {
        Self::from_anchors(vec![GridAnchor { pos: first_beat, bpm }], bar_offset)
    }

    /// Anker müssen nach Position aufsteigend sein und mindestens einen Eintrag haben.
    pub fn from_anchors(anchors: Vec<GridAnchor>, bar_offset: u8) -> Self {
        assert!(!anchors.is_empty(), "Beatgrid braucht mindestens einen Anker");
        let mut anchor_beats = Vec::with_capacity(anchors.len());
        let mut beat = 0.0;
        for (i, a) in anchors.iter().enumerate() {
            if i > 0 {
                let prev = anchors[i - 1];
                // Anker liegen auf Beats: auf die nächste ganze Beat-Nummer runden.
                beat += ((a.pos - prev.pos) * prev.bpm / 60.0).round();
            }
            anchor_beats.push(beat);
        }
        BeatGrid { anchors, anchor_beats, bar_offset: bar_offset % 4 }
    }

    pub fn anchors(&self) -> &[GridAnchor] {
        &self.anchors
    }

    fn segment_for_time(&self, sec: f64) -> usize {
        match self.anchors.iter().rposition(|a| a.pos <= sec) {
            Some(i) => i,
            None => 0,
        }
    }

    fn segment_for_beat(&self, beat: f64) -> usize {
        match self.anchor_beats.iter().rposition(|&b| b <= beat) {
            Some(i) => i,
            None => 0,
        }
    }

    /// Beat-Nummer (mit Nachkommaanteil) an der Zeit `sec`.
    pub fn beat_at(&self, sec: f64) -> f64 {
        let i = self.segment_for_time(sec);
        let a = self.anchors[i];
        self.anchor_beats[i] + (sec - a.pos) * a.bpm / 60.0
    }

    /// Zeit in Sekunden des Beats `beat`.
    pub fn sec_at(&self, beat: f64) -> f64 {
        let i = self.segment_for_beat(beat);
        let a = self.anchors[i];
        a.pos + (beat - self.anchor_beats[i]) * 60.0 / a.bpm
    }

    pub fn bpm_at(&self, sec: f64) -> f64 {
        self.anchors[self.segment_for_time(sec)].bpm
    }

    pub fn bpm_at_beat(&self, beat: f64) -> f64 {
        self.anchors[self.segment_for_beat(beat)].bpm
    }

    /// Erstes Tempo des Tracks (Anzeigewert bei konstantem Tempo).
    pub fn bpm(&self) -> f64 {
        self.anchors[0].bpm
    }

    pub fn is_downbeat(&self, beat: i64) -> bool {
        (beat - self.bar_offset as i64).rem_euclid(4) == 0
    }

    /// Nächster Beat (ganzzahlig) zur Zeit `sec`.
    pub fn nearest_beat(&self, sec: f64) -> i64 {
        self.beat_at(sec).round() as i64
    }

    /// Erster Downbeat bei oder nach `beat`.
    pub fn next_downbeat(&self, beat: f64) -> i64 {
        let mut b = beat.ceil() as i64;
        while !self.is_downbeat(b) {
            b += 1;
        }
        b
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn constant_grid_roundtrip() {
        let g = BeatGrid::constant(120.0, 0.25, 1);
        assert_eq!(g.beat_at(0.25), 0.0);
        assert!((g.beat_at(1.25) - 2.0).abs() < 1e-12);
        assert!((g.sec_at(4.0) - 2.25).abs() < 1e-12);
        assert!((g.beat_at(0.0) + 0.5).abs() < 1e-12);
        assert!(g.is_downbeat(1) && g.is_downbeat(5) && !g.is_downbeat(0));
        assert_eq!(g.next_downbeat(1.2), 5);
    }

    #[test]
    fn two_anchor_grid_is_continuous() {
        let g = BeatGrid::from_anchors(
            vec![GridAnchor { pos: 0.0, bpm: 120.0 }, GridAnchor { pos: 8.0, bpm: 150.0 }],
            0,
        );
        assert!((g.beat_at(8.0) - 16.0).abs() < 1e-12);
        assert!((g.beat_at(8.4) - 17.0).abs() < 1e-12);
        assert!((g.sec_at(17.0) - 8.4).abs() < 1e-12);
        assert_eq!(g.bpm_at(7.9), 120.0);
        assert_eq!(g.bpm_at(8.0), 150.0);
        for b in [-2.0, 0.0, 3.3, 16.0, 40.5] {
            assert!((g.beat_at(g.sec_at(b)) - b).abs() < 1e-9);
        }
    }
}
