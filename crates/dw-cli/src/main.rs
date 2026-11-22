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

  deckwerk transition <A> <B> -o <Ausgabe.wav> --a-pos <s> --end-in <Beats> --b-mark <s>
                      [--template bass_swap|fade] [--bars 32] [--lead 16] [--tail 16]
      Übergang rendern, wie die Anwendung ihn live fährt: A läuft und steht bei --a-pos; der
      Übergang endet --end-in Beats später (auf den nächsten Taktanfang gerückt), und B hat
      dann die Stelle --b-mark erreicht. --lead/--tail: Takte von A davor und von B danach.

  deckwerk check <Referenz.csv> <Musikordner> [--tol 0.05]
      Analyse gegen Referenzwerte prüfen (ANA-02, ANA-06): analysiert jede Datei der Referenz
      neu und meldet die Trefferquote bei Tempo und Tonart.
      Referenz: Zeilen „Datei;BPM;Tonart;Quelle“, Tonart in Camelot (z. B. 9A). Ein „?“ hinter
      einem Wert heißt: nur eine Quelle; solche Werte werden getrennt ausgewiesen. Leere Felder
      werden nicht gewertet, Zeilen mit # sind Kommentare.
      --tol  erlaubte Tempo-Abweichung in BPM (Standard 0,05)

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
        "check" => cmd_check(args),
        "transition" => cmd_transition(args),
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

/// Eine Zeile der Referenzdatei: Dateiname und, soweit bekannt, Tempo und Tonart.
#[derive(Debug, PartialEq)]
struct Reference {
    file: String,
    bpm: Option<f64>,
    /// Tempo durch zwei unabhängige Quellen belegt (kein „?“ in der Datei).
    bpm_sure: bool,
    key: Option<Key>,
    key_sure: bool,
}

/// Liest die Referenz: „Datei;BPM;Tonart;Quelle". Ein „?" hinter einem Wert kennzeichnet ihn als
/// nur einfach belegt. Leere Felder bleiben ungewertet; Zeilen mit `#` und die Kopfzeile werden
/// übergangen.
fn parse_reference(text: &str) -> Result<Vec<Reference>> {
    let mut rows = Vec::new();
    for (nr, line) in text.lines().enumerate() {
        let line = line.trim().trim_start_matches('\u{feff}');
        if line.is_empty() || line.starts_with('#') || line.to_lowercase().starts_with("datei;") {
            continue;
        }
        let fields: Vec<&str> = line.split(';').map(str::trim).collect();
        let file = fields[0].to_string();
        // „?" am Ende: Wert aus nur einer Quelle.
        let field = |i: usize| {
            let raw = fields.get(i).copied().unwrap_or("");
            (raw.trim_end_matches('?').trim(), !raw.ends_with('?'))
        };
        let ((bpm_text, bpm_sure), (key_text, key_sure)) = (field(1), field(2));
        let bpm = match bpm_text {
            "" => None,
            v => Some(v.replace(',', ".").parse::<f64>().map_err(|_| anyhow!("Zeile {}: „{v}“ ist kein Tempo", nr + 1))?),
        };
        let key = match key_text {
            "" => None,
            v => Some(parse_camelot(v).ok_or_else(|| anyhow!("Zeile {}: „{v}“ ist keine Camelot-Tonart", nr + 1))?),
        };
        rows.push(Reference { file, bpm, bpm_sure, key, key_sure });
    }
    Ok(rows)
}

fn parse_camelot(text: &str) -> Option<Key> {
    let letter = text.chars().last()?;
    let number: u8 = text[..text.len() - letter.len_utf8()].parse().ok()?;
    Key::from_camelot(number, letter)
}

/// Einordnung eines gemessenen Tempos gegen die Referenz.
#[derive(Debug, PartialEq, Clone, Copy)]
enum TempoVerdict {
    Match,
    /// Doppeltes oder halbes Tempo (ANA-02 zählt diese Fehler eigens).
    Octave,
    Off,
}

fn tempo_verdict(found: f64, expected: f64, tol: f64) -> TempoVerdict {
    if (found - expected).abs() <= tol {
        TempoVerdict::Match
    } else if (found - expected * 2.0).abs() <= tol * 2.0 || (found - expected / 2.0).abs() <= tol / 2.0 {
        TempoVerdict::Octave
    } else {
        TempoVerdict::Off
    }
}

/// Gewichtete Wertung einer Tonart nach Spezifikation 5.3: exakt 1,0; Quinte daneben 0,5;
/// Paralleltonart 0,3; gleichnamige Tonart (Dur/Moll) 0,2; sonst 0.
fn key_score(found: Key, expected: Key) -> f64 {
    let (found_nr, found_letter) = found.camelot();
    let (expected_nr, expected_letter) = expected.camelot();
    let step = (found_nr as i32 - expected_nr as i32).rem_euclid(12);
    if found == expected {
        1.0
    } else if found_letter == expected_letter && (step == 1 || step == 11) {
        0.5
    } else if found_nr == expected_nr {
        0.3
    } else if found.pitch_class() == expected.pitch_class() {
        0.2
    } else {
        0.0
    }
}

fn cmd_check(mut args: Vec<String>) -> Result<()> {
    let tol: f64 = number(option(&mut args, "--tol")?, 0.05, "--tol")?;
    let [reference, folder] = args.as_slice() else { bail!("erwartet: deckwerk check <Referenz.csv> <Musikordner>") };
    let text = std::fs::read_to_string(reference).with_context(|| reference.clone())?;
    let rows = parse_reference(&text)?;
    // Je Kennzahl zwei Zählungen: [nur doppelt belegte Referenzwerte, alle Referenzwerte].
    let (mut tempo_n, mut tempo_ok, mut tempo_octave) = ([0usize; 2], [0usize; 2], [0usize; 2]);
    let (mut key_n, mut key_exact, mut key_weighted) = ([0usize; 2], [0usize; 2], [0.0f64; 2]);
    let add = |counter: &mut [usize; 2], sure: bool| {
        counter[1] += 1;
        counter[0] += sure as usize;
    };
    let mut absent = 0usize;
    for row in &rows {
        let path = Path::new(folder).join(&row.file);
        if !path.is_file() {
            println!("fehlt     {}", row.file);
            absent += 1;
            continue;
        }
        let (_, a) = match load(&path) {
            Ok(loaded) => loaded,
            Err(e) => {
                println!("unlesbar  {} ({e:#})", row.file);
                absent += 1;
                continue;
            }
        };
        let tempo = match row.bpm {
            Some(expected) => {
                let weak = if row.bpm_sure { "" } else { "?" };
                add(&mut tempo_n, row.bpm_sure);
                match tempo_verdict(a.bpm, expected, tol) {
                    TempoVerdict::Match => {
                        add(&mut tempo_ok, row.bpm_sure);
                        format!("Tempo ok {:.2}", a.bpm)
                    }
                    TempoVerdict::Octave => {
                        add(&mut tempo_octave, row.bpm_sure);
                        format!("Tempo HALB/DOPPELT {:.2} statt {expected}{weak}", a.bpm)
                    }
                    TempoVerdict::Off => format!("Tempo FALSCH {:.2} statt {expected}{weak}", a.bpm),
                }
            }
            None => format!("Tempo – {:.2}", a.bpm),
        };
        let key = match (row.key, a.key) {
            (Some(expected), Some(found)) => {
                add(&mut key_n, row.key_sure);
                let score = key_score(found, expected);
                key_weighted[1] += score;
                key_weighted[0] += if row.key_sure { score } else { 0.0 };
                if score == 1.0 {
                    add(&mut key_exact, row.key_sure);
                    format!("Tonart ok {}", found.camelot_string())
                } else {
                    let weak = if row.key_sure { "" } else { "?" };
                    format!("Tonart FALSCH {} statt {}{weak} ({score:.1})", found.camelot_string(), expected.camelot_string())
                }
            }
            (Some(expected), None) => {
                add(&mut key_n, row.key_sure);
                format!("Tonart FALSCH – statt {}", expected.camelot_string())
            }
            (None, found) => format!("Tonart – {}", found.map_or("–".into(), |k| k.camelot_string())),
        };
        println!("{tempo:<38} {key:<34} {}", row.file);
    }
    let pct = |n: usize, of: usize| if of == 0 { 0.0 } else { 100.0 * n as f64 / of as f64 };
    println!();
    println!("Ziele: Tempo ≥ 97 % innerhalb ±{tol} BPM, halb/doppelt ≤ 2 % (ANA-02); Tonart exakt ≥ 70 %, gewichtet ≥ 0,80 (ANA-06)");
    for (i, label) in ["Doppelt belegte Referenz", "Alle Referenzwerte (auch einfach belegte)"].into_iter().enumerate() {
        println!(
            "{label}: Tempo {} von {} = {:.0} %, halb/doppelt {} = {:.1} %; Tonart exakt {} von {} = {:.0} %, gewichtet {:.2}",
            tempo_ok[i],
            tempo_n[i],
            pct(tempo_ok[i], tempo_n[i]),
            tempo_octave[i],
            pct(tempo_octave[i], tempo_n[i]),
            key_exact[i],
            key_n[i],
            pct(key_exact[i], key_n[i]),
            if key_n[i] == 0 { 0.0 } else { key_weighted[i] / key_n[i] as f64 }
        );
    }
    if absent > 0 {
        println!("{absent} Dateien der Referenz fehlen im Ordner oder sind unlesbar und wurden nicht gewertet.");
    }
    Ok(())
}

/// Rendert einen Übergang so, wie die Anwendung ihn live fahren würde: A läuft, das Ende wird
/// vorgegeben, B erreicht dort seine Marke. Derselbe Weg wie in der Anwendung (`plan_live`).
fn cmd_transition(mut args: Vec<String>) -> Result<()> {
    use dw_automix::{plan_live, LiveDeck, Roles, Transition, When};
    use dw_core::{Command, DeckId};
    use dw_engine::Engine;

    let out_file = option(&mut args, "-o")?.ok_or_else(|| anyhow!("Ausgabedatei fehlt (-o <Datei.wav>)"))?;
    let bars: u32 = number(option(&mut args, "--bars")?, 32, "--bars")?;
    let lead: u32 = number(option(&mut args, "--lead")?, 16, "--lead")?;
    let tail: u32 = number(option(&mut args, "--tail")?, 16, "--tail")?;
    let template = option(&mut args, "--template")?.unwrap_or_else(|| "bass_swap".into());
    let need = |args: &mut Vec<String>, name: &str| -> Result<f64> {
        let value = option(args, name)?.ok_or_else(|| anyhow!("{name} fehlt"))?;
        value.replace(',', ".").parse().map_err(|_| anyhow!("{name}: „{value}“ ist keine Zahl"))
    };
    let a_pos = need(&mut args, "--a-pos")?;
    let end_in = need(&mut args, "--end-in")?;
    let b_mark = need(&mut args, "--b-mark")?;
    let [a_path, b_path] = args.as_slice() else { bail!("transition braucht genau zwei Dateien") };

    let (a_audio, a) = load(Path::new(a_path))?;
    let (b_audio, b) = load(Path::new(b_path))?;
    println!("A: {:.2} BPM, {} · B: {:.2} BPM, {}", a.bpm, key_text(&a), b.bpm, key_text(&b));
    let transition = Transition::templates(bars).into_iter().find(|t| t.name == template).ok_or_else(|| anyhow!("unbekannte Vorlage „{template}“ (bass_swap, fade)"))?;
    let track = |audio, an: &TrackAnalysis| Arc::new(Track { audio, grid: an.grid.clone(), gain_db: an.auto_gain_db(REFERENCE_LUFS) });

    let mut e = Engine::new(ENGINE_RATE);
    e.load(DeckId::A, track(a_audio, &a));
    e.load(DeckId::B, track(b_audio, &b));
    // A läuft so weit vor der genannten Stelle an, dass der Vorlauf zu hören ist.
    let start = (a_pos - lead as f64 * 4.0 * 60.0 / a.bpm).max(0.0);
    e.apply(Command::Seek { deck: DeckId::A, pos: start });
    e.apply(Command::Seek { deck: DeckId::B, pos: b_mark });
    e.apply(Command::Play { deck: DeckId::A, on: true });

    let mut out: Vec<f32> = Vec::new();
    let mut cue = vec![0.0f32; 256 * 2];
    let mut block = |e: &mut Engine, out: &mut Vec<f32>| {
        let at = out.len();
        out.resize(at + 256 * 2, 0.0);
        e.process(&mut out[at..], &mut cue);
    };
    while e.snapshot().decks[0].pos < a_pos {
        block(&mut e, &mut out);
    }

    // „Jetzt": A steht an der genannten Stelle. Von hier aus wird geplant wie in der Anwendung.
    let s = e.snapshot();
    let plan = plan_live(
        &transition,
        Roles { out: DeckId::A, inn: DeckId::B },
        s.clock_beats,
        LiveDeck { grid: &a.grid, playing: true, pos: s.decks[0].pos, duration: s.decks[0].duration, track_bpm: s.decks[0].track_bpm },
        LiveDeck { grid: &b.grid, playing: false, pos: b_mark, duration: s.decks[1].duration, track_bpm: s.decks[1].track_bpm },
        When::EndIn { beats: end_in },
    )?;
    let l = transition.beats();
    let (t0, end) = (plan.t0, plan.t0 + l);
    let a_end = a.grid.sec_at(a.grid.beat_at(s.decks[0].pos) + (end - s.clock_beats));
    let b_end = b.grid.sec_at(b.grid.beat_at(plan.in_pos) + l);
    println!(
        "Übergang „{}“ über {} Takte: Ende bei {:.2} s in A und {:.2} s in B; B startet bei {:.2} s",
        transition.name, transition.bars, a_end, b_end, plan.in_pos
    );
    for cmd in &plan.now {
        e.apply(*cmd);
    }
    for (beat, cmd) in &plan.scheduled {
        if !e.schedule(*beat, *cmd) {
            bail!("Vormerk-Speicher der Engine voll");
        }
    }

    let stop = end + tail as f64 * 4.0;
    let mut marks = [(t0, 0usize), (t0 + l / 2.0, 0usize), (end, 0usize)];
    let mut worst = 0.0f64;
    while e.clock_beats() < stop {
        let before = e.clock_beats();
        let frame = out.len() / 2;
        block(&mut e, &mut out);
        for (beat, at) in marks.iter_mut() {
            if before < *beat && e.clock_beats() >= *beat {
                *at = frame;
            }
        }
        let s = e.snapshot();
        if s.decks[0].playing && s.decks[1].playing {
            worst = worst.max(s.decks[1].phase.abs());
        }
        if !s.decks[0].playing && !s.decks[1].playing {
            break;
        }
    }
    let mix = AudioBuf::new(ENGINE_RATE, out);
    let secs = |frame: usize| frame as f64 / ENGINE_RATE as f64;
    println!(
        "Gerendert: {:.1} s · Übergang von {:.1} s bis {:.1} s, Mitte bei {:.1} s · größter Beat-Versatz {:.4} Beats",
        mix.duration(),
        secs(marks[0].1),
        secs(marks[2].1),
        secs(marks[1].1),
        worst
    );
    write_wav(&PathBuf::from(&out_file), &mix, 24).with_context(|| out_file.clone())?;
    println!("Geschrieben: {out_file}");
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_file_is_parsed_with_gaps_and_comments() {
        let text = "# Kommentar\nDatei;BPM;Tonart;Quelle\na.mp3;128;9A;Beatport\nb.mp3;;8B?;nur Tonart\nc.mp3;100,5?;;Tempo mit Komma\n\nd.mp3\n";
        let rows = parse_reference(text).unwrap();
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0], Reference { file: "a.mp3".into(), bpm: Some(128.0), bpm_sure: true, key: Key::from_camelot(9, 'A'), key_sure: true });
        // „?" = nur eine Quelle.
        assert_eq!((rows[1].bpm, rows[1].key, rows[1].key_sure), (None, Key::from_camelot(8, 'B'), false));
        assert_eq!((rows[2].bpm, rows[2].bpm_sure, rows[2].key), (Some(100.5), false, None));
        assert_eq!((rows[3].bpm, rows[3].key), (None, None));
        assert!(parse_reference("a.mp3;schnell;9A").is_err());
        assert!(parse_reference("a.mp3;128;13A").is_err());
    }

    #[test]
    fn tempo_is_classified_as_match_octave_or_off() {
        assert_eq!(tempo_verdict(128.04, 128.0, 0.05), TempoVerdict::Match);
        assert_eq!(tempo_verdict(128.06, 128.0, 0.05), TempoVerdict::Off);
        assert_eq!(tempo_verdict(200.0, 100.0, 0.05), TempoVerdict::Octave);
        assert_eq!(tempo_verdict(70.0, 140.0, 0.05), TempoVerdict::Octave);
        assert_eq!(tempo_verdict(137.0, 130.0, 0.05), TempoVerdict::Off);
    }

    /// Gewichtung nach Spezifikation 5.3.
    #[test]
    fn key_score_follows_the_specified_weights() {
        let k = |n, l| Key::from_camelot(n, l).unwrap();
        assert_eq!(key_score(k(9, 'A'), k(9, 'A')), 1.0);
        // Quinte daneben: e-Moll (9A) gegen a-Moll (8A) und h-Moll (10A), auch über die 12 hinweg.
        assert_eq!(key_score(k(8, 'A'), k(9, 'A')), 0.5);
        assert_eq!(key_score(k(10, 'A'), k(9, 'A')), 0.5);
        assert_eq!(key_score(k(1, 'B'), k(12, 'B')), 0.5);
        // Paralleltonart: e-Moll (9A) gegen G-Dur (9B).
        assert_eq!(key_score(k(9, 'B'), k(9, 'A')), 0.3);
        // Gleichnamig: E-Dur (12B) gegen e-Moll (9A).
        assert_eq!(key_score(k(12, 'B'), k(9, 'A')), 0.2);
        assert_eq!(key_score(k(3, 'A'), k(9, 'A')), 0.0);
    }
}
