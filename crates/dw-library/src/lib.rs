//! Musikbibliothek (Spezifikation 4.1, 6.2): SQLite-Datenbank mit Tracks, Analyse,
//! Playlists, Suche und automatischer Sicherung.

mod import;
mod schema;

pub use import::{fingerprint, ImportReport, SUPPORTED_EXTENSIONS};

use dw_core::{BeatGrid, GridAnchor, Key, TrackAnalysis};
use rusqlite::{params, params_from_iter, Connection, OptionalExtension};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

const DB_FILE: &str = "library.db";
const BACKUP_DIR: &str = "backups";
const KEEP_BACKUPS: usize = 10;

#[derive(Debug, thiserror::Error)]
pub enum LibraryError {
    #[error("Datenbankfehler: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("Dateifehler: {0}")]
    Io(#[from] std::io::Error),
    #[error("Die Bibliothek ist beschädigt; {} Sicherung(en) vorhanden", .backups.len())]
    Corrupt { backups: Vec<PathBuf> },
    #[error("Eintrag nicht gefunden")]
    NotFound,
}

pub type Result<T> = std::result::Result<T, LibraryError>;

pub type TrackId = i64;
pub type PlaylistId = i64;

/// Zeile der Trackliste.
#[derive(Clone, Debug, PartialEq)]
pub struct TrackRow {
    pub id: TrackId,
    pub path: PathBuf,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub label: String,
    pub genre: String,
    pub year: Option<u32>,
    pub comment: String,
    pub duration: f64,
    pub format: String,
    /// Tempo aus der Analyse, sonst aus den Datei-Angaben.
    pub bpm: Option<f64>,
    pub key: Option<Key>,
    pub lufs: Option<f32>,
    pub rating: u8,
    pub analyzed: bool,
    pub missing: bool,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SortColumn {
    #[default]
    Added,
    Title,
    Artist,
    Bpm,
    Key,
    Duration,
    Rating,
}

/// Suche und Filter für die Trackliste (BIB-08, BIB-09).
#[derive(Clone, Debug, Default)]
pub struct TrackQuery {
    /// Suchtext; jedes Wort muss in Titel, Künstler, Album, Label oder Kommentar vorkommen.
    pub text: String,
    pub bpm_range: Option<(f64, f64)>,
    pub keys: Vec<Key>,
    pub playlist: Option<PlaylistId>,
    pub sort: SortColumn,
    pub descending: bool,
    pub limit: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Playlist {
    pub id: PlaylistId,
    pub parent: Option<PlaylistId>,
    pub name: String,
    pub is_folder: bool,
}

pub struct Library {
    conn: Connection,
    dir: PathBuf,
}

fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs() as i64)
}

fn search_text(parts: &[&str]) -> String {
    parts.join(" ").to_lowercase()
}

impl Library {
    /// Öffnet die Bibliothek im Ordner `dir`, prüft sie und legt eine Sicherung an (BIB-18).
    pub fn open(dir: &Path) -> Result<Library> {
        std::fs::create_dir_all(dir.join(BACKUP_DIR))?;
        let db_path = dir.join(DB_FILE);
        let existed = db_path.exists();
        let healthy = match Connection::open(&db_path) {
            Ok(conn) => conn
                .query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
                .map(|s| s == "ok")
                .unwrap_or(false)
                .then_some(conn),
            Err(_) => None,
        };
        let Some(conn) = healthy else {
            return Err(LibraryError::Corrupt { backups: Self::backups(dir) });
        };
        conn.execute_batch("PRAGMA journal_mode = WAL; PRAGMA foreign_keys = ON; PRAGMA synchronous = NORMAL;")?;
        schema::migrate(&conn)?;
        let lib = Library { conn, dir: dir.to_path_buf() };
        if existed {
            lib.backup()?;
        }
        Ok(lib)
    }

    /// Vorhandene Sicherungen, neueste zuerst.
    pub fn backups(dir: &Path) -> Vec<PathBuf> {
        let mut files: Vec<PathBuf> = std::fs::read_dir(dir.join(BACKUP_DIR))
            .map(|rd| rd.filter_map(|e| e.ok().map(|e| e.path())).filter(|p| p.extension().is_some_and(|e| e == "db")).collect())
            .unwrap_or_default();
        files.sort();
        files.reverse();
        files
    }

    /// Ersetzt die Bibliothek durch eine Sicherung. Die beschädigte Datei bleibt als `.defekt` erhalten.
    pub fn restore(dir: &Path, backup: &Path) -> Result<Library> {
        let db_path = dir.join(DB_FILE);
        if db_path.exists() {
            std::fs::rename(&db_path, dir.join(format!("{DB_FILE}.defekt")))?;
        }
        for ext in ["-wal", "-shm"] {
            let _ = std::fs::remove_file(dir.join(format!("{DB_FILE}{ext}")));
        }
        std::fs::copy(backup, &db_path)?;
        Self::open(dir)
    }

    fn backup(&self) -> Result<()> {
        let dir = self.dir.join(BACKUP_DIR);
        let mut target = dir.join(format!("library-{:012}.db", now()));
        let mut n = 1;
        while target.exists() {
            target = dir.join(format!("library-{:012}-{n}.db", now()));
            n += 1;
        }
        self.conn.execute("VACUUM INTO ?1", params![target.to_string_lossy()])?;
        for old in Self::backups(&self.dir).into_iter().skip(KEEP_BACKUPS) {
            let _ = std::fs::remove_file(old);
        }
        Ok(())
    }

    pub fn track_count(&self) -> Result<usize> {
        Ok(self.conn.query_row("SELECT COUNT(*) FROM track", [], |r| r.get::<_, i64>(0))? as usize)
    }

    /// Trackliste mit Suche, Filter und Sortierung.
    pub fn tracks(&self, q: &TrackQuery) -> Result<Vec<TrackRow>> {
        let mut sql = String::from(
            "SELECT t.id, t.path, t.title, t.artist, t.album, t.label, t.genre, t.year, t.comment, t.duration, t.format,
                    COALESCE(a.bpm, t.tag_bpm), a.key, a.lufs, t.rating, a.track_id IS NOT NULL, t.missing
             FROM track t LEFT JOIN analysis a ON a.track_id = t.id",
        );
        let mut args: Vec<rusqlite::types::Value> = Vec::new();
        let mut conds: Vec<String> = Vec::new();
        if let Some(pl) = q.playlist {
            sql.push_str(" JOIN playlist_item pi ON pi.track_id = t.id AND pi.playlist_id = ?");
            args.push(pl.into());
        }
        for word in q.text.split_whitespace() {
            conds.push("instr(t.search, ?) > 0".into());
            args.push(word.to_lowercase().into());
        }
        if let Some((lo, hi)) = q.bpm_range {
            conds.push("COALESCE(a.bpm, t.tag_bpm) BETWEEN ? AND ?".into());
            args.push(lo.into());
            args.push(hi.into());
        }
        if !q.keys.is_empty() {
            let marks = vec!["?"; q.keys.len()].join(",");
            conds.push(format!("a.key IN ({marks})"));
            args.extend(q.keys.iter().map(|k| (k.index() as i64).into()));
        }
        if !conds.is_empty() {
            sql.push_str(" WHERE ");
            sql.push_str(&conds.join(" AND "));
        }
        let column = match q.sort {
            SortColumn::Added if q.playlist.is_some() => "pi.position",
            SortColumn::Added => "t.id",
            SortColumn::Title => "t.title COLLATE NOCASE",
            SortColumn::Artist => "t.artist COLLATE NOCASE",
            SortColumn::Bpm => "COALESCE(a.bpm, t.tag_bpm)",
            SortColumn::Key => "a.key",
            SortColumn::Duration => "t.duration",
            SortColumn::Rating => "t.rating",
        };
        sql.push_str(&format!(" ORDER BY {column} {}, t.id", if q.descending { "DESC" } else { "ASC" }));
        if let Some(limit) = q.limit {
            sql.push_str(&format!(" LIMIT {limit}"));
        }
        let mut stmt = self.conn.prepare_cached(&sql)?;
        let rows = stmt.query_map(params_from_iter(args), |r| {
            Ok(TrackRow {
                id: r.get(0)?,
                path: PathBuf::from(r.get::<_, String>(1)?),
                title: r.get(2)?,
                artist: r.get(3)?,
                album: r.get(4)?,
                label: r.get(5)?,
                genre: r.get(6)?,
                year: r.get(7)?,
                comment: r.get(8)?,
                duration: r.get(9)?,
                format: r.get(10)?,
                bpm: r.get(11)?,
                key: r.get::<_, Option<i64>>(12)?.and_then(|k| Key::from_index(k as u8)),
                lufs: r.get::<_, Option<f64>>(13)?.map(|v| v as f32),
                rating: r.get::<_, i64>(14)? as u8,
                analyzed: r.get(15)?,
                missing: r.get(16)?,
            })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    pub fn track(&self, id: TrackId) -> Result<TrackRow> {
        // Ein einzelner Track ist die Liste ohne Filter, eingeschränkt auf die Kennung.
        let all = self.tracks(&TrackQuery::default())?;
        all.into_iter().find(|t| t.id == id).ok_or(LibraryError::NotFound)
    }

    /// Tracks ohne Analyse oder mit veralteter Analyse-Version (ANA-17), außer gesperrten.
    pub fn pending_analysis(&self, current_version: u32) -> Result<Vec<(TrackId, PathBuf)>> {
        let mut stmt = self.conn.prepare(
            "SELECT t.id, t.path FROM track t LEFT JOIN analysis a ON a.track_id = t.id
             WHERE t.missing = 0 AND (a.track_id IS NULL OR (a.version < ?1 AND a.locked = 0)) ORDER BY t.id",
        )?;
        let rows = stmt.query_map(params![current_version], |r| Ok((r.get(0)?, PathBuf::from(r.get::<_, String>(1)?))))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    /// Speichert das Analyseergebnis. Eine gesperrte Analyse wird nicht überschrieben (ANA-15).
    /// Gibt zurück, ob gespeichert wurde.
    pub fn set_analysis(&mut self, id: TrackId, a: &TrackAnalysis) -> Result<bool> {
        let locked: Option<bool> =
            self.conn.query_row("SELECT locked FROM analysis WHERE track_id = ?1", params![id], |r| r.get(0)).optional()?;
        if locked == Some(true) {
            return Ok(false);
        }
        let tx = self.conn.transaction()?;
        tx.execute(
            "INSERT OR REPLACE INTO analysis
             (track_id, version, locked, bpm, bpm_confidence, key, key_confidence, tuning_cents, lufs, lra, true_peak, bar_offset, source)
             VALUES (?1, ?2, 0, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 'analyse')",
            params![
                id,
                a.version,
                a.bpm,
                a.bpm_confidence as f64,
                a.key.map(|k| k.index() as i64),
                a.key_confidence as f64,
                a.tuning_cents as f64,
                a.lufs as f64,
                a.lra as f64,
                a.true_peak_db as f64,
                a.grid.bar_offset as i64
            ],
        )?;
        tx.execute("DELETE FROM grid_anchor WHERE track_id = ?1", params![id])?;
        for (i, anchor) in a.grid.anchors().iter().enumerate() {
            tx.execute(
                "INSERT INTO grid_anchor (track_id, idx, pos, bpm) VALUES (?1, ?2, ?3, ?4)",
                params![id, i as i64, anchor.pos, anchor.bpm],
            )?;
        }
        tx.commit()?;
        Ok(true)
    }

    pub fn analysis(&self, id: TrackId) -> Result<Option<TrackAnalysis>> {
        let head = self
            .conn
            .query_row(
                "SELECT a.version, a.bpm, a.bpm_confidence, a.key, a.key_confidence, a.tuning_cents, a.lufs, a.lra, a.true_peak,
                        a.bar_offset, t.duration
                 FROM analysis a JOIN track t ON t.id = a.track_id WHERE a.track_id = ?1",
                params![id],
                |r| {
                    Ok((
                        r.get::<_, u32>(0)?,
                        r.get::<_, f64>(1)?,
                        r.get::<_, f64>(2)?,
                        r.get::<_, Option<i64>>(3)?,
                        r.get::<_, f64>(4)?,
                        r.get::<_, f64>(5)?,
                        r.get::<_, f64>(6)?,
                        r.get::<_, f64>(7)?,
                        r.get::<_, f64>(8)?,
                        r.get::<_, i64>(9)?,
                        r.get::<_, f64>(10)?,
                    ))
                },
            )
            .optional()?;
        let Some((version, bpm, bpm_conf, key, key_conf, tuning, lufs, lra, true_peak, bar_offset, duration)) = head else {
            return Ok(None);
        };
        let mut stmt = self.conn.prepare_cached("SELECT pos, bpm FROM grid_anchor WHERE track_id = ?1 ORDER BY idx")?;
        let anchors: Vec<GridAnchor> = stmt
            .query_map(params![id], |r| Ok(GridAnchor { pos: r.get(0)?, bpm: r.get(1)? }))?
            .collect::<rusqlite::Result<_>>()?;
        if anchors.is_empty() {
            return Ok(None);
        }
        Ok(Some(TrackAnalysis {
            version,
            duration,
            bpm,
            bpm_confidence: bpm_conf as f32,
            grid: BeatGrid::from_anchors(anchors, bar_offset as u8),
            key: key.and_then(|k| Key::from_index(k as u8)),
            key_confidence: key_conf as f32,
            tuning_cents: tuning as f32,
            lufs: lufs as f32,
            lra: lra as f32,
            true_peak_db: true_peak as f32,
        }))
    }

    pub fn set_analysis_locked(&mut self, id: TrackId, locked: bool) -> Result<()> {
        self.conn.execute("UPDATE analysis SET locked = ?2 WHERE track_id = ?1", params![id, locked])?;
        Ok(())
    }

    pub fn set_rating(&mut self, id: TrackId, rating: u8) -> Result<()> {
        self.conn.execute("UPDATE track SET rating = ?2 WHERE id = ?1", params![id, rating.min(5) as i64])?;
        Ok(())
    }

    // ---------- Playlists (BIB-06) ----------

    pub fn create_playlist(&mut self, name: &str, parent: Option<PlaylistId>, is_folder: bool) -> Result<PlaylistId> {
        let position: i64 = self.conn.query_row(
            "SELECT COALESCE(MAX(position), -1) + 1 FROM playlist WHERE parent IS ?1",
            params![parent],
            |r| r.get(0),
        )?;
        self.conn.execute(
            "INSERT INTO playlist (parent, name, kind, position) VALUES (?1, ?2, ?3, ?4)",
            params![parent, name, if is_folder { "ordner" } else { "liste" }, position],
        )?;
        Ok(self.conn.last_insert_rowid())
    }

    pub fn rename_playlist(&mut self, id: PlaylistId, name: &str) -> Result<()> {
        match self.conn.execute("UPDATE playlist SET name = ?2 WHERE id = ?1", params![id, name])? {
            0 => Err(LibraryError::NotFound),
            _ => Ok(()),
        }
    }

    /// Löscht eine Playlist oder einen Ordner samt Inhalt. Die Tracks bleiben in der Bibliothek.
    pub fn delete_playlist(&mut self, id: PlaylistId) -> Result<()> {
        self.conn.execute("DELETE FROM playlist WHERE id = ?1", params![id])?;
        Ok(())
    }

    pub fn playlists(&self) -> Result<Vec<Playlist>> {
        let mut stmt = self.conn.prepare("SELECT id, parent, name, kind FROM playlist ORDER BY parent, position")?;
        let rows = stmt.query_map([], |r| {
            Ok(Playlist { id: r.get(0)?, parent: r.get(1)?, name: r.get(2)?, is_folder: r.get::<_, String>(3)? == "ordner" })
        })?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn playlist_ids(&self, id: PlaylistId) -> Result<Vec<TrackId>> {
        let mut stmt = self.conn.prepare_cached("SELECT track_id FROM playlist_item WHERE playlist_id = ?1 ORDER BY position")?;
        let rows = stmt.query_map(params![id], |r| r.get(0))?;
        Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
    }

    fn write_playlist(&mut self, id: PlaylistId, tracks: &[TrackId]) -> Result<()> {
        let tx = self.conn.transaction()?;
        tx.execute("DELETE FROM playlist_item WHERE playlist_id = ?1", params![id])?;
        for (i, t) in tracks.iter().enumerate() {
            tx.execute("INSERT INTO playlist_item (playlist_id, position, track_id) VALUES (?1, ?2, ?3)", params![id, i as i64, t])?;
        }
        tx.commit()?;
        Ok(())
    }

    /// Fügt Tracks an der Stelle `at` ein (`None` = ans Ende).
    pub fn add_to_playlist(&mut self, id: PlaylistId, tracks: &[TrackId], at: Option<usize>) -> Result<()> {
        let mut items = self.playlist_ids(id)?;
        let at = at.unwrap_or(items.len()).min(items.len());
        items.splice(at..at, tracks.iter().copied());
        self.write_playlist(id, &items)
    }

    pub fn remove_from_playlist(&mut self, id: PlaylistId, position: usize) -> Result<()> {
        let mut items = self.playlist_ids(id)?;
        if position >= items.len() {
            return Err(LibraryError::NotFound);
        }
        items.remove(position);
        self.write_playlist(id, &items)
    }

    /// Verschiebt den Eintrag von `from` nach `to`.
    pub fn move_in_playlist(&mut self, id: PlaylistId, from: usize, to: usize) -> Result<()> {
        let mut items = self.playlist_ids(id)?;
        if from >= items.len() {
            return Err(LibraryError::NotFound);
        }
        let item = items.remove(from);
        items.insert(to.min(items.len()), item);
        self.write_playlist(id, &items)
    }
}
