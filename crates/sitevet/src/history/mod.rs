//! Run history: a local SQLite file with one row per audit and the report stored as a zstd
//! blob. Behind the `history` feature; the CLI turns it on.

use std::path::{Path, PathBuf};

use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use serde::{Deserialize, Serialize};

use crate::{AuditReport, Error, Result};

/// A run's ID: a SQLite integer, short to type (`sitevet show 12`).
pub type RunId = i64;

/// The report schema this build writes.
const SCHEMA_VERSION: u32 = 1;

/// Applied in order; the database's `PRAGMA user_version` is how many have run.
const MIGRATIONS: &[&str] = &[r#"
    CREATE TABLE runs (
        id              INTEGER PRIMARY KEY AUTOINCREMENT,
        start_url       TEXT    NOT NULL,
        origin          TEXT    NOT NULL,
        started_at      TEXT    NOT NULL,
        duration_ms     INTEGER NOT NULL,
        pages           INTEGER NOT NULL,
        score           INTEGER NOT NULL,
        sitevet_version TEXT    NOT NULL,
        schema_version  INTEGER NOT NULL,
        options_json    TEXT    NOT NULL,
        report_zst      BLOB    NOT NULL
    );
    CREATE INDEX runs_start_url ON runs (start_url);
    CREATE INDEX runs_origin ON runs (origin);
    CREATE INDEX runs_started_at ON runs (started_at);
"#];

/// One row of `history list`.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[non_exhaustive]
pub struct RunSummary {
    pub id: RunId,
    pub start_url: String,
    pub origin: String,
    /// RFC 3339.
    pub started_at: String,
    pub duration_ms: u64,
    pub pages: u32,
    pub score: u8,
    pub sitevet_version: String,
    pub schema_version: u32,
}

/// The run history database.
pub struct History {
    conn: Connection,
    read_only: bool,
    path: PathBuf,
}

fn err(e: impl std::fmt::Display) -> Error {
    Error::History(e.to_string())
}

/// `https://example.com` for any URL on that site; the URL itself when it does not parse.
pub(crate) fn origin_of(url: &str) -> String {
    url::Url::parse(url).map_or_else(|_| url.to_string(), |u| u.origin().ascii_serialization())
}

impl History {
    /// `$SITEVET_HOME/history.db`, else `history.db` in the OS data folder
    /// (`~/Library/Application Support/sitevet`, `~/.local/share/sitevet`, `%APPDATA%\sitevet`).
    pub fn default_path() -> Option<PathBuf> {
        let dir = match std::env::var_os("SITEVET_HOME") {
            Some(home) => PathBuf::from(home),
            None => directories::BaseDirs::new()?.data_dir().join("sitevet"),
        };
        Some(dir.join("history.db"))
    }

    /// Opens or creates the database and applies pending migrations. A database written by a
    /// newer sitevet opens read-only, with a warning.
    pub fn open(path: Option<PathBuf>) -> Result<Self> {
        let path = match path {
            Some(p) => p,
            None => Self::default_path().ok_or_else(|| err("no home folder found; set SITEVET_HOME"))?,
        };
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| err(format!("could not create {}: {e}", dir.display())))?;
        }
        let conn = Connection::open(&path).map_err(err)?;
        let version = conn.pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0)).map_err(err)? as usize;
        if version > MIGRATIONS.len() {
            tracing::warn!(path = %path.display(), "history was written by a newer sitevet; opening it read-only");
            drop(conn);
            let conn = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(err)?;
            return Ok(Self { conn, read_only: true, path });
        }
        for (i, sql) in MIGRATIONS.iter().enumerate().skip(version) {
            let tx = conn.unchecked_transaction().map_err(err)?;
            tx.execute_batch(sql).map_err(err)?;
            tx.pragma_update(None, "user_version", (i + 1) as i64).map_err(err)?;
            tx.commit().map_err(err)?;
        }
        Ok(Self { conn, read_only: false, path })
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    fn writable(&self) -> Result<()> {
        if self.read_only {
            Err(err("the history database belongs to a newer sitevet and is read-only"))
        } else {
            Ok(())
        }
    }

    /// Stores the report and returns its new run ID.
    pub fn save(&self, report: &AuditReport) -> Result<RunId> {
        self.writable()?;
        let json = serde_json::to_vec(report).map_err(err)?;
        let blob = zstd::encode_all(json.as_slice(), 3).map_err(err)?;
        let options = serde_json::to_string(&report.options).map_err(err)?;
        self.conn
            .execute(
                "INSERT INTO runs (start_url, origin, started_at, duration_ms, pages, score, sitevet_version,
                                   schema_version, options_json, report_zst)
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
                params![
                    report.start_url,
                    origin_of(&report.start_url),
                    report.generated_at,
                    report.duration_ms as i64,
                    report.pages.len() as i64,
                    report.score,
                    report.sitevet_version,
                    report.schema_version,
                    options,
                    blob
                ],
            )
            .map_err(err)?;
        Ok(self.conn.last_insert_rowid())
    }

    /// Runs newest first, for one site when `origin` is given (any URL on the site works).
    pub fn list(&self, origin: Option<&str>, limit: usize) -> Result<Vec<RunSummary>> {
        let origin = origin.map(origin_of);
        let mut stmt = self
            .conn
            .prepare(
                "SELECT id, start_url, origin, started_at, duration_ms, pages, score, sitevet_version, schema_version
                 FROM runs WHERE (?1 IS NULL OR origin = ?1) ORDER BY id DESC LIMIT ?2",
            )
            .map_err(err)?;
        let rows = stmt
            .query_map(params![origin, limit as i64], |r| {
                Ok(RunSummary {
                    id: r.get(0)?,
                    start_url: r.get(1)?,
                    origin: r.get(2)?,
                    started_at: r.get(3)?,
                    duration_ms: r.get::<_, i64>(4)? as u64,
                    pages: r.get(5)?,
                    score: r.get(6)?,
                    sitevet_version: r.get(7)?,
                    schema_version: r.get(8)?,
                })
            })
            .map_err(err)?;
        rows.collect::<std::result::Result<_, _>>().map_err(err)
    }

    /// The summary row of one run.
    pub fn summary(&self, id: RunId) -> Result<RunSummary> {
        self.list(None, usize::MAX >> 1)?.into_iter().find(|r| r.id == id).ok_or_else(|| err(format!("no run {id}")))
    }

    /// The stored report. Fails for a run written with a newer report schema; its
    /// [`History::summary`] still reads.
    pub fn load(&self, id: RunId) -> Result<AuditReport> {
        let row: Option<(u32, Vec<u8>)> = self
            .conn
            .query_row("SELECT schema_version, report_zst FROM runs WHERE id = ?1", [id], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .optional()
            .map_err(err)?;
        let (schema, blob) = row.ok_or_else(|| err(format!("no run {id}")))?;
        if schema > SCHEMA_VERSION {
            return Err(err(format!(
                "run {id} uses report schema {schema}, newer than this sitevet reads ({SCHEMA_VERSION})"
            )));
        }
        let json = zstd::decode_all(blob.as_slice()).map_err(err)?;
        serde_json::from_slice(&json).map_err(err)
    }

    /// Removes one run. False when there was no such run.
    pub fn delete(&self, id: RunId) -> Result<bool> {
        self.writable()?;
        Ok(self.conn.execute("DELETE FROM runs WHERE id = ?1", [id]).map_err(err)? > 0)
    }

    /// Keeps the newest `keep` runs and removes the rest. Returns how many were removed.
    pub fn prune(&self, keep: usize) -> Result<usize> {
        self.writable()?;
        self.conn
            .execute("DELETE FROM runs WHERE id NOT IN (SELECT id FROM runs ORDER BY id DESC LIMIT ?1)", [keep as i64])
            .map_err(err)
    }

    /// The newest run for a site (any URL on it), for `--baseline last`.
    pub fn latest_for(&self, url: &str) -> Result<Option<RunId>> {
        self.conn.query_row("SELECT max(id) FROM runs WHERE origin = ?1", [origin_of(url)], |r| r.get(0)).map_err(err)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report(url: &str, score: u8) -> AuditReport {
        let json = format!(
            r#"{{"schema_version":1,"sitevet_version":"0.2.0","start_url":"{url}","final_url":"{url}","generated_at":"2026-11-10T09:30:00Z",
                "duration_ms":1200,"options":{{}},"score":{score},"categories":[],"pages":[],"blocked":[]}}"#
        );
        serde_json::from_str(&json).unwrap()
    }

    fn temp_db(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("sitevet-history-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir.join("history.db")
    }

    #[test]
    fn save_list_load_delete_prune() {
        let path = temp_db("crud");
        let h = History::open(Some(path.clone())).unwrap();
        let a = h.save(&report("https://a.test/", 80)).unwrap();
        let b = h.save(&report("https://b.test/x", 70)).unwrap();
        let c = h.save(&report("https://a.test/blog", 90)).unwrap();
        assert_eq!(h.list(None, 10).unwrap().iter().map(|r| r.id).collect::<Vec<_>>(), [c, b, a], "newest first");
        assert_eq!(h.list(Some("https://a.test/anything"), 10).unwrap().len(), 2);
        assert_eq!(h.latest_for("https://a.test/").unwrap(), Some(c));
        assert_eq!(h.latest_for("https://none.test/").unwrap(), None);
        assert_eq!(h.load(b).unwrap().score, 70);
        assert!(h.load(999).is_err());
        assert!(h.delete(b).unwrap());
        assert!(!h.delete(b).unwrap());
        let d = h.save(&report("https://a.test/", 60)).unwrap();
        assert_eq!(h.prune(2).unwrap(), 1);
        assert_eq!(h.list(None, 10).unwrap().iter().map(|r| r.id).collect::<Vec<_>>(), [d, c]);
        drop(h);
        // Reopening keeps the data and does not rerun migrations.
        assert_eq!(History::open(Some(path)).unwrap().list(None, 10).unwrap().len(), 2);
    }

    #[test]
    fn newer_database_opens_read_only() {
        let path = temp_db("newer");
        History::open(Some(path.clone())).unwrap().save(&report("https://a.test/", 80)).unwrap();
        Connection::open(&path).unwrap().pragma_update(None, "user_version", 99).unwrap();
        let h = History::open(Some(path)).unwrap();
        assert_eq!(h.list(None, 10).unwrap().len(), 1);
        assert!(h.save(&report("https://a.test/", 80)).unwrap_err().to_string().contains("read-only"));
    }
}
