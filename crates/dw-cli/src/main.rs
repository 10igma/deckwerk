//! Kommandozeile von Deckwerk: Analyse und Rendern ohne Oberfläche (Spezifikation 7.2).

use anyhow::{anyhow, bail, Context, Result};
use dw_analysis::{analyze, AnalysisConfig};
use dw_automix::{plan_blend, render_blend};
use dw_core::{harmonic_compat, AudioBuf, Key, TrackAnalysis};
use dw_decode::{decode_file, resample, write_wav};
use dw_engine::Track;
use dw_testgen::{techno_track, SectionKind, SectionSpec, TrackSpec};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

const ENGINE_RATE: u32 = 48_000;
/// Referenz-Lautheit für den Auto-Gain (MIX-01).
const REFERENCE_LUFS: f32 = -14.0;

const HELP: &str = "Deckwerk – Kommandozeile

  deckwerk analyze [--json] <Datei>...
      Tempo, Beatgrid, Tonart und Lautheit ermitteln.

  deckwerk mix <A> <B> -o <Ausgabe.wav> [--bars 16|32|64] [--lead N] [--tail N]
      Übergang von A nach B rendern: langer Blend mit Bass-Tausch.
      --bars  Länge des Übergangs in Takten (Standard 32)
      --lead  Takte von A vor dem Übergang (Standard 16)
      --tail  Takte von B nach dem Übergang (Standard 16)

  deckwerk testtrack <Ausgabe.wav> [--bpm 132] [--key 8A] [--seed 1]
      Synthetischen Techno-Track mit bekanntem Tempo und bekannter Tonart erzeugen.
";

fn main() {
    if let Err(e) = run() {
        eprintln!("Fehler: {e:#}");
        std::process::exit(1);
    }
}

/// Holt den Wert einer Option `--name wert` aus der Argumentliste.
fn option(args: &mut Vec<String>, name: &str) -> Result<Option<String>> {
    match args.iter().position(|a| a == name) {
        Some(i) if i + 1 < args.len() => {
            let value = args.remove(i + 1);
            args.remove(i);
            Ok(Some(value))
        }
        Some(_) => bail!("Option {name} braucht einen Wert"),
        None => Ok(None),
    }
}

fn flag(args: &mut Vec<String>, name: &str) -> bool {
    match args.iter().position(|a| a == name) {
        Some(i) => {
            args.remove(i);
            true
        }
        None => false,
    }
}

fn number<T: std::str::FromStr>(value: Option<String>, default: T, name: &str) -> Result<T> {
    match value {
        Some(v) => v.parse().map_err(|_| anyhow!("{name}: „{v}“ ist keine gültige Zahl")),
        None => Ok(default),
    }
}

fn load(path: &Path) -> Result<(AudioBuf, TrackAnalysis)> {
    let decoded = decode_file(path).with_context(|| format!("{}", path.display()))?;
    let audio = resample(&decoded.audio, ENGINE_RATE);
    let analysis = analyze(&audio, &AnalysisConfig::default());
    Ok((audio, analysis))
}

fn key_text(a: &TrackAnalysis) -> String {
    match a.key {
        Some(k) if a.key_confidence >= 0.3 => format!("{} ({})", k.camelot_string(), k.name_de()),
        Some(k) => format!("{} ? ({}, unsicher)", k.camelot_string(), k.name_de()),
        None => "–".into(),
    }
}

fn run() -> Result<()> {
    let mut args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() || flag(&mut args, "--help") || flag(&mut args, "-h") {
        print!("{HELP}");
        return Ok(());
    }
    match args.remove(0).as_str() {
        "analyze" => cmd_analyze(args),
        "mix" => cmd_mix(args),
        "testtrack" => cmd_testtrack(args),
        other => bail!("unbekannter Befehl „{other}“ (Hilfe: deckwerk --help)"),
    }
}

fn cmd_analyze(mut args: Vec<String>) -> Result<()> {
    let json = flag(&mut args, "--json");
    if args.is_empty() {
        bail!("keine Datei angegeben");
    }
    let mut results = Vec::new();
    for file in &args {
        let started = Instant::now();
        match load(Path::new(file)) {
            Ok((_, a)) => {
                if !json {
                    let check = if a.bpm_confidence < 0.5 { "  (Tempo prüfen)" } else { "" };
                    println!(
                        "{file}\n  Tempo {:.2} BPM{check} · Tonart {} · Lautheit {:.1} LUFS · Spitze {:.1} dBTP · Länge {}:{:02} · {:.1} s Rechenzeit",
                        a.bpm,
                        key_text(&a),
                        a.lufs,
                        a.true_peak_db,
                        a.duration as u32 / 60,
                        a.duration as u32 % 60,
                        started.elapsed().as_secs_f64()
                    );
                }
                results.push(serde_json::json!({ "file": file, "analysis": a }));
            }
            Err(e) => {
                eprintln!("{e:#}");
                results.push(serde_json::json!({ "file": file, "error": format!("{e:#}") }));
            }
        }
    }
    if json {
        println!("{}", serde_json::to_string_pretty(&results)?);
    }
    Ok(())
}

fn cmd_mix(mut args: Vec<String>) -> Result<()> {
    let out = option(&mut args, "-o")?.ok_or_else(|| anyhow!("Ausgabedatei fehlt (-o Ausgabe.wav)"))?;
    let bars: u32 = number(option(&mut args, "--bars")?, 32, "--bars")?;
    let lead: u32 = number(option(&mut args, "--lead")?, 16, "--lead")?;
    let tail: u32 = number(option(&mut args, "--tail")?, 16, "--tail")?;
    let [a_path, b_path] = args.as_slice() else { bail!("mix braucht genau zwei Dateien") };

    let (a_audio, a) = load(Path::new(a_path))?;
    let (b_audio, b) = load(Path::new(b_path))?;
    println!("A: {:.2} BPM, {}", a.bpm, key_text(&a));
    println!("B: {:.2} BPM, {}", b.bpm, key_text(&b));
    if let (Some(ka), Some(kb)) = (a.key, b.key) {
        let k = harmonic_compat(ka, kb);
        let word = if k >= 0.9 { "passt" } else if k >= 0.5 { "geht" } else { "reibt" };
        println!("Tonarten: {} → {}: {word} (K = {k:.1})", ka.camelot_string(), kb.camelot_string());
    }

    let plan = plan_blend(&a, &b, bars, lead, tail)?;
    println!(
        "Übergang: Blend mit Bass-Tausch über {} Takte, Tausch nach {} Takten; A ab {:.1} s, Übergang ab {:.1} s, B ab {:.1} s",
        plan.bars,
        plan.swap_bar(),
        plan.a_start,
        plan.a_out,
        plan.b_in
    );
    if plan.tempo_ramp.0 != plan.tempo_ramp.1 {
        println!(
            "Tempo: {:.2} → {:.2} BPM ({:.1} %), angeglichen über {:.0} Takte. Ohne Key Lock ändert sich dabei die Tonhöhe.",
            plan.bpm_a,
            plan.bpm_b,
            plan.tempo_gap_percent(),
            plan.tempo_ramp.1 - plan.tempo_ramp.0
        );
    }

    let track = |audio, an: &TrackAnalysis| Arc::new(Track { audio, grid: an.grid.clone(), gain_db: an.auto_gain_db(REFERENCE_LUFS) });
    let started = Instant::now();
    let (mix, report) = render_blend(track(a_audio, &a), track(b_audio, &b), &plan);
    let secs = |frame: usize| frame as f64 / mix.sample_rate as f64;
    println!(
        "Gerendert: {:.1} s in {:.1} s · Übergang von {:.1} s bis {:.1} s, Bass-Tausch bei {:.1} s · größter Beat-Versatz {:.4} Beats · Limiter bis {:.1} dB",
        mix.duration(),
        started.elapsed().as_secs_f64(),
        secs(report.t0_frame),
        secs(report.end_frame),
        secs(report.swap_frame),
        report.max_phase_error,
        report.max_limiter_reduction_db
    );
    write_wav(&PathBuf::from(&out), &mix, 24).with_context(|| out.clone())?;
    println!("Geschrieben: {out}");
    Ok(())
}

fn cmd_testtrack(mut args: Vec<String>) -> Result<()> {
    let bpm: f64 = number(option(&mut args, "--bpm")?, 132.0, "--bpm")?;
    let seed: u64 = number(option(&mut args, "--seed")?, 1, "--seed")?;
    let key_arg = option(&mut args, "--key")?.unwrap_or_else(|| "8A".into());
    let (digits, letter) = key_arg.split_at(key_arg.len().saturating_sub(1));
    let key = digits
        .parse::<u8>()
        .ok()
        .zip(letter.chars().next())
        .and_then(|(n, l)| Key::from_camelot(n, l))
        .ok_or_else(|| anyhow!("--key: „{key_arg}“ ist kein Camelot-Code (z. B. 8A oder 11B)"))?;
    let [out] = args.as_slice() else { bail!("testtrack braucht genau eine Ausgabedatei") };

    use SectionKind::*;
    let sections = [(Intro, 16), (Main, 32), (Breakdown, 8), (Buildup, 8), (Drop, 32), (Outro, 32)]
        .into_iter()
        .map(|(kind, bars)| SectionSpec::new(kind, bars))
        .collect();
    let mut spec = TrackSpec::new(bpm, key, sections, seed);
    spec.sample_rate = ENGINE_RATE;
    let (audio, truth) = techno_track(&spec);
    write_wav(Path::new(out), &audio, 24).with_context(|| out.clone())?;
    println!(
        "Geschrieben: {out} · {:.2} BPM · {} ({}) · {} Takte · {:.0} s",
        truth.bpm,
        key.camelot_string(),
        key.name_de(),
        spec.total_bars(),
        audio.duration()
    );
    Ok(())
}
