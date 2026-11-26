//! Ein Übergang als Datenobjekt: Reglerverläufe (Automation) beider Decks über eine Länge in Takten.
//!
//! Dieselbe Beschreibung wird offline gerendert und live abgespielt: [`Transition::commands`] macht
//! daraus vorgemerkte Befehle der Befehlsschicht (7.4), die die Engine sample-genau ausführt. Das
//! Format entspricht dem Abschnitt „Automation" des Mix-Plans (6.3): je Regler Stützpunkte mit
//! Master-Beat relativ zu T0, Wert und Kurvenform.

use dw_core::{Band, Command, DeckId};
use serde::{Deserialize, Deserializer, Serialize};

/// Schritte je Beat, in denen ein Verlauf gesendet wird (die Engine glättet dazwischen).
pub const STEPS_PER_BEAT: u32 = 4;

/// Seite des Übergangs: das Deck, das geht, oder das Deck, das kommt.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Side {
    Out,
    In,
}

/// Regler, den eine Spur fährt. Werte: Fader 0 … 1; EQ in dB (`-inf` = Kill, in JSON `null`);
/// Filter −1 … +1.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Param {
    Fader,
    EqLow,
    EqMid,
    EqHigh,
    Filter,
}

impl Param {
    /// Ruhestellung des Reglers.
    pub fn neutral(self) -> f32 {
        match self {
            Param::Fader => 1.0,
            Param::EqLow | Param::EqMid | Param::EqHigh | Param::Filter => 0.0,
        }
    }

    fn is_eq(self) -> bool {
        matches!(self, Param::EqLow | Param::EqMid | Param::EqHigh)
    }

    fn command(self, deck: DeckId, value: f32) -> Command {
        match self {
            Param::Fader => Command::Fader { deck, value: value.clamp(0.0, 1.0) },
            Param::EqLow => Command::Eq { deck, band: Band::Low, db: value.min(6.0) },
            Param::EqMid => Command::Eq { deck, band: Band::Mid, db: value.min(6.0) },
            Param::EqHigh => Command::Eq { deck, band: Band::High, db: value.min(6.0) },
            Param::Filter => Command::Filter { deck, pos: value.clamp(-1.0, 1.0) },
        }
    }
}

/// Form des Wegs vom vorigen Stützpunkt zu diesem.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Curve {
    #[default]
    Linear,
    /// Weich anlaufend und auslaufend (halbe Kosinuswelle).
    Smooth,
    /// Der vorige Wert bleibt stehen, am Stützpunkt springt der Regler.
    Step,
}

fn db_or_kill<'de, D: Deserializer<'de>>(d: D) -> Result<f32, D::Error> {
    Ok(Option::<f32>::deserialize(d)?.unwrap_or(f32::NEG_INFINITY))
}

/// Stützpunkt: Zeitpunkt in Master-Beats relativ zu T0 (negativ = davor) und Wert.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Point {
    pub beat: f64,
    #[serde(deserialize_with = "db_or_kill")]
    pub value: f32,
    #[serde(default)]
    pub curve: Curve,
}

impl Point {
    pub fn new(beat: f64, value: f32) -> Point {
        Point { beat, value, curve: Curve::Linear }
    }

    pub fn step(beat: f64, value: f32) -> Point {
        Point { beat, value, curve: Curve::Step }
    }
}

/// Automationsspur eines Reglers. Die Stützpunkte sind nach `beat` aufsteigend sortiert.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Lane {
    pub side: Side,
    pub param: Param,
    pub points: Vec<Point>,
}

fn db_to_lin(db: f32) -> f32 {
    if db == f32::NEG_INFINITY {
        0.0
    } else {
        10f32.powf(db / 20.0)
    }
}

fn lin_to_db(lin: f32) -> f32 {
    if lin <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * lin.log10()
    }
}

impl Lane {
    /// Wert zwischen zwei Stützpunkten; `t` 0 … 1. EQ-Verläufe von oder zu Kill laufen linear im
    /// Pegelfaktor (sonst wäre der Weg fast überall unhörbar), alle anderen linear im Wert.
    fn between(&self, from: Point, to: Point, t: f64) -> f32 {
        let t = match to.curve {
            Curve::Linear => t,
            Curve::Smooth => 0.5 - 0.5 * (std::f64::consts::PI * t).cos(),
            Curve::Step => {
                if t >= 1.0 {
                    1.0
                } else {
                    0.0
                }
            }
        } as f32;
        if self.param.is_eq() && (from.value == f32::NEG_INFINITY || to.value == f32::NEG_INFINITY) {
            let (a, b) = (db_to_lin(from.value), db_to_lin(to.value));
            lin_to_db(a + (b - a) * t)
        } else {
            from.value + (to.value - from.value) * t
        }
    }

    /// Stellung des Reglers zum Zeitpunkt `beat`; vor dem ersten Stützpunkt `None` (die Spur greift
    /// dort noch nicht ein), nach dem letzten dessen Wert.
    pub fn value_at(&self, beat: f64) -> Option<f32> {
        let first = self.points.first()?;
        if beat < first.beat {
            return None;
        }
        for pair in self.points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            if beat < b.beat {
                return Some(self.between(a, b, (beat - a.beat) / (b.beat - a.beat)));
            }
        }
        self.points.last().map(|p| p.value)
    }

    /// Die Spur als Folge von (Beat relativ zu T0, Wert).
    fn steps(&self) -> Vec<(f64, f32)> {
        let mut out = Vec::new();
        let Some(first) = self.points.first() else { return out };
        out.push((first.beat, first.value));
        for pair in self.points.windows(2) {
            let (a, b) = (pair[0], pair[1]);
            // Bleibt der Wert stehen oder springt er, genügt der Endpunkt.
            let flat = a.value == b.value || b.curve == Curve::Step || b.beat <= a.beat;
            let n = if flat { 1 } else { (((b.beat - a.beat) * STEPS_PER_BEAT as f64).round() as u32).max(1) };
            for i in 1..=n {
                let t = i as f64 / n as f64;
                out.push((a.beat + (b.beat - a.beat) * t, self.between(a, b, t)));
            }
        }
        out
    }
}

/// Welches Deck geht und welches kommt.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Roles {
    pub out: DeckId,
    pub inn: DeckId,
}

impl Roles {
    fn deck(self, side: Side) -> DeckId {
        match side {
            Side::Out => self.out,
            Side::In => self.inn,
        }
    }
}

/// Ein Übergang: Länge, Start und Stopp der Decks und die Automationsspuren.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Transition {
    /// Kennung der Vorlage oder frei gewählter Name.
    pub name: String,
    /// Länge L in Takten.
    pub bars: u32,
    /// Das kommende Deck startet zu diesem Beat relativ zu T0 (üblich: 0). `None` = es läuft schon.
    pub start_in: Option<f64>,
    /// Das gehende Deck stoppt zu diesem Beat relativ zu T0 (üblich: L). `None` = es läuft weiter.
    pub stop_out: Option<f64>,
    /// Nach dem Stopp die Regler des gehenden Decks in Ruhestellung bringen, damit der nächste
    /// Track dort nicht mit zugedrehtem EQ beginnt.
    #[serde(default)]
    pub reset_out: bool,
    pub lanes: Vec<Lane>,
}

impl Transition {
    pub fn beats(&self) -> f64 {
        self.bars as f64 * 4.0
    }

    /// Frühester Zeitpunkt, zu dem der Übergang einen Regler anfasst, in Beats relativ zu T0 (≤ 0).
    pub fn first_beat(&self) -> f64 {
        let lanes = self.lanes.iter().filter_map(|l| l.points.first()).map(|p| p.beat);
        lanes.chain(self.start_in).fold(0.0, f64::min)
    }

    /// Letzter Zeitpunkt, zu dem der Übergang etwas tut, in Beats relativ zu T0.
    pub fn last_beat(&self) -> f64 {
        let lanes = self.lanes.iter().filter_map(|l| l.points.last()).map(|p| p.beat);
        let end = lanes.chain(self.stop_out).fold(self.beats(), f64::max);
        if self.reset_out && self.stop_out.is_some() {
            end.max(self.stop_out.unwrap() + 1.0)
        } else {
            end
        }
    }

    /// Spur eines Reglers, falls der Übergang ihn fährt.
    pub fn lane(&self, side: Side, param: Param) -> Option<&Lane> {
        self.lanes.iter().find(|l| l.side == side && l.param == param)
    }

    /// Prüft, was sich abspielen lässt: Länge, sortierte Stützpunkte, Werte im Bereich des Reglers.
    pub fn validate(&self) -> Result<(), String> {
        if self.bars == 0 || self.bars > 256 {
            return Err(format!("Länge {} Takte liegt außerhalb von 1 … 256", self.bars));
        }
        for (i, lane) in self.lanes.iter().enumerate() {
            if self.lanes[..i].iter().any(|l| l.side == lane.side && l.param == lane.param) {
                return Err(format!("Spur {:?}/{:?} kommt doppelt vor", lane.side, lane.param));
            }
            if lane.points.is_empty() {
                return Err(format!("Spur {:?}/{:?} hat keine Stützpunkte", lane.side, lane.param));
            }
            for pair in lane.points.windows(2) {
                if pair[1].beat < pair[0].beat {
                    return Err(format!("Spur {:?}/{:?}: Stützpunkte sind nicht nach Zeit sortiert", lane.side, lane.param));
                }
            }
            for p in &lane.points {
                let ok = p.beat.is_finite()
                    && match lane.param {
                        Param::Fader => (0.0..=1.0).contains(&p.value),
                        Param::Filter => (-1.0..=1.0).contains(&p.value),
                        _ => p.value == f32::NEG_INFINITY || (p.value.is_finite() && p.value <= 6.0),
                    };
                if !ok {
                    return Err(format!("Spur {:?}/{:?}: Wert {} bei Beat {} ist nicht erlaubt", lane.side, lane.param, p.value, p.beat));
                }
            }
        }
        Ok(())
    }

    /// Der Übergang als vorgemerkte Befehle: (Clock-Beat, Befehl), aufsteigend nach Zeit. `t0` ist
    /// der Clock-Beat, an dem der Übergang beginnt.
    pub fn commands(&self, t0: f64, roles: Roles) -> Vec<(f64, Command)> {
        let mut out: Vec<(f64, Command)> = Vec::new();
        for lane in &self.lanes {
            let deck = roles.deck(lane.side);
            let mut last: Option<f32> = None;
            for (beat, value) in lane.steps() {
                // Gleiche Werte hintereinander nicht doppelt senden.
                if last != Some(value) {
                    out.push((t0 + beat, lane.param.command(deck, value)));
                    last = Some(value);
                }
            }
        }
        if let Some(beat) = self.start_in {
            out.push((t0 + beat, Command::Play { deck: roles.inn, on: true }));
        }
        if let Some(beat) = self.stop_out {
            out.push((t0 + beat, Command::Play { deck: roles.out, on: false }));
            if self.reset_out {
                for lane in self.lanes.iter().filter(|l| l.side == Side::Out) {
                    out.push((t0 + beat + 1.0, lane.param.command(roles.out, lane.param.neutral())));
                }
            }
        }
        // Stabil sortieren: Bei gleichem Zeitpunkt bleibt die Reihenfolge oben erhalten
        // (Regler vor Start und Stopp).
        out.sort_by(|a, b| a.0.total_cmp(&b.0));
        out
    }

    /// T1 – langer Blend mit Bass-Tausch (5.10). Tauschpunkt S in der Mitte.
    pub fn bass_swap(bars: u32) -> Transition {
        let l = bars as f64 * 4.0;
        let s = l / 2.0;
        let kill = f32::NEG_INFINITY;
        let lane = |side, param, points: Vec<Point>| Lane { side, param, points };
        Transition {
            name: "bass_swap".into(),
            bars,
            start_in: Some(0.0),
            stop_out: Some(l),
            reset_out: true,
            lanes: vec![
                // B: zu, ohne Bass, Mitten und Höhen zurückgenommen; dann öffnen.
                lane(Side::In, Param::Fader, vec![Point::new(-1.0, 0.0), Point::new(0.0, 0.0), Point::new(l / 8.0, 1.0)]),
                lane(Side::In, Param::EqLow, vec![Point::new(-1.0, kill), Point::new(s - 1.0, kill), Point::new(s, 0.0)]),
                lane(Side::In, Param::EqMid, vec![Point::new(-1.0, -6.0), Point::new(0.0, -6.0), Point::new(s, 0.0)]),
                lane(Side::In, Param::EqHigh, vec![Point::new(-1.0, -3.0), Point::new(0.0, -3.0), Point::new(s / 2.0, 0.0)]),
                // A: Bass im letzten Beat vor S heraus, danach Mitten und Höhen zurück, zuletzt der Fader.
                lane(Side::Out, Param::EqLow, vec![Point::new(s - 1.0, 0.0), Point::new(s, kill)]),
                lane(Side::Out, Param::EqMid, vec![Point::new(s, 0.0), Point::new(l - l / 8.0, -9.0)]),
                lane(Side::Out, Param::EqHigh, vec![Point::new(s, 0.0), Point::new(l - l / 8.0, -12.0)]),
                lane(Side::Out, Param::Fader, vec![Point::new(l - l / 4.0, 1.0), Point::new(l, 0.0)]),
            ],
        }
    }

    /// Einfaches Überblenden: B kommt über die erste Hälfte, A geht über die zweite; der Bass
    /// wechselt weich um die Mitte, damit nie zwei volle Bässe lange zusammen laufen.
    pub fn fade(bars: u32) -> Transition {
        let l = bars as f64 * 4.0;
        let s = l / 2.0;
        let kill = f32::NEG_INFINITY;
        let smooth = |beat, value| Point { beat, value, curve: Curve::Smooth };
        let lane = |side, param, points: Vec<Point>| Lane { side, param, points };
        // Der Bass wechselt über zwei Takte um S (bei kurzen Übergängen entsprechend kürzer).
        let w = (l / 8.0).min(4.0);
        Transition {
            name: "fade".into(),
            bars,
            start_in: Some(0.0),
            stop_out: Some(l),
            reset_out: true,
            lanes: vec![
                lane(Side::In, Param::Fader, vec![Point::new(-1.0, 0.0), Point::new(0.0, 0.0), smooth(s, 1.0)]),
                lane(Side::In, Param::EqLow, vec![Point::new(-1.0, kill), Point::new(s - w, kill), Point::new(s + w, 0.0)]),
                lane(Side::Out, Param::EqLow, vec![Point::new(s - w, 0.0), Point::new(s + w, kill)]),
                lane(Side::Out, Param::Fader, vec![Point::new(s, 1.0), smooth(l, 0.0)]),
            ],
        }
    }

    /// Die eingebauten Vorlagen für eine Länge.
    pub fn templates(bars: u32) -> Vec<Transition> {
        vec![Transition::bass_swap(bars), Transition::fade(bars)]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use DeckId::{A, B};

    const ROLES: Roles = Roles { out: A, inn: B };

    fn values(cmds: &[(f64, Command)], want: impl Fn(&Command) -> Option<f32>) -> Vec<(f64, f32)> {
        cmds.iter().filter_map(|(b, c)| want(c).map(|v| (*b, v))).collect()
    }

    /// TRANS-01: Die Verläufe der Vorlage T1 stehen an den Rasterpunkten genau dort, wo 5.10 sie nennt.
    #[test]
    fn bass_swap_follows_the_template() {
        let t = Transition::bass_swap(32);
        t.validate().unwrap();
        let (l, s) = (128.0, 64.0);
        let at = |side, param, beat| t.lane(side, param).unwrap().value_at(beat);
        // B: zu und ohne Bass bis T0, offen nach L/8, Bass erst am Tauschpunkt.
        assert_eq!(at(Side::In, Param::Fader, 0.0), Some(0.0));
        assert_eq!(at(Side::In, Param::Fader, l / 8.0), Some(1.0));
        assert_eq!(at(Side::In, Param::Fader, l / 16.0), Some(0.5));
        assert_eq!(at(Side::In, Param::EqLow, s - 1.0), Some(f32::NEG_INFINITY));
        assert_eq!(at(Side::In, Param::EqLow, s), Some(0.0));
        assert_eq!(at(Side::In, Param::EqMid, 0.0), Some(-6.0));
        assert_eq!(at(Side::In, Param::EqMid, s / 2.0), Some(-3.0));
        assert_eq!(at(Side::In, Param::EqHigh, s / 2.0), Some(0.0));
        // A: Bass bis einen Beat vor S voll, an S aus; Fader im letzten Viertel zu.
        assert_eq!(at(Side::Out, Param::EqLow, s - 2.0), None, "vor dem ersten Stützpunkt greift die Spur nicht ein");
        assert_eq!(at(Side::Out, Param::EqLow, s - 1.0), Some(0.0));
        assert_eq!(at(Side::Out, Param::EqLow, s), Some(f32::NEG_INFINITY));
        assert_eq!(at(Side::Out, Param::EqMid, l - l / 8.0), Some(-9.0));
        assert_eq!(at(Side::Out, Param::EqHigh, l - l / 8.0), Some(-12.0));
        assert_eq!(at(Side::Out, Param::Fader, l - l / 4.0), Some(1.0));
        assert_eq!(at(Side::Out, Param::Fader, l), Some(0.0));
        // Bass-Tausch: In der Mitte des Tausch-Beats stehen beide Bässe auf halbem Pegel (−6 dB).
        let half = at(Side::Out, Param::EqLow, s - 0.5).unwrap();
        assert!((half + 6.0206).abs() < 1e-3, "{half}");
        assert!((at(Side::In, Param::EqLow, s - 0.5).unwrap() + 6.0206).abs() < 1e-3);
        assert_eq!((t.first_beat(), t.last_beat()), (-1.0, l + 1.0));
    }

    /// Die Befehle kommen in zeitlicher Reihenfolge, in Schritten von einem Viertel-Beat, auf den
    /// richtigen Decks, mit Start und Stopp und dem Zurückstellen danach.
    #[test]
    fn commands_are_ordered_stepped_and_addressed() {
        let t0 = 100.0;
        let cmds = Transition::bass_swap(16).commands(t0, ROLES);
        assert!(cmds.windows(2).all(|w| w[0].0 <= w[1].0), "nicht sortiert");
        let (l, s) = (64.0, 32.0);
        // Fader von B: 0 vor T0, dann in 4 Schritten je Beat bis 1 bei L/8.
        let fader_b = values(&cmds, |c| match c {
            Command::Fader { deck: B, value } => Some(*value),
            _ => None,
        });
        assert_eq!(fader_b.first(), Some(&(t0 - 1.0, 0.0)));
        assert_eq!(fader_b.last(), Some(&(t0 + l / 8.0, 1.0)));
        assert_eq!(fader_b.len(), 1 + (l / 8.0) as usize * STEPS_PER_BEAT as usize);
        assert!(fader_b.windows(2).all(|w| w[1].1 > w[0].1), "Fader steigt nicht stetig");
        // Start von B an T0, Stopp von A an T0 + L; davor liegt am selben Beat der letzte Reglerwert.
        let play: Vec<_> = cmds.iter().filter(|(_, c)| matches!(c, Command::Play { .. })).collect();
        assert_eq!(play.len(), 2);
        assert_eq!(*play[0], (t0, Command::Play { deck: B, on: true }));
        assert_eq!(*play[1], (t0 + l, Command::Play { deck: A, on: false }));
        // Bass von A: genau im letzten Beat vor S, endet im Kill.
        let low_a = values(&cmds, |c| match c {
            Command::Eq { deck: A, band: Band::Low, db } => Some(*db),
            _ => None,
        });
        assert_eq!(low_a.first(), Some(&(t0 + s - 1.0, 0.0)));
        assert_eq!(low_a[4], (t0 + s, f32::NEG_INFINITY));
        // Danach stehen die Regler von A wieder in Ruhestellung.
        assert_eq!(low_a.last(), Some(&(t0 + l + 1.0, 0.0)));
        let fader_a = values(&cmds, |c| match c {
            Command::Fader { deck: A, value } => Some(*value),
            _ => None,
        });
        assert_eq!(fader_a.last(), Some(&(t0 + l + 1.0, 1.0)));
        // Mit vertauschten Rollen fährt dieselbe Vorlage die anderen Decks.
        let swapped = Transition::bass_swap(16).commands(t0, Roles { out: B, inn: A });
        assert!(swapped.contains(&(t0, Command::Play { deck: A, on: true })));
    }

    /// Kurvenformen: linear, weich, Stufe.
    #[test]
    fn curve_shapes() {
        let lane = |curve| Lane { side: Side::In, param: Param::Fader, points: vec![Point::new(0.0, 0.0), Point { beat: 8.0, value: 1.0, curve }] };
        assert_eq!(lane(Curve::Linear).value_at(2.0), Some(0.25));
        let smooth = lane(Curve::Smooth);
        assert!((smooth.value_at(4.0).unwrap() - 0.5).abs() < 1e-6);
        assert!(smooth.value_at(2.0).unwrap() < 0.25 && smooth.value_at(6.0).unwrap() > 0.75);
        let step = lane(Curve::Step);
        assert_eq!((step.value_at(7.99), step.value_at(8.0)), (Some(0.0), Some(1.0)));
        assert_eq!(step.steps(), vec![(0.0, 0.0), (8.0, 1.0)]);
    }

    /// Ein Übergang übersteht Schreiben und Lesen als JSON (Kill = null) und wird auf Unsinn geprüft.
    #[test]
    fn json_roundtrip_and_validation() {
        for t in Transition::templates(16) {
            t.validate().unwrap();
            let json = serde_json::to_string(&t).unwrap();
            assert!(!json.contains("inf"), "{json}");
            assert_eq!(serde_json::from_str::<Transition>(&json).unwrap(), t, "{}", t.name);
        }
        let json = r#"{"name":"eigen","bars":8,"start_in":0,"stop_out":null,"lanes":[{"side":"in","param":"eq_low","points":[{"beat":0,"value":null},{"beat":16,"value":0,"curve":"smooth"}]}]}"#;
        let t: Transition = serde_json::from_str(json).unwrap();
        t.validate().unwrap();
        assert_eq!(t.lanes[0].points[0].value, f32::NEG_INFINITY);
        assert_eq!((t.stop_out, t.reset_out, t.lanes[0].points[1].curve), (None, false, Curve::Smooth));

        let mut bad = Transition::fade(16);
        bad.lanes[0].points[1].value = 1.5;
        assert!(bad.validate().is_err());
        let mut unsorted = Transition::fade(16);
        unsorted.lanes[0].points.swap(1, 2);
        assert!(unsorted.validate().is_err());
        let mut twice = Transition::fade(16);
        twice.lanes.push(twice.lanes[0].clone());
        assert!(twice.validate().is_err());
    }
}
