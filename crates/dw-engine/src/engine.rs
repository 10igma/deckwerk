//! Engine: Decks, Master-Clock, Sync, Mixer-Summen und zeitgenaue Befehle.

use crate::deck::{Deck, Jump};
use crate::BLOCK;
use dw_core::{
    db_to_lin, lin_to_db, AudioBuf, Band, BeatGrid, Command, DeckId, DeckState, MixerState, Snapshot, SyncMode,
    XfAssign, XfCurve,
};
use dw_dsp::{Limiter, Smoother};
use std::f32::consts::FRAC_PI_2;
use std::sync::Arc;

/// Decke des Master-Limiters in dBFS.
const LIMITER_CEILING_DB: f32 = -0.3;
/// Höchstzahl gleichzeitig vorgemerkter Befehle (Speicher wird vorab angefordert).
const SCHEDULE_CAPACITY: usize = 16_384;

/// Spielbereiter Track: Audiodaten in der Abtastrate der Engine, Beatgrid und Auto-Gain.
pub struct Track {
    pub audio: AudioBuf,
    pub grid: BeatGrid,
    /// Auto-Gain in dB (aus der Lautheits-Analyse).
    pub gain_db: f32,
}

struct Scheduled {
    beat: f64,
    cmd: Command,
}

pub struct Engine {
    sr: f64,
    decks: [Deck; 4],
    clock_beats: f64,
    clock_bpm: f64,
    master: Option<usize>,
    /// Clock-Beats minus Track-Beats des Master-Decks.
    master_offset: f64,
    master_auto: bool,
    crossfader: f32,
    xf_curve: XfCurve,
    master_db: f32,
    master_gain: Smoother,
    limiter: Limiter,
    limiter_gr: f32,
    cue_mix_value: f32,
    cue_mix: Smoother,
    cue_db: f32,
    cue_gain: Smoother,
    /// Verzögert die Cue-Summe um die Vorschau des Limiters, damit Cue und Master im Kopfhörer deckungsgleich sind.
    cue_delay: Vec<f32>,
    cue_delay_idx: usize,
    master_bus: [f32; BLOCK * 2],
    cue_bus: [f32; BLOCK * 2],
    master_peak: [f32; 2],
    /// Absteigend nach Zeitpunkt sortiert; der nächste Befehl steht am Ende.
    scheduled: Vec<Scheduled>,
    frames: u64,
}

fn wrap_half(x: f64) -> f64 {
    x - x.round()
}

impl Engine {
    pub fn new(sample_rate: u32) -> Self {
        dw_dsp::interp::init();
        let sr = sample_rate as f64;
        let limiter = Limiter::new(sr, LIMITER_CEILING_DB);
        let look = limiter.lookahead();
        let mut e = Engine {
            sr,
            decks: [Deck::new(sr), Deck::new(sr), Deck::new(sr), Deck::new(sr)],
            clock_beats: 0.0,
            clock_bpm: 120.0,
            master: None,
            master_offset: 0.0,
            master_auto: true,
            crossfader: 0.0,
            xf_curve: XfCurve::Smooth,
            master_db: 0.0,
            master_gain: Smoother::new(1.0, sr, 5.0),
            limiter,
            limiter_gr: 1.0,
            cue_mix_value: 0.0,
            cue_mix: Smoother::new(0.0, sr, 5.0),
            cue_db: 0.0,
            cue_gain: Smoother::new(1.0, sr, 5.0),
            cue_delay: vec![0.0; look * 2],
            cue_delay_idx: 0,
            master_bus: [0.0; BLOCK * 2],
            cue_bus: [0.0; BLOCK * 2],
            master_peak: [0.0; 2],
            scheduled: Vec::with_capacity(SCHEDULE_CAPACITY),
            frames: 0,
        };
        e.update_outputs();
        e
    }

    pub fn sample_rate(&self) -> u32 {
        self.sr as u32
    }

    /// Verzögerung des Master-Ausgangs in Frames (Vorschau des Limiters).
    pub fn latency(&self) -> usize {
        self.limiter.lookahead()
    }

    pub fn clock_beats(&self) -> f64 {
        self.clock_beats
    }

    /// Lädt einen Track. Gibt den bisherigen Track zurück, damit er außerhalb
    /// des Audio-Threads freigegeben werden kann.
    pub fn load(&mut self, deck: DeckId, track: Arc<Track>) -> Option<Arc<Track>> {
        assert_eq!(track.audio.sample_rate as f64, self.sr, "Track muss in der Abtastrate der Engine vorliegen");
        let i = deck.index();
        if self.master == Some(i) {
            self.master = None;
        }
        let d = &mut self.decks[i];
        d.playing = false;
        d.reset_position();
        d.strip.auto_gain_db = track.gain_db;
        d.strip.update_gain();
        d.track.replace(track)
    }

    /// Merkt einen Befehl für einen Zeitpunkt in Clock-Beats vor. Er wird sample-genau ausgeführt.
    /// Gibt `false` zurück, wenn der Vormerk-Speicher voll ist.
    pub fn schedule(&mut self, at_beat: f64, cmd: Command) -> bool {
        if self.scheduled.len() >= SCHEDULE_CAPACITY {
            return false;
        }
        let item = Scheduled { beat: at_beat, cmd };
        // Absteigend sortiert; bei gleichem Zeitpunkt bleibt die Reihenfolge des Vormerkens erhalten.
        let idx = self.scheduled.partition_point(|s| s.beat > item.beat);
        self.scheduled.insert(idx, item);
        true
    }

    fn any_playing_except(&self, i: usize) -> bool {
        self.decks.iter().enumerate().any(|(j, d)| j != i && d.playing)
    }

    /// `adopt_clock_tempo`: Das Deck übernimmt das laufende Clock-Tempo, damit beim Wechsel kein Temposprung entsteht.
    fn set_master(&mut self, new: Option<usize>, adopt_clock_tempo: bool) {
        if new == self.master {
            return;
        }
        if let Some(y) = new {
            let clock_beats = self.clock_beats;
            let clock_bpm = self.clock_bpm;
            let d = &mut self.decks[y];
            let was_locked = adopt_clock_tempo && d.playing && d.sync == SyncMode::Beat;
            if adopt_clock_tempo && d.playing && d.sync != SyncMode::Off && d.track_bpm() > 0.0 {
                d.tempo_offset = clock_bpm / d.track_bpm() - 1.0;
            }
            self.master_offset = if was_locked { d.sync_offset } else { (clock_beats - d.beat()).round() };
        }
        self.master = new;
    }

    fn xf_gain(&self, assign: XfAssign) -> f32 {
        let t = (self.crossfader.clamp(-1.0, 1.0) + 1.0) / 2.0;
        match (self.xf_curve, assign) {
            (XfCurve::Off, _) | (_, XfAssign::Thru) => 1.0,
            (XfCurve::Smooth, XfAssign::Left) => (t * FRAC_PI_2).cos(),
            (XfCurve::Smooth, XfAssign::Right) => (t * FRAC_PI_2).sin(),
            (XfCurve::Sharp, XfAssign::Left) => ((1.0 - t) / 0.05).clamp(0.0, 1.0),
            (XfCurve::Sharp, XfAssign::Right) => (t / 0.05).clamp(0.0, 1.0),
        }
    }

    /// Läuft das Deck und ist es auf dem Master zu hören (Fader offen, Crossfader nicht abgewandt)?
    fn audible(&self, i: usize) -> bool {
        let d = &self.decks[i];
        d.playing && d.strip.fader_value > 0.0 && self.xf_gain(d.strip.xf_assign) > 0.0
    }

    fn update_outputs(&mut self) {
        for i in 0..4 {
            let g = self.xf_gain(self.decks[i].strip.xf_assign);
            self.decks[i].strip.set_output(g);
        }
    }

    /// Führt einen Befehl sofort aus.
    pub fn apply(&mut self, cmd: Command) {
        match cmd {
            Command::Play { deck, on } => {
                let i = deck.index();
                if !self.decks[i].loaded() {
                    return;
                }
                if on {
                    self.start_deck(i);
                } else {
                    self.decks[i].stop();
                }
            }
            Command::PlayIn { deck, beats } => {
                let i = deck.index();
                if !self.decks[i].loaded() || self.decks[i].playing || !beats.is_finite() {
                    return;
                }
                // Folgt das Deck nach dem Start der Clock, läuft es mit deren Tempo, sonst mit dem eigenen.
                let own_tempo = (self.master_auto && !self.any_playing_except(i)) || self.master == Some(i);
                let clock_bpm = self.clock_bpm.max(1.0);
                let sr = self.sr;
                let d = &mut self.decks[i];
                // Ein vorgemerkter Sprung und ein laufendes Ausblenden gelten zuerst: Gezählt wird ab der Stelle, an der das Deck steht.
                d.settle();
                if let Some(Jump::To(p) | Jump::ToInPhase(p)) = d.jump.take() {
                    d.set_position(p);
                }
                let track_bpm = d.track_bpm();
                let rate = if own_tempo || d.sync == SyncMode::Off || track_bpm <= 0.0 { 1.0 + d.tempo_offset } else { clock_bpm / track_bpm };
                d.pos -= beats * 60.0 / clock_bpm * rate.max(0.01) * sr;
                self.start_deck(i);
            }
            Command::CancelScheduled => self.scheduled.clear(),
            Command::Seek { deck, pos } => {
                self.decks[deck.index()].request_jump(Jump::To(pos * self.sr));
            }
            Command::SeekSnap { deck, pos } => {
                self.decks[deck.index()].request_jump(Jump::ToInPhase(pos * self.sr));
            }
            Command::Tempo { deck, offset } => {
                self.decks[deck.index()].tempo_offset = offset.clamp(-0.99, 1.0);
            }
            Command::Bend { deck, amount } => {
                self.decks[deck.index()].bend = amount.clamp(-0.5, 0.5);
            }
            Command::Sync { deck, mode } => {
                let i = deck.index();
                let clock = self.clock_beats;
                let is_master = self.master == Some(i);
                // Sync an einem stehenden Deck wirkt sofort und sichtbar (DECK-08): Es übernimmt das
                // Tempo, statt bis zum Start zu warten. Läuft etwas, ist das Tempo der Clock gemeint
                // (die Phase rastet beim Start ein). Läuft nichts, richtet es sich am anderen Deck
                // aus – am Master, sonst am ersten geladenen – und rückt im Modus „Beat" um höchstens
                // einen halben Beat, sodass beide gleich im Beat stehen.
                if mode != SyncMode::Off && self.decks[i].loaded() && !self.decks[i].playing {
                    let any_playing = self.decks.iter().any(|d| d.playing);
                    let other = self.master.filter(|&m| m != i && self.decks[m].loaded()).or_else(|| (0..4).find(|&j| j != i && self.decks[j].loaded()));
                    let reference = if any_playing {
                        (!is_master).then_some((self.clock_bpm, None))
                    } else {
                        other.map(|r| {
                            let r = &mut self.decks[r];
                            r.settle();
                            (r.track_bpm() * (1.0 + r.tempo_offset), Some(r.beat()))
                        })
                    };
                    if let Some((bpm, ref_beat)) = reference {
                        let sr = self.sr;
                        let d = &mut self.decks[i];
                        d.settle();
                        if bpm > 0.0 && d.track_bpm() > 0.0 {
                            d.tempo_offset = (bpm / d.track_bpm() - 1.0).clamp(-0.99, 1.0);
                        }
                        if let (SyncMode::Beat, Some(ref_beat), None) = (mode, ref_beat, d.jump) {
                            let beat = d.beat();
                            let mut target = beat + wrap_half(ref_beat - beat);
                            let grid = &d.track.as_ref().unwrap().grid;
                            if grid.sec_at(target) < 0.0 && d.pos >= 0.0 {
                                target += 1.0;
                            }
                            let p = grid.sec_at(target) * sr;
                            d.set_position(p);
                        }
                    }
                }
                let d = &mut self.decks[i];
                if mode == SyncMode::Off && d.sync != SyncMode::Off && d.playing && !is_master {
                    // Beim Verlassen des Sync das aktuelle Tempo behalten.
                    d.tempo_offset = d.rate - 1.0;
                }
                d.sync = mode;
                if mode == SyncMode::Beat && !is_master && d.playing {
                    d.sync_offset = (clock - d.beat()).round();
                    d.jump = Some(Jump::Relock);
                }
            }
            Command::SetMaster { deck } => {
                self.master_auto = false;
                self.set_master(deck.map(DeckId::index), true);
            }
            Command::MasterAuto => self.master_auto = true,
            Command::ClockTempo { bpm } => {
                let bpm = bpm.clamp(20.0, 300.0);
                match self.master {
                    Some(m) if self.decks[m].playing && self.decks[m].track_bpm() > 0.0 => {
                        let d = &mut self.decks[m];
                        d.tempo_offset = bpm / d.track_bpm() - 1.0;
                    }
                    _ => self.clock_bpm = bpm,
                }
            }
            Command::Gain { deck, db } => {
                let s = &mut self.decks[deck.index()].strip;
                s.user_gain_db = db;
                s.update_gain();
            }
            Command::Eq { deck, band, db } => {
                let b = match band {
                    Band::Low => 0,
                    Band::Mid => 1,
                    Band::High => 2,
                };
                self.decks[deck.index()].strip.set_eq(b, db);
            }
            Command::Filter { deck, pos } => self.decks[deck.index()].strip.set_filter(pos),
            Command::Fader { deck, value } => {
                self.decks[deck.index()].strip.fader_value = value.clamp(0.0, 1.0);
                self.update_outputs();
            }
            Command::Cue { deck, on } => self.decks[deck.index()].strip.set_cue(on),
            Command::XfAssign { deck, assign } => {
                self.decks[deck.index()].strip.xf_assign = assign;
                self.update_outputs();
            }
            Command::Crossfader { pos } => {
                self.crossfader = pos.clamp(-1.0, 1.0);
                self.update_outputs();
            }
            Command::XfCurve { curve } => {
                self.xf_curve = curve;
                self.update_outputs();
            }
            Command::MasterVolume { db } => {
                self.master_db = db.min(12.0);
                self.master_gain.set(db_to_lin(self.master_db));
            }
            Command::Limiter { on } => self.limiter.enabled = on,
            Command::CueMix { value } => {
                self.cue_mix_value = value.clamp(0.0, 1.0);
                self.cue_mix.set(self.cue_mix_value);
            }
            Command::CueVolume { db } => {
                self.cue_db = db.min(12.0);
                self.cue_gain.set(db_to_lin(self.cue_db));
            }
        }
    }

    /// Berechnet Master- und Kopfhörer-Ausgang. Beide Puffer sind verschachtelt stereo und gleich lang.
    pub fn process(&mut self, master_out: &mut [f32], cue_out: &mut [f32]) {
        assert_eq!(master_out.len(), cue_out.len());
        let total = master_out.len() / 2;
        let mut done = 0;
        while done < total {
            let mut n = (total - done).min(BLOCK);
            // Fällige Befehle ausführen; den Block am nächsten Befehl enden lassen.
            while let Some(next) = self.scheduled.last() {
                let beats_per_frame = self.clock_bpm.max(1.0) / 60.0 / self.sr;
                let k = ((next.beat - self.clock_beats) / beats_per_frame).round();
                if k <= 0.0 {
                    let s = self.scheduled.pop().unwrap();
                    self.apply(s.cmd);
                } else {
                    if k < n as f64 {
                        n = k as usize;
                    }
                    break;
                }
            }
            self.block(n, &mut master_out[done * 2..(done + n) * 2], &mut cue_out[done * 2..(done + n) * 2]);
            done += n;
        }
    }

    /// Startet ein geladenes, stehendes Deck an seiner jetzigen Stelle.
    fn start_deck(&mut self, i: usize) {
        if self.decks[i].playing {
            return;
        }
        let becomes_master = self.master_auto && !self.any_playing_except(i);
        self.decks[i].start(self.frames);
        if becomes_master {
            self.master = None;
            self.set_master(Some(i), false);
        } else if self.master != Some(i) && self.decks[i].sync == SyncMode::Beat {
            // Auf die Phase der Clock einrasten (Sprung um höchstens einen halben Beat).
            // Das Deck war stumm, der Sprung geschieht deshalb sofort.
            let clock = self.clock_beats;
            let sr = self.sr;
            let d = &mut self.decks[i];
            d.sync_offset = (clock - d.beat()).round();
            let p = d.track.as_ref().unwrap().grid.sec_at(clock - d.sync_offset) * sr;
            d.pos = p;
        } else if self.master == Some(i) {
            self.master_offset = (self.clock_beats - self.decks[i].beat()).round();
        }
    }

    fn execute_jumps(&mut self) {
        for i in 0..4 {
            let Some(jump) = self.decks[i].jump else { continue };
            // Ein hörbares Deck blendet erst aus (siehe Deck::render).
            if self.decks[i].audible() && !self.decks[i].silent() {
                continue;
            }
            let clock = self.clock_beats;
            let sr = self.sr;
            let is_master = self.master == Some(i);
            let d = &mut self.decks[i];
            d.jump = None;
            // Sprung im Takt: Das Ziel rückt um höchstens einen halben Beat, sodass das Deck nach dem
            // Sprung denselben Abstand zur Clock hat wie davor. Der Abstand wird hier gemessen, im
            // Augenblick des Sprungs – nicht dort, wo die Oberfläche das Ziel berechnet hat.
            let jump = match jump {
                Jump::ToInPhase(p) => match &d.track {
                    Some(t) if d.playing => {
                        let before = clock - d.beat();
                        let landing = t.grid.beat_at(p / sr);
                        Jump::To(t.grid.sec_at(landing + wrap_half(clock - landing - before)) * sr)
                    }
                    _ => Jump::To(p),
                },
                other => other,
            };
            match jump {
                Jump::ToInPhase(_) => {}
                Jump::To(p) => {
                    d.set_position(p);
                    if is_master {
                        // Der Master gibt den Takt vor: Die Clock bleibt auf seinem Raster. Sie läuft über
                        // den Sprung hinweg weiter (ganze Beats), ein Bruchteil geht an die Clock, nicht
                        // an den Master – sonst richteten sich Decks im Beat-Sync an einer Clock aus, die
                        // neben dem Raster des Masters liegt, und liefen hörbar versetzt.
                        self.master_offset = (clock - d.beat()).round();
                    } else if d.sync == SyncMode::Beat {
                        d.sync_offset = clock - d.beat();
                    }
                }
                Jump::Relock => {
                    if let Some(t) = &d.track {
                        let p = t.grid.sec_at(clock - d.sync_offset) * sr;
                        d.set_position(p);
                    }
                }
            }
        }
    }

    fn block(&mut self, n: usize, mout: &mut [f32], cout: &mut [f32]) {
        self.execute_jumps();

        // Automatische Master-Wahl (DECK-09): das am längsten laufende hörbare Deck.
        // Ist kein laufendes Deck hörbar, bleibt der bisherige Master, solange er läuft.
        if self.master_auto {
            let current_playing = self.master.is_some_and(|m| self.decks[m].playing);
            let current_audible = self.master.is_some_and(|m| self.audible(m));
            if !current_audible {
                let by_start = |e: &Engine, audible: bool| {
                    (0..4).filter(|&i| e.decks[i].playing && (!audible || e.audible(i))).min_by_key(|&i| e.decks[i].started_at)
                };
                match by_start(self, true) {
                    Some(best) => self.set_master(Some(best), true),
                    None if !current_playing => {
                        let best = by_start(self, false);
                        self.set_master(best, true);
                    }
                    None => {}
                }
            }
        }

        let dt = n as f64 / self.sr;
        let mut rendered = [false; 4];

        // Clock: vom Master-Deck getrieben oder frei laufend.
        let master_playing = self.master.filter(|&m| self.decks[m].playing && self.decks[m].jump.is_none());
        if let Some(m) = master_playing {
            let d = &mut self.decks[m];
            let rate = (1.0 + d.tempo_offset + d.bend).max(0.01);
            rendered[m] = d.render(n, rate);
            self.clock_beats = d.beat() + self.master_offset;
            self.clock_bpm = (d.track_bpm() * rate).max(1.0);
        } else {
            self.clock_beats += self.clock_bpm / 60.0 * dt;
        }

        for i in 0..4 {
            if Some(i) == master_playing || !self.decks[i].audible() {
                continue;
            }
            let clock_beats = self.clock_beats;
            let clock_bpm = self.clock_bpm;
            let sr = self.sr;
            let d = &mut self.decks[i];
            let track_bpm = d.track_bpm();
            let locked = d.playing && d.jump.is_none() && track_bpm > 0.0;
            let rate = match d.sync {
                SyncMode::Beat if locked => {
                    // Anschieben verschiebt den Versatz zur Clock.
                    d.sync_offset -= d.bend * clock_bpm / 60.0 * dt;
                    let grid = &d.track.as_ref().unwrap().grid;
                    let target = grid.sec_at(clock_beats - d.sync_offset) * sr;
                    let r = (target - d.pos) / n as f64;
                    if r > 0.0 && r < 4.0 {
                        r
                    } else {
                        // Aus dem Tritt (z. B. nach einem Clock-Sprung): neu einrasten.
                        d.jump = Some(Jump::Relock);
                        clock_bpm / track_bpm
                    }
                }
                SyncMode::Tempo if locked => clock_bpm / track_bpm * (1.0 + d.bend),
                SyncMode::Off if d.playing && d.jump.is_none() => (1.0 + d.tempo_offset + d.bend).max(0.01),
                // Ausblenden: mit der letzten Rate weiterlaufen.
                _ => d.rate,
            };
            rendered[i] = d.render(n, rate);
        }

        self.master_bus[..n * 2].fill(0.0);
        self.cue_bus[..n * 2].fill(0.0);
        for i in 0..4 {
            let d = &mut self.decks[i];
            if rendered[i] {
                d.strip.process(&mut d.buf[..n * 2], &mut self.master_bus[..n * 2], &mut self.cue_bus[..n * 2]);
            } else {
                d.strip.idle(n);
            }
        }

        // Master: Lautstärke → Limiter → Ausgang.
        for frame in self.master_bus[..n * 2].chunks_exact_mut(2) {
            let g = self.master_gain.next();
            frame[0] *= g;
            frame[1] *= g;
        }
        self.limiter.process(&mut self.master_bus[..n * 2]);
        self.limiter_gr = self.limiter_gr.min(self.limiter.take_min_gain());
        mout.copy_from_slice(&self.master_bus[..n * 2]);

        // Kopfhörer: verzögerte Cue-Summe mit dem Master mischen.
        let look2 = self.cue_delay.len();
        for (i, frame) in cout.chunks_exact_mut(2).enumerate() {
            self.master_peak[0] = self.master_peak[0].max(mout[i * 2].abs());
            self.master_peak[1] = self.master_peak[1].max(mout[i * 2 + 1].abs());
            let di = self.cue_delay_idx;
            let delayed = [self.cue_delay[di], self.cue_delay[di + 1]];
            self.cue_delay[di] = self.cue_bus[i * 2];
            self.cue_delay[di + 1] = self.cue_bus[i * 2 + 1];
            self.cue_delay_idx = (di + 2) % look2;
            let mix = self.cue_mix.next();
            let vol = self.cue_gain.next();
            frame[0] = (delayed[0] * (1.0 - mix) + mout[i * 2] * mix) * vol;
            frame[1] = (delayed[1] * (1.0 - mix) + mout[i * 2 + 1] * mix) * vol;
        }
        self.frames += n as u64;
    }

    /// Zustand für die Anzeige. Setzt die Spitzenwert-Speicher zurück.
    pub fn snapshot(&mut self) -> Snapshot {
        let mut snap = Snapshot {
            clock_bpm: self.clock_bpm,
            clock_beats: self.clock_beats,
            master: self.master.and_then(DeckId::from_index),
            master_auto: self.master_auto,
            frames: self.frames,
            ..Default::default()
        };
        for i in 0..4 {
            let is_master = self.master == Some(i);
            let clock = self.clock_beats;
            let d = &mut self.decks[i];
            let track_bpm = d.track_bpm();
            let running = d.playing && d.jump.is_none();
            let follows = running && !is_master && d.sync != SyncMode::Off;
            let rate = if follows { d.rate } else { 1.0 + d.tempo_offset };
            snap.decks[i] = DeckState {
                loaded: d.loaded(),
                playing: d.playing,
                pos: d.pos_sec(),
                duration: d.track.as_ref().map_or(0.0, |t| t.audio.duration()),
                beat: d.beat(),
                track_bpm,
                bpm: track_bpm * rate,
                tempo_offset: rate - 1.0,
                sync: d.sync,
                is_master,
                phase: if d.loaded() { wrap_half(d.beat() - clock) } else { 0.0 },
                peak: std::mem::take(&mut d.strip.peak),
                gain_db: d.strip.user_gain_db,
                eq_db: d.strip.eq_db,
                filter: d.strip.filter_pos,
                fader: d.strip.fader_value,
                cue: d.strip.cue_on,
                xf_assign: d.strip.xf_assign,
            };
        }
        snap.mixer = MixerState {
            crossfader: self.crossfader,
            xf_curve: self.xf_curve,
            master_db: self.master_db,
            limiter_on: self.limiter.enabled,
            limiter_gr_db: lin_to_db(std::mem::replace(&mut self.limiter_gr, 1.0)),
            master_peak: std::mem::take(&mut self.master_peak),
            cue_mix: self.cue_mix_value,
            cue_db: self.cue_db,
        };
        snap
    }
}

/// Offline-Render: berechnet `frames` Frames und gibt Master- und Kopfhörer-Signal zurück.
pub fn render(engine: &mut Engine, frames: usize) -> (AudioBuf, AudioBuf) {
    let mut master = vec![0.0f32; frames * 2];
    let mut cue = vec![0.0f32; frames * 2];
    for (m, c) in master.chunks_mut(512).zip(cue.chunks_mut(512)) {
        engine.process(m, c);
    }
    let sr = engine.sample_rate();
    (AudioBuf::new(sr, master), AudioBuf::new(sr, cue))
}
