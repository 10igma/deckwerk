//! Ein Deck: Abspielposition, Tempo, Sync-Zustand und Ein-/Ausblenden bei Start, Stopp und Sprung.

use crate::engine::Track;
use crate::strip::Strip;
use crate::BLOCK;
use dw_core::SyncMode;
use dw_dsp::sample_stereo;
use std::sync::Arc;

/// Sprung, der ausgeführt wird, sobald das Deck stumm ist.
#[derive(Clone, Copy, PartialEq, Debug)]
pub(crate) enum Jump {
    /// Auf eine Frame-Position.
    To(f64),
    /// Auf die Position, die die Master-Clock vorgibt (Beat-Sync).
    Relock,
}

pub(crate) struct Deck {
    pub track: Option<Arc<Track>>,
    /// Position in Frames (darf negativ sein: Vorlauf vor dem Track).
    pub pos: f64,
    pub playing: bool,
    /// Lautstärke des Ein-/Ausblendens, 0 … 1.
    fade: f32,
    fade_in_step: f32,
    fade_out_step: f32,
    /// Position, auf die nach dem Ausblenden beim Stopp zurückgesetzt wird.
    stop_pos: Option<f64>,
    pub jump: Option<Jump>,
    /// Tempo-Regler als Faktor-Abweichung.
    pub tempo_offset: f64,
    pub bend: f64,
    pub sync: SyncMode,
    /// Clock-Beats minus Track-Beats bei Beat-Sync.
    pub sync_offset: f64,
    /// Frame-Zähler der Engine beim Start (für die automatische Master-Wahl).
    pub started_at: u64,
    /// Zuletzt verwendete Abspielrate.
    pub rate: f64,
    pub strip: Strip,
    pub buf: [f32; BLOCK * 2],
    sr: f64,
}

impl Deck {
    pub fn new(sr: f64) -> Self {
        Deck {
            track: None,
            pos: 0.0,
            playing: false,
            fade: 0.0,
            fade_in_step: (1.0 / (0.001 * sr)) as f32,
            fade_out_step: (1.0 / (0.003 * sr)) as f32,
            stop_pos: None,
            jump: None,
            tempo_offset: 0.0,
            bend: 0.0,
            sync: SyncMode::Off,
            sync_offset: 0.0,
            started_at: 0,
            rate: 1.0,
            strip: Strip::new(sr),
            buf: [0.0; BLOCK * 2],
            sr,
        }
    }

    pub fn loaded(&self) -> bool {
        self.track.is_some()
    }

    /// Erzeugt das Deck gerade Ton (läuft oder blendet noch aus)?
    pub fn audible(&self) -> bool {
        self.loaded() && (self.playing || self.fade > 0.0)
    }

    pub fn silent(&self) -> bool {
        self.fade == 0.0
    }

    pub fn pos_sec(&self) -> f64 {
        self.pos / self.sr
    }

    /// Beat-Nummer im Beatgrid des Tracks an der aktuellen Position.
    pub fn beat(&self) -> f64 {
        match &self.track {
            Some(t) => t.grid.beat_at(self.pos_sec()),
            None => 0.0,
        }
    }

    pub fn track_bpm(&self) -> f64 {
        match &self.track {
            Some(t) => t.grid.bpm_at(self.pos_sec()),
            None => 0.0,
        }
    }

    /// Setzt die Position, ohne dass ein Ausblenden sie später überschreibt.
    pub fn set_position(&mut self, frames: f64) {
        self.pos = frames;
        self.stop_pos = None;
        if !self.playing {
            self.fade = 0.0;
        }
    }

    /// Zustand nach dem Laden eines neuen Tracks.
    pub fn reset_position(&mut self) {
        self.pos = 0.0;
        self.fade = 0.0;
        self.stop_pos = None;
        self.jump = None;
        self.rate = 1.0;
    }

    pub fn start(&mut self, now: u64) {
        if self.loaded() && !self.playing {
            // Läuft noch ein Ausblenden vom letzten Stopp, zuerst auf die Stopp-Position zurück.
            if let Some(p) = self.stop_pos.take() {
                self.pos = p;
                self.fade = 0.0;
            }
            // Start ohne Einblenden: Der Anschlag am Cue-Punkt bleibt erhalten.
            self.fade = 1.0;
            self.playing = true;
            self.started_at = now;
        }
    }

    /// Beendet ein laufendes Ausblenden vom letzten Stopp: Das stehende Deck steht danach an seiner Stelle.
    pub fn settle(&mut self) {
        if !self.playing {
            if let Some(p) = self.stop_pos.take() {
                self.pos = p;
            }
            self.fade = 0.0;
        }
    }

    pub fn stop(&mut self) {
        if self.playing {
            self.playing = false;
            self.stop_pos = Some(self.pos);
        }
    }

    /// Springt sofort, wenn das Deck stumm ist, sonst nach kurzem Ausblenden.
    pub fn request_jump(&mut self, jump: Jump) {
        self.jump = Some(jump);
    }

    /// Liest `n` Frames mit Rate `rate` in `self.buf`. Gibt zurück, ob Ton erzeugt wurde.
    pub fn render(&mut self, n: usize, rate: f64) -> bool {
        let Some(track) = self.track.clone() else { return false };
        if !self.audible() {
            return false;
        }
        self.rate = rate;
        let data = &track.audio.data;
        let fading_for_jump = self.jump.is_some();
        for i in 0..n {
            let target = if self.playing && !fading_for_jump { 1.0 } else { 0.0 };
            if self.fade < target {
                self.fade = (self.fade + self.fade_in_step).min(1.0);
            } else if self.fade > target {
                self.fade = (self.fade - self.fade_out_step).max(0.0);
            }
            let (l, r) = sample_stereo(data, self.pos);
            self.buf[i * 2] = l * self.fade;
            self.buf[i * 2 + 1] = r * self.fade;
            self.pos += rate;
        }
        if !self.playing && self.fade == 0.0 {
            if let Some(p) = self.stop_pos.take() {
                self.pos = p;
            }
        }
        // Trackende: anhalten.
        if self.playing && self.pos >= track.audio.frames() as f64 {
            self.playing = false;
            self.fade = 0.0;
            self.stop_pos = None;
            self.pos = track.audio.frames() as f64;
        }
        true
    }
}
