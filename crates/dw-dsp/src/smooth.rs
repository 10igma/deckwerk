//! Lineare Glättung von Reglerwerten (MIX-13): Ein Sprung wird über eine feste Zeit verteilt.

#[derive(Clone, Debug)]
pub struct Smoother {
    current: f32,
    target: f32,
    step: f32,
    remaining: u32,
    ramp_len: u32,
}

impl Smoother {
    pub fn new(value: f32, sample_rate: f64, ms: f64) -> Self {
        let ramp_len = ((sample_rate * ms / 1000.0).round() as u32).max(1);
        Smoother { current: value, target: value, step: 0.0, remaining: 0, ramp_len }
    }

    pub fn set(&mut self, target: f32) {
        if target != self.target {
            self.target = target;
            self.remaining = self.ramp_len;
            self.step = (target - self.current) / self.ramp_len as f32;
        }
    }

    pub fn set_immediate(&mut self, value: f32) {
        self.current = value;
        self.target = value;
        self.remaining = 0;
    }

    #[inline]
    pub fn next(&mut self) -> f32 {
        if self.remaining > 0 {
            self.remaining -= 1;
            if self.remaining == 0 {
                self.current = self.target;
            } else {
                self.current += self.step;
            }
        }
        self.current
    }

    pub fn value(&self) -> f32 {
        self.current
    }

    pub fn target(&self) -> f32 {
        self.target
    }

    pub fn is_settled(&self) -> bool {
        self.remaining == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ramps_linearly_and_lands_exactly() {
        let mut s = Smoother::new(0.0, 1000.0, 10.0); // 10 Samples
        s.set(1.0);
        let v: Vec<f32> = (0..12).map(|_| s.next()).collect();
        assert!((v[0] - 0.1).abs() < 1e-6);
        assert!((v[4] - 0.5).abs() < 1e-6);
        assert_eq!(v[9], 1.0);
        assert_eq!(v[11], 1.0);
        assert!(s.is_settled());
    }
}
