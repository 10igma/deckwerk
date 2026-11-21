//! Import von Dateien und Ordnern (BIB-01, BIB-03, BIB-05).

use crate::{now, search_text, Library, Result};
use lofty::config::{ParseOptions, ParsingMode};
use lofty::file::TaggedFile;
use lofty::prelude::*;
use lofty::probe::Probe;
use rusqlite::{params, OptionalExtension};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

pub const SUPPORTED_EXTENSIONS: &[&str] = &["mp3", "wav", "aif", "aiff", "flac", "m4a", "mp4", "aac", "ogg", "oga", "opus"];

/// Größe des Anfangs- und Endstücks, das in den Fingerabdruck eingeht.
const FINGERPRINT_CHUNK: u64 = 1 << 20;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportReport {
    pub added: usize,
    /// Bereits vorhanden (gleicher Pfad oder gleicher Inhalt).
    pub duplicates: usize,
    /// Bekannter Inhalt an neuem Ort: Der Pfad wurde aktualisiert, die Analyse bleibt.
    pub relinked: usize,
    /// Dateien, die sich nicht lesen ließen, mit Grund.
    pub failed: Vec<(PathBuf, String)>,
}

/// Schneller Fingerabdruck des Inhalts: Dateigröße plus das erste und letzte Megabyte.
/// Damit lassen sich 1 000 Dateien in Sekunden importieren, ohne jede ganz zu lesen.
pub fn fingerprint(path: &Path) -> std::io::Result<String> {
    let mut file = std::fs::File::open(path)?;
    let size = file.metadata()?.len();
    let mut hasher = blake3::Hasher::new();
    hasher.update(&size.to_le_bytes());
    let mut buf = vec![0u8; FINGERPRINT_CHUNK.min(size) as usize];
    file.read_exact(&mut buf)?;
    hasher.update(&buf);
    if size > 2 * FINGERPRINT_CHUNK {
        file.seek(SeekFrom::End(-(FINGERPRINT_CHUNK as i64)))?;
        file.read_exact(&mut buf)?;
        hasher.update(&buf);
    }
    Ok(hasher.finalize().to_hex()[..32].to_string())
}

fn is_supported(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|e| SUPPORTED_EXTENSIONS.contains(&e.to_ascii_lowercase().as_str()))
}

fn collect(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_dir() {
        let mut entries: Vec<PathBuf> = std::fs::read_dir(path).map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).collect()).unwrap_or_default();
        entries.sort();
        for e in entries {
            collect(&e, out);
        }
    } else if is_supported(path) {
        out.push(path.to_path_buf());
    }
}

#[derive(Default)]
struct Meta {
    title: String,
    artist: String,
    album: String,
    label: String,
    genre: String,
    year: Option<u32>,
    comment: String,
    bpm: Option<f64>,
    key: Option<String>,
    duration: f64,
    sample_rate: u32,
    channels: u8,
}

/// Liest Eigenschaften und Tags. Fehlerhafte Angaben (etwa ein Datum im Jahres-Feld) sollen den
/// Track nicht aus der Bibliothek halten: erst nachsichtig lesen, zur Not ganz ohne Tags.
fn read_tagged(path: &Path) -> std::result::Result<TaggedFile, String> {
    let attempts = [ParseOptions::new().parsing_mode(ParsingMode::Relaxed), ParseOptions::new().read_tags(false)];
    let mut first_error = None;
    for options in attempts {
        match Probe::open(path).map_err(|e| e.to_string())?.options(options).read() {
            Ok(tagged) => return Ok(tagged),
            Err(e) => first_error = first_error.or(Some(e.to_string())),
        }
    }
    Err(first_error.unwrap_or_default())
}

fn read_meta(path: &Path) -> std::result::Result<Meta, String> {
    let tagged = read_tagged(path)?;
    let props = tagged.properties();
    let mut m = Meta {
        duration: props.duration().as_secs_f64(),
        sample_rate: props.sample_rate().unwrap_or(0),
        channels: props.channels().unwrap_or(0),
        ..Default::default()
    };
    if let Some(tag) = tagged.primary_tag().or_else(|| tagged.first_tag()) {
        let text = |key: &ItemKey| tag.get_string(key).map(str::trim).filter(|s| !s.is_empty()).map(str::to_string);
        m.title = tag.title().map(|s| s.trim().to_string()).unwrap_or_default();
        m.artist = tag.artist().map(|s| s.trim().to_string()).unwrap_or_default();
        m.album = tag.album().map(|s| s.trim().to_string()).unwrap_or_default();
        m.genre = tag.genre().map(|s| s.trim().to_string()).unwrap_or_default();
        // Manche Programme legen den Kommentar unter anderem Namen ab
        // (DESCRIPTION in FLAC/OGG, freies Textfeld „comment" in ID3).
        let loose_comment = || {
            tag.items().find_map(|item| match (item.key(), item.value().text()) {
                (ItemKey::Unknown(k), Some(v)) if k.eq_ignore_ascii_case("description") || k.eq_ignore_ascii_case("comment") => {
                    Some(v.trim().to_string())
                }
                _ => None,
            })
        };
        m.comment = tag
            .comment()
            .map(|s| s.trim().to_string())
            .or_else(|| text(&ItemKey::Description))
            .or_else(loose_comment)
            .unwrap_or_default();
        m.year = tag.year();
        m.label = text(&ItemKey::Label).or_else(|| text(&ItemKey::Publisher)).unwrap_or_default();
        m.bpm = text(&ItemKey::Bpm)
            .or_else(|| text(&ItemKey::IntegerBpm))
            .and_then(|s| s.replace(',', ".").parse::<f64>().ok())
            .filter(|b| (20.0..400.0).contains(b));
        m.key = text(&ItemKey::InitialKey);
    }
    Ok(m)
}

impl Library {
    /// Importiert Dateien und Ordner (rekursiv). Unlesbare Dateien brechen den Import nicht ab.
    pub fn import_paths(&mut self, paths: &[PathBuf]) -> Result<ImportReport> {
        let mut files = Vec::new();
        for p in paths {
            collect(p, &mut files);
        }
        let mut report = ImportReport::default();
        let tx = self.conn.transaction()?;
        for file in files {
            let path_str = file.to_string_lossy().to_string();
            let known: Option<i64> = tx.query_row("SELECT id FROM track WHERE path = ?1", params![path_str], |r| r.get(0)).optional()?;
            if known.is_some() {
                report.duplicates += 1;
                continue;
            }
            let (hash, size, mtime) = match (fingerprint(&file), std::fs::metadata(&file)) {
                (Ok(h), Ok(md)) => {
                    let mtime = md.modified().ok().and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map_or(0, |d| d.as_secs() as i64);
                    (h, md.len() as i64, mtime)
                }
                (Err(e), _) | (_, Err(e)) => {
                    report.failed.push((file, e.to_string()));
                    continue;
                }
            };
            // Gleicher Inhalt schon bekannt: entweder Duplikat oder verschobene Datei.
            let same: Option<(i64, String)> =
                tx.query_row("SELECT id, path FROM track WHERE hash = ?1", params![hash], |r| Ok((r.get(0)?, r.get(1)?))).optional()?;
            if let Some((id, old_path)) = same {
                if Path::new(&old_path).exists() {
                    report.duplicates += 1;
                } else {
                    tx.execute("UPDATE track SET path = ?2, mtime = ?3, missing = 0 WHERE id = ?1", params![id, path_str, mtime])?;
                    report.relinked += 1;
                }
                continue;
            }
            let meta = match read_meta(&file) {
                Ok(m) => m,
                Err(e) => {
                    report.failed.push((file, e));
                    continue;
                }
            };
            let stem = file.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_default();
            // Ohne Titel-Angabe dient der Dateiname als Titel.
            let title = if meta.title.is_empty() { stem } else { meta.title };
            let format = file.extension().map(|e| e.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            let search = search_text(&[&title, &meta.artist, &meta.album, &meta.label, &meta.comment, &meta.genre]);
            tx.execute(
                "INSERT INTO track (path, hash, size, mtime, format, sample_rate, channels, duration, title, artist, album, label,
                                    genre, year, comment, tag_bpm, tag_key, search, added_at)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)",
                params![
                    path_str, hash, size, mtime, format, meta.sample_rate, meta.channels, meta.duration, title, meta.artist,
                    meta.album, meta.label, meta.genre, meta.year, meta.comment, meta.bpm, meta.key, search, now()
                ],
            )?;
            report.added += 1;
        }
        tx.commit()?;
        Ok(report)
    }

    /// Markiert Tracks, deren Datei nicht mehr am gespeicherten Ort liegt. Gibt deren Anzahl zurück.
    pub fn mark_missing(&mut self) -> Result<usize> {
        let paths: Vec<(i64, String)> = {
            let mut stmt = self.conn.prepare("SELECT id, path FROM track")?;
            let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<rusqlite::Result<_>>()?
        };
        let tx = self.conn.transaction()?;
        let mut missing = 0;
        for (id, path) in paths {
            let gone = !Path::new(&path).exists();
            missing += gone as usize;
            tx.execute("UPDATE track SET missing = ?2 WHERE id = ?1", params![id, gone])?;
        }
        tx.commit()?;
        Ok(missing)
    }
}
