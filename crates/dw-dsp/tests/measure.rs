//! Messtests der Signalverarbeitung gegen die Abnahmekriterien MIX-02, MIX-04, MIX-08 und 5.13.

use dw_dsp::{sample_stereo, FilterKnob, Isolator, Limiter};
use rustfft::{num_complex::Complex, FftPlanner};
use std::f64::consts::PI;

const SR: f64 = 48_000.0;
const N: usize = 1 << 16;

/// Betragsfrequenzgang in dB aus einer Impulsantwort (linker Kanal).
fn response_db(process: &mut dyn FnMut(&mut [f32])) -> Vec<f64> {
    let mut buf = vec![0.0f32; N * 2];
    buf[0] = 1.0;
    buf[1] = 1.0;
    process(&mut buf);
    let mut spec: Vec<Complex<f64>> = buf.chunks_exact(2).map(|f| Complex::new(f[0] as f64, 0.0)).collect();
    FftPlanner::new().plan_fft_forward(N).process(&mut spec);
    spec[..N / 2].iter().map(|c| 20.0 * c.norm().max(1e-12).log10()).collect()
}

fn at(db: &[f64], hz: f64) -> f64 {
    db[(hz * N as f64 / SR).round() as usize]
}

/// Lässt Glättungen einschwingen, bevor gemessen wird.
fn settle(process: &mut dyn FnMut(&mut [f32])) {
    let mut warm = vec![0.0f32; 4800];
    process(&mut warm);
}

#[test]
fn isolator_is_flat_in_centre_position() {
    let mut eq = Isolator::new(SR);
    let db = response_db(&mut |b| eq.process(b));
    let (lo, hi) = ((20.0 * N as f64 / SR) as usize, (20_000.0 * N as f64 / SR) as usize);
    let worst = db[lo..hi].iter().fold(0.0f64, |m, v| m.max(v.abs()));
    assert!(worst <= 0.1, "Abweichung in Mittelstellung {worst:.4} dB");
}

#[test]
fn isolator_kill_attenuates_each_band() {
    // Erwartet: 24 dB/Oktave, also ≥ 24 dB eine und ≥ 48 dB zwei Oktaven innerhalb des Bands.
    let mut eq = Isolator::new(SR);
    eq.set_gain(0, 0.0);
    settle(&mut |b| eq.process(b));
    let db = response_db(&mut |b| eq.process(b));
    assert!(at(&db, 100.0) <= -24.0, "Tief-Kill bei 100 Hz: {:.1} dB", at(&db, 100.0));
    assert!(at(&db, 50.0) <= -48.0, "Tief-Kill bei 50 Hz: {:.1} dB", at(&db, 50.0));
    assert!(at(&db, 1000.0).abs() <= 0.5, "Mitten bleiben: {:.2} dB", at(&db, 1000.0));

    let mut eq = Isolator::new(SR);
    eq.set_gain(2, 0.0);
    settle(&mut |b| eq.process(b));
    let db = response_db(&mut |b| eq.process(b));
    assert!(at(&db, 5000.0) <= -24.0, "Hoch-Kill bei 5 kHz: {:.1} dB", at(&db, 5000.0));
    assert!(at(&db, 10_000.0) <= -48.0, "Hoch-Kill bei 10 kHz: {:.1} dB", at(&db, 10_000.0));
    assert!(at(&db, 700.0).abs() <= 0.5);

    let mut eq = Isolator::new(SR);
    eq.set_gain(1, 0.0);
    settle(&mut |b| eq.process(b));
    let db = response_db(&mut |b| eq.process(b));
    assert!(at(&db, 707.0) <= -20.0, "Mitten-Kill bei 707 Hz: {:.1} dB", at(&db, 707.0));
    assert!(at(&db, 60.0).abs() <= 0.5 && at(&db, 10_000.0).abs() <= 0.5);
}

#[test]
fn isolator_boost_is_six_db() {
    let mut eq = Isolator::new(SR);
    eq.set_gain(0, 2.0);
    settle(&mut |b| eq.process(b));
    let db = response_db(&mut |b| eq.process(b));
    assert!((at(&db, 50.0) - 6.02).abs() <= 0.2);
}

#[test]
fn filter_centre_is_bit_exact() {
    let mut f = FilterKnob::new(SR);
    let src: Vec<f32> = (0..9600).map(|i| ((i * 7919) % 2001) as f32 / 1000.0 - 1.0).collect();
    let mut buf = src.clone();
    f.process(&mut buf);
    assert_eq!(buf, src);
    // Auch nach einem Ausflug zurück in die Mitte wieder bitgenau
    f.set_pos(-0.6);
    f.process(&mut vec![0.1; 9600]);
    f.set_pos(0.0);
    f.process(&mut vec![0.1; 9600]);
    let mut buf = src.clone();
    f.process(&mut buf);
    assert_eq!(buf, src);
}

#[test]
fn filter_follows_the_specified_curve() {
    assert!((dw_dsp::filter::cutoff_hz(-1.0) - 80.0).abs() < 1e-6);
    assert!((dw_dsp::filter::cutoff_hz(1.0) - 8000.0).abs() < 1e-6);
    assert!((dw_dsp::filter::cutoff_hz(0.38) - 195.0).abs() < 2.0);

    // Tiefpass bei −0,5: Grenzfrequenz ≈ 1 265 Hz, 12 dB/Oktave
    let mut f = FilterKnob::new(SR);
    f.set_pos(-0.5);
    settle(&mut |b| f.process(b));
    let db = response_db(&mut |b| f.process(b));
    let fc = dw_dsp::filter::cutoff_hz(-0.5);
    assert!(at(&db, 100.0).abs() <= 0.3);
    assert!((at(&db, fc) - 20.0 * 1.2f64.log10()).abs() <= 0.5, "Pegel an der Grenzfrequenz {:.2}", at(&db, fc));
    assert!((at(&db, fc * 4.0) + 24.0).abs() <= 2.0, "zwei Oktaven darüber {:.2}", at(&db, fc * 4.0));

    // Hochpass bei +0,5: Grenzfrequenz 400 Hz
    let mut f = FilterKnob::new(SR);
    f.set_pos(0.5);
    settle(&mut |b| f.process(b));
    let db = response_db(&mut |b| f.process(b));
    assert!(at(&db, 8000.0).abs() <= 0.3);
    assert!((at(&db, 100.0) + 24.0).abs() <= 2.0, "zwei Oktaven darunter {:.2}", at(&db, 100.0));
}

#[test]
fn limiter_holds_the_ceiling() {
    let ceiling_db = -0.3f32;
    let ceiling = 10f32.powf(ceiling_db / 20.0);
    let mut lim = Limiter::new(SR, ceiling_db);
    // Sinus mit +12 dB über Vollaussteuerung, dazu harte Sprünge
    let mut buf: Vec<f32> = (0..48_000)
        .flat_map(|i| {
            let s = 3.98 * (2.0 * PI * 60.0 * i as f64 / SR).sin() as f32;
            let s = if i % 5000 < 3 { 6.0 } else { s };
            [s, -s * 0.7]
        })
        .collect();
    lim.process(&mut buf);
    let peak = buf.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(peak <= ceiling + 1e-6, "Spitze {peak} über der Decke {ceiling}");
    assert!(lim.take_min_gain() < 0.3);
}

#[test]
fn limiter_reduces_gain_instead_of_clipping() {
    // Die Decke wird über Pegelreduktion gehalten: Ein übersteuerter Sinus bleibt ein Sinus.
    let mut lim = Limiter::new(SR, -0.3);
    let mut buf: Vec<f32> = (0..96_000)
        .flat_map(|i| {
            let s = 2.0 * (2.0 * PI * 1000.0 * i as f64 / SR).sin() as f32;
            [s, s]
        })
        .collect();
    lim.process(&mut buf);
    // Klirranteil im eingeschwungenen Teil messen
    let seg: Vec<f64> = buf[2 * 48_000..2 * (48_000 + 32_768)].chunks_exact(2).map(|f| f[0] as f64).collect();
    let thd = residual_db(&seg, 1000.0);
    assert!(thd <= -50.0, "Verzerrung {thd:.1} dB");
}

#[test]
fn limiter_passes_quiet_signal_unchanged_with_delay() {
    let mut lim = Limiter::new(SR, -0.3);
    let look = lim.lookahead();
    let src: Vec<f32> = (0..4000).flat_map(|i| { let s = 0.5 * (i as f32 * 0.05).sin(); [s, s] }).collect();
    let mut buf = src.clone();
    lim.process(&mut buf);
    for i in look..2000 {
        assert!((buf[i * 2] - src[(i - look) * 2]).abs() < 1e-6);
    }
}

/// Pegel von allem außer dem Sinus bei `hz`, relativ zum Sinus, in dB.
fn residual_db(x: &[f64], hz: f64) -> f64 {
    let n = x.len() as f64;
    let (mut a, mut b) = (0.0, 0.0);
    for (i, &v) in x.iter().enumerate() {
        let ph = 2.0 * PI * hz * i as f64 / SR;
        a += v * ph.sin();
        b += v * ph.cos();
    }
    let (a, b) = (2.0 * a / n, 2.0 * b / n);
    let mut sig = 0.0;
    let mut err = 0.0;
    for (i, &v) in x.iter().enumerate() {
        let ph = 2.0 * PI * hz * i as f64 / SR;
        let fit = a * ph.sin() + b * ph.cos();
        sig += fit * fit;
        err += (v - fit) * (v - fit);
    }
    10.0 * (err / sig).log10()
}

fn resample_sine(hz: f64, rate: f64, out_frames: usize) -> Vec<f64> {
    let src: Vec<f32> = (0..(out_frames as f64 * rate) as usize + 200)
        .flat_map(|i| { let s = 0.5 * (2.0 * PI * hz * i as f64 / SR).sin() as f32; [s, s] })
        .collect();
    (0..out_frames).map(|i| sample_stereo(&src, 64.0 + i as f64 * rate).0 as f64).collect()
}

#[test]
fn interpolation_is_clean_and_keeps_level() {
    for rate in [0.94, 1.0 + 1.0 / 3.0e2, 1.06, 1.08] {
        // 1-kHz-Sinus, um `rate` schneller gelesen: Ausgabe ist ein Sinus mit 1 kHz × rate
        let out = resample_sine(1000.0, rate, 32_768);
        let res = residual_db(&out, 1000.0 * rate);
        assert!(res <= -70.0, "Rate {rate}: Störabstand nur {res:.1} dB");
    }
    // Pegel im Hörbereich bis 15 kHz innerhalb 0,5 dB
    for hz in [100.0, 5000.0, 15_000.0] {
        let out = resample_sine(hz, 1.03, 32_768);
        let rms = (out.iter().map(|v| v * v).sum::<f64>() / out.len() as f64).sqrt();
        let db = 20.0 * (rms / (0.5 / 2f64.sqrt())).log10();
        assert!(db.abs() <= 0.5, "{hz} Hz: {db:.2} dB");
    }
}

#[test]
fn interpolation_at_integer_positions_is_bit_exact() {
    let src: Vec<f32> = (0..2000).map(|i| (i as f32 * 0.37).sin()).collect();
    for i in [0usize, 1, 17, 500, 999] {
        assert_eq!(sample_stereo(&src, i as f64), (src[i * 2], src[i * 2 + 1]));
    }
    assert_eq!(sample_stereo(&src, -40.0), (0.0, 0.0));
    assert_eq!(sample_stereo(&src, 5000.5), (0.0, 0.0));
}

/// MIX-08: Die Decke gilt als True Peak, also auch zwischen zwei Samples.
#[test]
fn limiter_holds_the_ceiling_as_true_peak() {
    use dw_dsp::resample::resample_stereo;
    // 12 kHz mit 45° Phasenlage: Die Samples erreichen nur 71 % der wahren Spitze.
    let amp = 1.3f64;
    let src: Vec<f32> = (0..48_000)
        .flat_map(|i| {
            let s = (amp * (2.0 * PI * 12_000.0 * i as f64 / SR + PI / 4.0).sin()) as f32;
            [s, s]
        })
        .collect();
    let sample_peak = src.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    assert!(sample_peak < 0.93, "Samples allein liegen unter der Decke: {sample_peak}");

    let true_peak = |x: &[f32]| {
        let up = resample_stereo(x, 48_000, 384_000);
        up[20_000..up.len() - 20_000].iter().fold(0.0f32, |m, s| m.max(s.abs()))
    };
    assert!(true_peak(&src) > 1.25, "das Testsignal hat eine wahre Spitze von 1,3");

    let mut lim = Limiter::new(SR, -0.3);
    let mut out = src.clone();
    lim.process(&mut out);
    let tp_db = 20.0 * true_peak(&out).log10();
    println!("True Peak nach dem Limiter: {tp_db:.2} dBTP");
    assert!(tp_db <= -0.3 + 0.1, "True Peak {tp_db:.2} dBTP über der Decke");
    assert!(tp_db >= -1.5, "Limiter nimmt zu viel weg: {tp_db:.2} dBTP");
}
