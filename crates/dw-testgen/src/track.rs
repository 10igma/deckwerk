//! Techno-Track-Generator mit exakter Grundwahrheit (Tempo, Beatgrid, Tonart,
//! Abschnitte, Ereignisse).

use crate::rng::Rng;
use dw_core::{AudioBuf, BeatGrid, Key};
use std::f64::consts::TAU;

/// Abschnittstyp (Spezifikation ANA-09).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SectionKind {
    Intro,
    Main,
    Breakdown,
    Buildup,
    Drop,
    Outro,
}

/// Welche Klangelemente in einem Abschnitt spielen.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Elements {
    /// Kick auf jedem Beat.
    pub kick: bool,
    /// Offbeat-Bass auf dem Grundton.
    pub bass: bool,
    /// Hi-Hats (Muster siehe [`HatPattern`]).
    pub hats: bool,
    /// Akkord-Stab (Dreiklang der Tonart).
    pub stab: bool,
    /// Tonfolge aus allen sieben Leitertönen.
    pub lead: bool,
    /// Fläche (gehaltener Dreiklang).
    pub pad: bool,
    /// Rausch-Riser über den ganzen Abschnitt.
    pub riser: bool,
    /// Roll in den letzten bis zu 4 Takten (1/8 → 1/16 → 1/32).
    pub roll: bool,
}

impl SectionKind {
    /// Übliche Besetzung des Abschnittstyps. Im Breakdown und im Aufbau spielt keine Kick.
    pub fn default_elements(self) -> Elements {
        let none = Elements::default();
        match self {
            SectionKind::Intro | SectionKind::Outro => Elements { kick: true, hats: true, ..none },
            SectionKind::Main => Elements { kick: true, bass: true, hats: true, stab: true, ..none },
            SectionKind::Breakdown => Elements { pad: true, lead: true, ..none },
            SectionKind::Buildup => Elements { pad: true, hats: true, riser: true, roll: true, ..none },
            SectionKind::Drop => {
                Elements { kick: true, bass: true, hats: true, stab: true, lead: true, ..none }
            }
        }
    }
}

/// Ein Abschnitt der Struktur: Typ, Länge in Takten, Besetzung.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SectionSpec {
    pub kind: SectionKind,
    pub bars: u32,
    pub elements: Elements,
}

impl SectionSpec {
    /// Abschnitt mit der üblichen Besetzung des Typs.
    pub fn new(kind: SectionKind, bars: u32) -> Self {
        SectionSpec { kind, bars, elements: kind.default_elements() }
    }

    /// Abschnitt mit eigener Besetzung.
    pub fn with_elements(kind: SectionKind, bars: u32, elements: Elements) -> Self {
        SectionSpec { kind, bars, elements }
    }
}

/// Hi-Hat-Muster.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HatPattern {
    /// Eine Hi-Hat je Beat auf der „Und".
    Offbeat,
    /// Vier Hi-Hats je Beat, die „Und" betont.
    Sixteenth,
}

/// Parameter eines synthetischen Tracks.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackSpec {
    pub name: String,
    pub sample_rate: u32,
    /// Auch gebrochene Werte (z. B. 133,37).
    pub bpm: f64,
    pub key: Key,
    /// Zeit des ersten Beats (Beat 0) in Sekunden; davor ist Stille.
    pub first_beat_sec: f64,
    /// Nummer des ersten Beats, der eine Eins ist (0–3). Bei > 0 beginnt der Track
    /// mit einem Auftakt aus `bar_offset` Beats in der Besetzung des ersten Abschnitts.
    pub bar_offset: u8,
    pub sections: Vec<SectionSpec>,
    pub hats: HatPattern,
    /// Verstimmung aller Töne gegenüber a′ = 440 Hz in Cent.
    pub tuning_cents: f32,
    pub seed: u64,
}

impl TrackSpec {
    /// Track mit Standardwerten: 44,1 kHz, erster Beat bei 0 s, `bar_offset` 0,
    /// Offbeat-Hi-Hats, keine Verstimmung.
    pub fn new(bpm: f64, key: Key, sections: Vec<SectionSpec>, seed: u64) -> Self {
        TrackSpec {
            name: format!("synth-{seed}"),
            sample_rate: 44_100,
            bpm,
            key,
            first_beat_sec: 0.0,
            bar_offset: 0,
            sections,
            hats: HatPattern::Offbeat,
            tuning_cents: 0.0,
            seed,
        }
    }

    /// Summe aller Abschnittslängen in Takten.
    pub fn total_bars(&self) -> u32 {
        self.sections.iter().map(|s| s.bars).sum()
    }
}

/// Abschnitt in der Grundwahrheit. Takt 0 beginnt auf Beat `bar_offset`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Section {
    pub kind: SectionKind,
    pub start_bar: u32,
    pub bars: u32,
}

/// Ereignistyp (Spezifikation 5.8, soweit der Generator ihn erzeugt).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum EventKind {
    KickIn,
    KickOut,
    BassIn,
    BassOut,
    /// Höhen (Hi-Hats) setzen ein.
    HighsIn,
    HighsOut,
    Roll,
    Riser,
}

/// Ereignis in der Grundwahrheit. Alle Positionen liegen auf Beats.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Event {
    pub kind: EventKind,
    /// Position (bei Roll und Riser: Beginn) in Sekunden.
    pub pos: f64,
    /// Ende in Sekunden; bei punktförmigen Ereignissen gleich `pos`.
    pub end: f64,
}

/// Grundwahrheit eines synthetischen Tracks.
///
/// Ereignisse „setzt ein" gibt es auch am Track-Anfang (Beat 0), wenn das Element im
/// ersten Abschnitt spielt; am Track-Ende wird kein „fällt weg" gemeldet.
#[derive(Clone, Debug, PartialEq)]
pub struct GroundTruth {
    pub bpm: f64,
    pub grid: BeatGrid,
    pub key: Key,
    pub tuning_cents: f32,
    pub sections: Vec<Section>,
    pub events: Vec<Event>,
    /// Anzahl gespielter Beats einschließlich Auftakt.
    pub beats: usize,
}

impl GroundTruth {
    /// Zeit in Sekunden, an der Takt `bar` beginnt.
    pub fn bar_start_sec(&self, bar: u32) -> f64 {
        self.grid.sec_at(self.grid.bar_offset as f64 + 4.0 * bar as f64)
    }

    /// Zeit jedes gespielten Beats in Sekunden.
    pub fn beat_times(&self) -> Vec<f64> {
        (0..self.beats).map(|b| self.grid.sec_at(b as f64)).collect()
    }
}

/// Stille nach dem letzten Takt, damit Klänge ausklingen können.
const TAIL_SEC: f64 = 1.0;
/// Spitzenpegel des fertigen Tracks.
const TARGET_PEAK: f32 = 0.89;

const MAJOR_SCALE: [i32; 7] = [0, 2, 4, 5, 7, 9, 11];
/// Natürliches Moll.
const MINOR_SCALE: [i32; 7] = [0, 2, 3, 5, 7, 8, 10];

/// Stab-Muster als Sechzehntel-Positionen im Takt.
const STAB_PATTERNS: [&[u8]; 4] = [&[0, 3, 6, 10], &[2, 6, 10, 14], &[0, 3, 6, 9, 12], &[4, 7, 12, 15]];

/// Erzeugt einen Track und seine Grundwahrheit.
///
/// Jeder Klang beginnt auf dem Frame, der seiner Sollzeit am nächsten liegt
/// (Fehler ≤ ½ Frame, kein Aufsummieren).
pub fn techno_track(spec: &TrackSpec) -> (AudioBuf, GroundTruth) {
    assert!(spec.sample_rate > 0 && spec.bpm > 0.0 && spec.first_beat_sec >= 0.0);
    assert!(!spec.sections.is_empty() && spec.sections.iter().all(|s| s.bars > 0));

    let sr = spec.sample_rate as f64;
    let period = 60.0 / spec.bpm;
    let offset = (spec.bar_offset % 4) as usize;
    let total_bars = spec.total_bars() as usize;
    let total_beats = offset + 4 * total_bars;
    let grid = BeatGrid::constant(spec.bpm, spec.first_beat_sec, offset as u8);
    let frames = ((spec.first_beat_sec + total_beats as f64 * period + TAIL_SEC) * sr).ceil() as usize;

    let mut rng = Rng::new(spec.seed);
    let mut mix = Mix { data: vec![0.0; frames * 2] };
    let frame_at = |beat: f64| ((spec.first_beat_sec + beat * period) * sr).round() as i64;

    // Abschnitt je Takt und Grundwahrheit der Abschnitte.
    let mut section_of_bar = Vec::with_capacity(total_bars);
    let mut sections = Vec::with_capacity(spec.sections.len());
    let mut bar = 0u32;
    for (i, s) in spec.sections.iter().enumerate() {
        sections.push(Section { kind: s.kind, start_bar: bar, bars: s.bars });
        section_of_bar.extend(std::iter::repeat(i).take(s.bars as usize));
        bar += s.bars;
    }
    // Beat, auf dem der Abschnitt beginnt bzw. endet; der Auftakt gehört zum ersten Abschnitt.
    let section_start_beat =
        |i: usize| if i == 0 { 0.0 } else { (offset + 4 * sections[i].start_bar as usize) as f64 };
    let section_end_beat = |i: usize| (offset + 4 * (sections[i].start_bar + sections[i].bars) as usize) as f64;

    // Tonhöhen.
    let tune = spec.tuning_cents as f64 / 1200.0;
    let hz = |midi: i32| 440.0 * 2f64.powf((midi - 69) as f64 / 12.0 + tune);
    let pc = spec.key.pitch_class() as i32;
    let scale = if spec.key.is_minor() { MINOR_SCALE } else { MAJOR_SCALE };
    let third = scale[2];
    let bass_midi = 29 + (pc - 5).rem_euclid(12); // F1 … E2
    let pad_midi = 48 + pc; // C3 … H3
    let stab_midi = 55 + (pc - 7).rem_euclid(12); // G3 … Fis4
    let lead_midi = 67 + (pc - 7).rem_euclid(12); // G4 … Fis5

    // Pegel der Elemente, je Track um ±1,5 dB gestreut.
    let mut level = |base: f32| base * 10f32.powf(rng.range(-1.5, 1.5) as f32 / 20.0);
    let g_kick = level(0.8);
    let g_bass = level(0.4);
    let g_hat = level(0.2);
    let g_stab = level(0.1);
    let g_lead = level(0.1);
    let g_pad = level(0.05);
    let g_riser = level(0.25);
    let g_roll = level(0.3);

    // Einmal berechnete Klänge.
    let kick = render_kick(sr, &mut rng);
    let bass = render_tone(
        sr,
        &[(hz(bass_midi), 1.0), (2.0 * hz(bass_midi), 0.5), (3.0 * hz(bass_midi), 0.25), (4.0 * hz(bass_midi), 0.12)],
        0.42 * period,
        0.004,
        0.3 * period,
        None,
    );
    let stab_partials: Vec<(f64, f32)> = [0, third, 7]
        .iter()
        .flat_map(|&iv| {
            let f = hz(stab_midi + iv);
            [(f, 1.0), (2.0 * f, 0.5), (3.0 * f, 0.25)]
        })
        .collect();
    let stab = render_tone(sr, &stab_partials, 0.25, 0.002, 0.06, Some(&mut rng));
    let lead_notes: Vec<Vec<f32>> = scale
        .iter()
        .map(|&iv| {
            let f = hz(lead_midi + iv);
            render_tone(sr, &[(f, 1.0), (2.0 * f, 0.3)], 0.3, 0.003, 0.08, None)
        })
        .collect();
    let snares: Vec<Vec<f32>> =
        [0.030, 0.018, 0.009].iter().map(|&tau| render_snare(sr, tau, &mut rng)).collect();

    // Muster: Stab-Rhythmus und Tonfolge (16 Achtel über 2 Takte, alle 7 Leitertöne).
    let stab_pattern = STAB_PATTERNS[rng.below(STAB_PATTERNS.len())];
    let mut lead_pattern: Vec<Option<usize>> = [0, 0, 0, 2, 2, 4, 4, 1, 3, 5, 6]
        .iter()
        .map(|&d| Some(d))
        .chain(std::iter::repeat(None).take(5))
        .collect();
    rng.shuffle(&mut lead_pattern);

    // Schlagweise Elemente.
    let mut hat_count = 0usize;
    for b in 0..total_beats {
        // Auftakt: Besetzung des ersten Abschnitts, Lage wie am Ende eines Taktes davor.
        let (sec, in_bar, bar_parity) = if b < offset {
            (0, 4 - offset + b, 1)
        } else {
            let bar = (b - offset) / 4;
            (section_of_bar[bar], (b - offset) % 4, bar % 2)
        };
        let el = spec.sections[sec].elements;
        let bf = b as f64;

        if el.kick {
            mix.add(frame_at(bf), &kick, g_kick, g_kick);
        }
        if el.bass {
            mix.add(frame_at(bf + 0.5), &bass, g_bass, g_bass);
        }
        if el.hats {
            let hits: &[(f64, f32, f64)] = match spec.hats {
                HatPattern::Offbeat => &[(0.5, 1.0, 0.025)],
                HatPattern::Sixteenth => {
                    &[(0.0, 0.5, 0.012), (0.25, 0.35, 0.012), (0.5, 1.0, 0.025), (0.75, 0.35, 0.012)]
                }
            };
            for &(pos, gain, tau) in hits {
                let hat = render_hat(sr, tau, &mut rng);
                // Abwechselnd leicht links und rechts.
                let pan = if hat_count % 2 == 0 { 0.2 } else { -0.2 };
                hat_count += 1;
                mix.add(frame_at(bf + pos), &hat, g_hat * gain * (1.0 - pan), g_hat * gain * (1.0 + pan));
            }
        }
        if el.stab {
            for &p in stab_pattern {
                if p as usize / 4 == in_bar {
                    mix.add(frame_at(bf + (p % 4) as f64 * 0.25), &stab, g_stab, g_stab);
                }
            }
        }
        if el.lead {
            for half in 0..2 {
                if let Some(degree) = lead_pattern[bar_parity * 8 + in_bar * 2 + half] {
                    mix.add(frame_at(bf + half as f64 * 0.5), &lead_notes[degree], g_lead, g_lead);
                }
            }
        }
    }

    // Abschnittsweise Elemente: Fläche, Riser, Roll.
    for (i, s) in spec.sections.iter().enumerate() {
        let start = frame_at(section_start_beat(i)).max(0) as usize;
        let end = (frame_at(section_end_beat(i)).max(0) as usize).min(frames);
        if s.elements.pad {
            let mut partials = Vec::new();
            for iv in [0, third, 7, 12] {
                let f = hz(pad_midi + iv);
                partials.extend([(f, 1.0f32), (2.0 * f, 0.4), (3.0 * f, 0.2)]);
            }
            render_pad(&mut mix, sr, start, end, &partials, g_pad, &mut rng);
        }
        if s.elements.riser {
            render_riser(&mut mix, sr, start, end, g_riser, &mut rng);
        }
        if s.elements.roll {
            let roll_beats = 4.0 * s.bars.min(4) as f64;
            let roll_start = section_end_beat(i) - roll_beats;
            let mut t = 0.0;
            while t < roll_beats - 1e-9 {
                // Erste Hälfte Achtel, drittes Viertel Sechzehntel, letztes Viertel 32stel.
                let (step, density) = if t < 0.5 * roll_beats {
                    (0.5, 0)
                } else if t < 0.75 * roll_beats {
                    (0.25, 1)
                } else {
                    (0.125, 2)
                };
                let gain = g_roll * (0.6 + 0.4 * (t / roll_beats) as f32);
                mix.add(frame_at(roll_start + t), &snares[density], gain, gain);
                t += step;
            }
        }
    }

    // Ereignisse aus den Wechseln der Besetzung.
    let mut events = Vec::new();
    let none = Elements::default();
    for (i, s) in spec.sections.iter().enumerate() {
        let prev = if i == 0 { none } else { spec.sections[i - 1].elements };
        let cur = s.elements;
        let pos = grid.sec_at(section_start_beat(i));
        let end = grid.sec_at(section_end_beat(i));
        let mut point = |was: bool, is: bool, on: EventKind, off: EventKind| {
            if is != was {
                events.push(Event { kind: if is { on } else { off }, pos, end: pos });
            }
        };
        point(prev.kick, cur.kick, EventKind::KickIn, EventKind::KickOut);
        point(prev.bass, cur.bass, EventKind::BassIn, EventKind::BassOut);
        point(prev.hats, cur.hats, EventKind::HighsIn, EventKind::HighsOut);
        if cur.riser {
            events.push(Event { kind: EventKind::Riser, pos, end });
        }
        if cur.roll {
            let roll_start = section_end_beat(i) - 4.0 * s.bars.min(4) as f64;
            events.push(Event { kind: EventKind::Roll, pos: grid.sec_at(roll_start), end });
        }
    }
    events.sort_by(|a, b| a.pos.total_cmp(&b.pos));

    // Auf festen Spitzenpegel bringen.
    let peak = mix.data.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    if peak > 0.0 {
        let g = TARGET_PEAK / peak;
        mix.data.iter_mut().for_each(|s| *s *= g);
    }

    let truth = GroundTruth {
        bpm: spec.bpm,
        grid,
        key: spec.key,
        tuning_cents: spec.tuning_cents,
        sections,
        events,
        beats: total_beats,
    };
    (AudioBuf::new(spec.sample_rate, mix.data), truth)
}

/// Stereo-Mischpuffer (verschachtelt).
struct Mix {
    data: Vec<f32>,
}

impl Mix {
    /// Addiert einen Mono-Klang ab Frame `start` mit getrennten Kanalpegeln.
    fn add(&mut self, start: i64, buf: &[f32], gain_l: f32, gain_r: f32) {
        let frames = (self.data.len() / 2) as i64;
        for (i, &s) in buf.iter().enumerate() {
            let f = start + i as i64;
            if f < 0 {
                continue;
            }
            if f >= frames {
                break;
            }
            self.data[2 * f as usize] += s * gain_l;
            self.data[2 * f as usize + 1] += s * gain_r;
        }
    }
}

/// Kick: Sinus-Sweep 150 → 50 Hz (Zeitkonstante 30 ms) plus Rausch-Klick am Anfang.
fn render_kick(sr: f64, rng: &mut Rng) -> Vec<f32> {
    let len = (0.32 * sr) as usize;
    let tau_pitch = 0.03;
    let fade = (0.02 * sr) as usize;
    (0..len)
        .map(|i| {
            let t = i as f64 / sr;
            let phase = TAU * (50.0 * t + 100.0 * tau_pitch * (1.0 - (-t / tau_pitch).exp()));
            let mut s = phase.sin() * (-t / 0.11).exp();
            if i + fade > len {
                s *= (len - i) as f64 / fade as f64;
            }
            // Klick: 3 ms Rauschen, Zeitkonstante 0,5 ms.
            if t < 0.003 {
                s += 0.5 * rng.bipolar() as f64 * (-t / 0.0005).exp();
            }
            s as f32
        })
        .collect()
}

/// Ton aus Teiltönen `(Frequenz, Pegel)` mit linearem Anstieg, exponentiellem Abklingen
/// und kurzer Ausblende. Mit `rng` bekommen die Teiltöne zufällige Startphasen.
fn render_tone(
    sr: f64,
    partials: &[(f64, f32)],
    seconds: f64,
    attack: f64,
    decay_tau: f64,
    mut rng: Option<&mut Rng>,
) -> Vec<f32> {
    let len = (seconds * sr) as usize;
    let release = (0.01 * sr) as usize;
    let mut out = vec![0.0f32; len];
    for &(freq, amp) in partials {
        let phase0 = rng.as_deref_mut().map_or(0.0, |r| r.range(0.0, TAU));
        for (i, o) in out.iter_mut().enumerate() {
            *o += amp * (phase0 + TAU * freq * i as f64 / sr).sin() as f32;
        }
    }
    for (i, o) in out.iter_mut().enumerate() {
        let t = i as f64 / sr;
        let mut env = (t / attack).min(1.0) * (-t / decay_tau).exp();
        if i + release > len {
            env *= (len - i) as f64 / release as f64;
        }
        *o *= env as f32;
    }
    out
}

/// Hi-Hat: hochpassgefiltertes Rauschen (zweifache Differenz), exponentiell abklingend.
fn render_hat(sr: f64, tau: f64, rng: &mut Rng) -> Vec<f32> {
    let len = (tau * 4.0 * sr) as usize;
    let (mut x1, mut x2) = (0.0f32, 0.0f32);
    (0..len)
        .map(|i| {
            let x = rng.bipolar();
            let y = (x - 2.0 * x1 + x2) * 0.4;
            x2 = x1;
            x1 = x;
            y * (-(i as f64 / sr) / tau).exp() as f32
        })
        .collect()
}

/// Snare für den Roll: Bandpass-Rauschen (ca. 700 Hz–4 kHz), exponentiell abklingend.
fn render_snare(sr: f64, tau: f64, rng: &mut Rng) -> Vec<f32> {
    let len = (tau * 5.0 * sr) as usize;
    let a_hi = 1.0 - (-TAU * 4000.0 / sr).exp() as f32;
    let a_lo = 1.0 - (-TAU * 700.0 / sr).exp() as f32;
    let (mut lp_hi, mut lp_lo) = (0.0f32, 0.0f32);
    (0..len)
        .map(|i| {
            let x = rng.bipolar();
            lp_hi += a_hi * (x - lp_hi);
            lp_lo += a_lo * (x - lp_lo);
            2.0 * (lp_hi - lp_lo) * (-(i as f64 / sr) / tau).exp() as f32
        })
        .collect()
}

/// Fläche: gehaltene Teiltöne von `start` bis `end` mit weichem Ein- und Ausblenden.
fn render_pad(mix: &mut Mix, sr: f64, start: usize, end: usize, partials: &[(f64, f32)], gain: f32, rng: &mut Rng) {
    if end <= start {
        return;
    }
    let len = end - start;
    let fade = ((0.08 * sr) as usize).min(len / 2).max(1);
    let mut buf = vec![0.0f32; len];
    for &(freq, amp) in partials {
        // Drehzeiger statt sin() je Sample; regelmäßig neu normiert.
        let w = TAU * freq / sr;
        let (rot_s, rot_c) = w.sin_cos();
        let phase0 = rng.range(0.0, TAU);
        let (mut s, mut c) = phase0.sin_cos();
        for (i, o) in buf.iter_mut().enumerate() {
            *o += amp * s as f32;
            let ns = s * rot_c + c * rot_s;
            c = c * rot_c - s * rot_s;
            s = ns;
            if i % 4096 == 4095 {
                let norm = (s * s + c * c).sqrt();
                s /= norm;
                c /= norm;
            }
        }
    }
    for i in 0..fade {
        let g = i as f32 / fade as f32;
        buf[i] *= g;
        buf[len - 1 - i] *= g;
    }
    mix.add(start as i64, &buf, gain, gain);
}

/// Riser: Rauschen durch einen zweipoligen Tiefpass, dessen Grenzfrequenz von 200 Hz
/// auf 10 kHz steigt; der Pegel wächst mit. Endet am Abschnittsende mit 5 ms Ausblende.
fn render_riser(mix: &mut Mix, sr: f64, start: usize, end: usize, gain: f32, rng: &mut Rng) {
    if end <= start {
        return;
    }
    let len = end - start;
    let fade = ((0.005 * sr) as usize).max(1);
    let (mut lp1, mut lp2) = (0.0f32, 0.0f32);
    let mut a = 0.0f32;
    let buf: Vec<f32> = (0..len)
        .map(|i| {
            let x = i as f64 / len as f64;
            if i % 64 == 0 {
                let fc = 200.0 * 50f64.powf(x);
                a = 1.0 - (-TAU * fc / sr).exp() as f32;
            }
            lp1 += a * (rng.bipolar() - lp1);
            lp2 += a * (lp1 - lp2);
            let mut g = (0.15 + 0.85 * x * x) as f32;
            if i + fade > len {
                g *= (len - i) as f32 / fade as f32;
            }
            lp2 * g
        })
        .collect();
    mix.add(start as i64, &buf, gain, gain);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small_spec() -> TrackSpec {
        let mut spec = TrackSpec::new(
            133.37,
            Key::new(9, true),
            vec![
                SectionSpec::new(SectionKind::Intro, 4),
                SectionSpec::new(SectionKind::Main, 4),
                SectionSpec::new(SectionKind::Breakdown, 4),
                SectionSpec::new(SectionKind::Buildup, 4),
                SectionSpec::new(SectionKind::Drop, 4),
                SectionSpec::new(SectionKind::Outro, 4),
            ],
            7,
        );
        spec.first_beat_sec = 0.25;
        spec.bar_offset = 2;
        spec
    }

    #[test]
    fn track_is_deterministic() {
        let spec = small_spec();
        let (a, ta) = techno_track(&spec);
        let (b, tb) = techno_track(&spec);
        assert_eq!(a, b);
        assert_eq!(ta, tb);
        let mut other = spec.clone();
        other.seed = 8;
        assert_ne!(techno_track(&other).0, a);
    }

    #[test]
    fn ground_truth_matches_spec() {
        let spec = small_spec();
        let (audio, truth) = techno_track(&spec);
        let period = 60.0 / spec.bpm;
        assert_eq!(truth.bpm, 133.37);
        assert_eq!(truth.key, spec.key);
        assert_eq!(truth.grid.bar_offset, 2);
        assert_eq!(truth.grid.sec_at(0.0), 0.25);
        assert_eq!(truth.beats, 2 + 4 * 24);
        assert_eq!(truth.sections.len(), 6);
        assert_eq!(truth.sections[2], Section { kind: SectionKind::Breakdown, start_bar: 8, bars: 4 });
        assert!((truth.bar_start_sec(8) - (0.25 + 34.0 * period)).abs() < 1e-9);
        let expected = 0.25 + truth.beats as f64 * period + TAIL_SEC;
        assert!((audio.duration() - expected).abs() < 1.0 / 44_100.0);
        assert!((audio.peak() - TARGET_PEAK).abs() < 1e-4);
    }

    #[test]
    fn events_follow_section_changes() {
        let (_, truth) = techno_track(&small_spec());
        let at = |kind: EventKind| -> Vec<f64> {
            truth.events.iter().filter(|e| e.kind == kind).map(|e| e.pos).collect()
        };
        let bar = |b: u32| truth.bar_start_sec(b);
        // Kick: an ab Beat 0, aus im Breakdown (Takt 8), an im Drop (Takt 16).
        assert_eq!(at(EventKind::KickIn), vec![0.25, bar(16)]);
        assert_eq!(at(EventKind::KickOut), vec![bar(8)]);
        assert_eq!(at(EventKind::BassIn), vec![bar(4), bar(16)]);
        assert_eq!(at(EventKind::BassOut), vec![bar(8), bar(20)]);
        assert_eq!(at(EventKind::HighsOut), vec![bar(8)]);
        assert_eq!(at(EventKind::HighsIn), vec![0.25, bar(12)]);
        let riser = truth.events.iter().find(|e| e.kind == EventKind::Riser).unwrap();
        assert_eq!((riser.pos, riser.end), (bar(12), bar(16)));
        let roll = truth.events.iter().find(|e| e.kind == EventKind::Roll).unwrap();
        assert_eq!((roll.pos, roll.end), (bar(12), bar(16)));
    }

    /// RMS im Bereich `[from, to)` Sekunden.
    fn rms(audio: &AudioBuf, from: f64, to: f64) -> f64 {
        let sr = audio.sample_rate as f64;
        let a = (from * sr) as usize * 2;
        let b = ((to * sr) as usize * 2).min(audio.data.len());
        (audio.data[a..b].iter().map(|s| (*s as f64).powi(2)).sum::<f64>() / (b - a) as f64).sqrt()
    }

    #[test]
    fn kick_starts_exactly_on_beats_and_is_missing_in_breakdown() {
        let mut spec = small_spec();
        // Nur Kick im Intro, damit der Einsatz eindeutig messbar ist.
        spec.sections[0].elements = Elements { kick: true, ..Elements::default() };
        let (audio, truth) = techno_track(&spec);
        let sr = audio.sample_rate as f64;
        assert_eq!(rms(&audio, 0.0, 0.2499), 0.0);
        for b in 0..8 {
            let t = truth.grid.sec_at(b as f64);
            let f = (t * sr).round() as usize;
            // Direkt vor dem Beat ist die vorige Kick ausgeklungen, direkt danach ist Pegel da.
            let before = rms(&audio, t - 0.004, t - 0.0005);
            let after = rms(&audio, f as f64 / sr, f as f64 / sr + 0.003);
            assert!(after > 0.1, "Beat {b}: nach dem Einsatz {after}");
            assert!(before < 0.02, "Beat {b}: vor dem Einsatz {before}");
        }
        // Breakdown (Takte 8–11) hat deutlich weniger Pegel als der Hauptteil.
        let main = rms(&audio, truth.bar_start_sec(4), truth.bar_start_sec(8));
        let breakdown = rms(&audio, truth.bar_start_sec(8) + 0.5, truth.bar_start_sec(12));
        assert!(breakdown < 0.5 * main, "Breakdown {breakdown}, Hauptteil {main}");
    }
}
