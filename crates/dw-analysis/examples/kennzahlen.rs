//! Gibt die Kennzahlen der Analyse auf dem synthetischen Referenz-Set als Tabelle aus
//! und misst die Analysezeit für einen 7-Minuten-Track (48 kHz).
//!
//! `cargo run --release -p dw-analysis --example kennzahlen`

#[path = "../tests/common/mod.rs"]
mod common;

use dw_analysis::{analyze, waveform, AnalysisConfig};
use dw_testgen::{sine, techno_track};
use std::time::Instant;

fn main() {
    let started = Instant::now();
    let results = common::evaluate_reference_set();
    println!("{}", common::format_report(&results));
    println!("Referenz-Set erzeugt und analysiert in {:.1} s\n", started.elapsed().as_secs_f64());

    // Lautheit: 1-kHz-Sinus, stereo, −23 dBFS.
    let tone = sine(48_000, 1000.0, 20.0, 10f32.powf(-23.0 / 20.0));
    let loudness = analyze(&tone, &AnalysisConfig::default());
    println!("Lautheit 1 kHz / −23 dBFS: {:.2} LUFS, True Peak {:.2} dBTP (Ziel −23,0 ±0,1)\n", loudness.lufs, loudness.true_peak_db);

    // Geschwindigkeit: 7 Minuten bei 48 kHz.
    let (audio, _) = techno_track(&common::seven_minute_spec());
    println!("Geschwindigkeit: Track mit {:.0} s bei {} Hz, {} Threads", audio.duration(), audio.sample_rate, rayon::current_num_threads());
    for run in 1..=3 {
        let t0 = Instant::now();
        let analysis = analyze(&audio, &AnalysisConfig::default());
        let t_analyze = t0.elapsed().as_secs_f64();
        let t1 = Instant::now();
        let wave = waveform(&audio);
        let t_wave = t1.elapsed().as_secs_f64();
        println!(
            "  Lauf {run}: analyze {:.2} s + waveform {:.2} s = {:.2} s (Ziel ≤ 15 s); {:.2} BPM, {} Punkte in der feinsten Stufe",
            t_analyze,
            t_wave,
            t_analyze + t_wave,
            analysis.bpm,
            wave.levels[0].points()
        );
    }
}
