//! Audio-Puffer: Stereo, 32-bit-Gleitkomma, verschachtelt (L R L R …).

/// Vollständig dekodierter Track oder Render-Ergebnis.
#[derive(Clone, Debug, PartialEq)]
pub struct AudioBuf {
    pub sample_rate: u32,
    /// Verschachtelte Stereo-Samples, Länge = 2 × Frames.
    pub data: Vec<f32>,
}

impl AudioBuf {
    pub fn new(sample_rate: u32, data: Vec<f32>) -> Self {
        assert!(data.len() % 2 == 0, "Stereo-Puffer braucht eine gerade Sample-Zahl");
        AudioBuf { sample_rate, data }
    }

    pub fn silence(sample_rate: u32, frames: usize) -> Self {
        AudioBuf { sample_rate, data: vec![0.0; frames * 2] }
    }

    pub fn from_mono(sample_rate: u32, mono: &[f32]) -> Self {
        let mut data = Vec::with_capacity(mono.len() * 2);
        for &s in mono {
            data.push(s);
            data.push(s);
        }
        AudioBuf { sample_rate, data }
    }

    pub fn frames(&self) -> usize {
        self.data.len() / 2
    }

    pub fn duration(&self) -> f64 {
        self.frames() as f64 / self.sample_rate as f64
    }

    /// Mono-Mischung (Mittelwert beider Kanäle).
    pub fn to_mono(&self) -> Vec<f32> {
        self.data.chunks_exact(2).map(|f| 0.5 * (f[0] + f[1])).collect()
    }

    pub fn peak(&self) -> f32 {
        self.data.iter().fold(0.0f32, |m, s| m.max(s.abs()))
    }
}
