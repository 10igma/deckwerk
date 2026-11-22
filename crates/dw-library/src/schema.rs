//! Datenbank-Schema mit Versionsnummer und schrittweiser Migration (9.4).

use rusqlite::Connection;

/// Jede Stufe hebt die Schema-Version um eins. Bestehende Einträge bleiben unverändert.
const MIGRATIONS: &[&str] = &[
    // Version 1: Tracks, Analyse, Beatgrid, Playlists
    "CREATE TABLE track (
        id INTEGER PRIMARY KEY,
        path TEXT NOT NULL UNIQUE,
        hash TEXT NOT NULL,
        size INTEGER NOT NULL,
        mtime INTEGER NOT NULL,
        format TEXT NOT NULL DEFAULT '',
        sample_rate INTEGER NOT NULL DEFAULT 0,
        channels INTEGER NOT NULL DEFAULT 0,
        duration REAL NOT NULL DEFAULT 0,
        title TEXT NOT NULL DEFAULT '',
        artist TEXT NOT NULL DEFAULT '',
        album TEXT NOT NULL DEFAULT '',
        label TEXT NOT NULL DEFAULT '',
        genre TEXT NOT NULL DEFAULT '',
        year INTEGER,
        comment TEXT NOT NULL DEFAULT '',
        tag_bpm REAL,
        tag_key TEXT,
        search TEXT NOT NULL DEFAULT '',
        rating INTEGER NOT NULL DEFAULT 0,
        color INTEGER,
        added_at INTEGER NOT NULL DEFAULT 0,
        last_played INTEGER,
        play_count INTEGER NOT NULL DEFAULT 0,
        missing INTEGER NOT NULL DEFAULT 0
    );
    CREATE INDEX track_hash ON track(hash);
    CREATE TABLE analysis (
        track_id INTEGER PRIMARY KEY REFERENCES track(id) ON DELETE CASCADE,
        version INTEGER NOT NULL,
        locked INTEGER NOT NULL DEFAULT 0,
        bpm REAL NOT NULL,
        bpm_confidence REAL NOT NULL,
        key INTEGER,
        key_confidence REAL NOT NULL,
        tuning_cents REAL NOT NULL,
        lufs REAL NOT NULL,
        lra REAL NOT NULL,
        true_peak REAL NOT NULL,
        bar_offset INTEGER NOT NULL,
        source TEXT NOT NULL
    );
    CREATE TABLE grid_anchor (
        track_id INTEGER NOT NULL REFERENCES track(id) ON DELETE CASCADE,
        idx INTEGER NOT NULL,
        pos REAL NOT NULL,
        bpm REAL NOT NULL,
        PRIMARY KEY (track_id, idx)
    );
    CREATE TABLE playlist (
        id INTEGER PRIMARY KEY,
        parent INTEGER REFERENCES playlist(id) ON DELETE CASCADE,
        name TEXT NOT NULL,
        kind TEXT NOT NULL,
        rules TEXT,
        position INTEGER NOT NULL DEFAULT 0
    );
    CREATE TABLE playlist_item (
        playlist_id INTEGER NOT NULL REFERENCES playlist(id) ON DELETE CASCADE,
        position INTEGER NOT NULL,
        track_id INTEGER NOT NULL REFERENCES track(id) ON DELETE CASCADE,
        PRIMARY KEY (playlist_id, position)
    );",
    // Unvollständige Dateien: weniger Audio als die Kopfdaten ankündigen (BIB-20).
    "ALTER TABLE track ADD COLUMN incomplete INTEGER NOT NULL DEFAULT 0;",
];

pub fn migrate(conn: &Connection) -> rusqlite::Result<()> {
    let version: i64 = conn.query_row("PRAGMA user_version", [], |r| r.get(0))?;
    for (i, sql) in MIGRATIONS.iter().enumerate().skip(version as usize) {
        conn.execute_batch(&format!("BEGIN; {sql} PRAGMA user_version = {}; COMMIT;", i + 1))?;
    }
    Ok(())
}
