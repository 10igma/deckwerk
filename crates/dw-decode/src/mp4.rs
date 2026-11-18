//! Liest aus einer MP4-Datei die Schnittliste (`elst`) der Audiospur. Sie gibt an,
//! wie viele Samples Encoder-Vorlauf am Anfang zu verwerfen sind und wie lang der
//! gültige Teil ist. Der Demuxer wertet sie nicht aus, bei AAC stimmen sonst alle
//! Positionen um den Vorlauf (typisch 1 024 oder 2 112 Samples) nicht.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

/// Vorlauf und gültige Länge, beide in Samples der Audiospur.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EditInfo {
    pub delay: u64,
    pub valid: Option<u64>,
}

struct Atom {
    kind: [u8; 4],
    start: u64,
    end: u64,
}

fn read_u32(f: &mut File) -> std::io::Result<u32> {
    let mut b = [0u8; 4];
    f.read_exact(&mut b)?;
    Ok(u32::from_be_bytes(b))
}

fn read_u64(f: &mut File) -> std::io::Result<u64> {
    let mut b = [0u8; 8];
    f.read_exact(&mut b)?;
    Ok(u64::from_be_bytes(b))
}

/// Listet die Atome im Bereich `start..end` auf.
fn atoms(f: &mut File, start: u64, end: u64) -> std::io::Result<Vec<Atom>> {
    let mut out = Vec::new();
    let mut pos = start;
    while pos + 8 <= end {
        f.seek(SeekFrom::Start(pos))?;
        let size32 = read_u32(f)? as u64;
        let mut kind = [0u8; 4];
        f.read_exact(&mut kind)?;
        let (header, size) = match size32 {
            0 => (8, end - pos),
            1 => (16, read_u64(f)?),
            n => (8, n),
        };
        if size < header || pos + size > end {
            break;
        }
        out.push(Atom { kind, start: pos + header, end: pos + size });
        pos += size;
    }
    Ok(out)
}

fn find(f: &mut File, parent: &Atom, kind: &[u8; 4]) -> std::io::Result<Option<Atom>> {
    Ok(atoms(f, parent.start, parent.end)?.into_iter().find(|a| &a.kind == kind))
}

/// Zeitbasis aus `mvhd` oder `mdhd` (Feld liegt je nach Version an anderer Stelle).
fn timescale(f: &mut File, atom: &Atom) -> std::io::Result<u32> {
    f.seek(SeekFrom::Start(atom.start))?;
    let version = read_u32(f)? >> 24;
    f.seek(SeekFrom::Current(if version == 1 { 16 } else { 8 }))?;
    read_u32(f)
}

pub fn audio_edit_info(path: &Path) -> std::io::Result<Option<EditInfo>> {
    let mut f = File::open(path)?;
    let len = f.metadata()?.len();
    let root = Atom { kind: *b"root", start: 0, end: len };
    let Some(moov) = find(&mut f, &root, b"moov")? else { return Ok(None) };
    let movie_scale = match find(&mut f, &moov, b"mvhd")? {
        Some(a) => timescale(&mut f, &a)?,
        None => return Ok(None),
    };
    for trak in atoms(&mut f, moov.start, moov.end)?.into_iter().filter(|a| &a.kind == b"trak") {
        let Some(mdia) = find(&mut f, &trak, b"mdia")? else { continue };
        // Nur die Tonspur: Handler-Typ „soun".
        let Some(hdlr) = find(&mut f, &mdia, b"hdlr")? else { continue };
        f.seek(SeekFrom::Start(hdlr.start + 8))?;
        let mut handler = [0u8; 4];
        f.read_exact(&mut handler)?;
        if &handler != b"soun" {
            continue;
        }
        let media_scale = match find(&mut f, &mdia, b"mdhd")? {
            Some(a) => timescale(&mut f, &a)?,
            None => continue,
        };
        let Some(edts) = find(&mut f, &trak, b"edts")? else { return Ok(None) };
        let Some(elst) = find(&mut f, &edts, b"elst")? else { return Ok(None) };
        f.seek(SeekFrom::Start(elst.start))?;
        let version = read_u32(&mut f)? >> 24;
        let count = read_u32(&mut f)?;
        for _ in 0..count.min(16) {
            let (duration, media_time) = if version == 1 {
                (read_u64(&mut f)?, read_u64(&mut f)? as i64)
            } else {
                (read_u32(&mut f)? as u64, read_u32(&mut f)? as i32 as i64)
            };
            let _rate = read_u32(&mut f)?;
            // Einträge mit negativer Zeit sind Leerstellen; der erste echte Eintrag zählt.
            if media_time >= 0 {
                let valid = (movie_scale > 0 && duration > 0)
                    .then(|| (duration as u128 * media_scale as u128 / movie_scale as u128) as u64);
                return Ok(Some(EditInfo { delay: media_time as u64, valid }));
            }
        }
        return Ok(None);
    }
    Ok(None)
}
