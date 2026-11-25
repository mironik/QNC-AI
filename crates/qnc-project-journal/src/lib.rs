//! The journal of a project (user 2026-11-25: "a quasi git that remembers every action from the
//! start of the edit to its end, for the whole application"; "the project belongs to the user,
//! nobody but the user can delete"). One thing only: it keeps commits. A commit is one write of an
//! edit: when, who (the user and the computer), which piece wrote it and its action, and the rows
//! it changed before and after (piece `qnc-change-capture`). It only appends and reads; going
//! back to a commit is a new commit made by its owner, so nothing is ever lost here.
//!
//! The journal lives in the database the edit is written to (the project's, or a draft story's
//! of an S-Story version), in two tables of its own.

use qnc_change_capture::RowChange;
use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};

pub const MODULE_ID: &str = "qnc.module.project-journal";

/// The tables of the journal (the writer of an edit must be let append to them).
pub const TABLES: [&str; 2] = ["project_journal", "project_journal_changes"];

pub type Result<T> = std::result::Result<T, String>;

/// A commit to append.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewCommit {
    /// Who made it: the user and the computer.
    pub author: String,
    /// The piece that wrote it (its module id).
    pub module: String,
    /// Its action (the kind of the piece's request).
    pub action: String,
    pub changes: Vec<RowChange>,
}

/// A commit of the journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Commit {
    pub id: i64,
    pub made_at_utc: String,
    pub author: String,
    pub module: String,
    pub action: String,
    pub changes: Vec<RowChange>,
}

/// Creates the journal's tables when missing (on a connection that may create tables).
pub fn ensure_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS project_journal (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            made_at_utc TEXT NOT NULL,
            author TEXT NOT NULL,
            module TEXT NOT NULL,
            action TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS project_journal_changes (
            commit_id INTEGER NOT NULL REFERENCES project_journal(id),
            seq INTEGER NOT NULL,
            tbl TEXT NOT NULL,
            op TEXT NOT NULL CHECK (op IN ('insert', 'update', 'delete')),
            before_row TEXT,
            after_row TEXT,
            PRIMARY KEY (commit_id, seq)
        );
        CREATE VIEW IF NOT EXISTS public_project_journal AS
            SELECT id, made_at_utc, author, module, action FROM project_journal;",
    )
    .map_err(err)
}

/// Whether the journal's tables are there.
pub fn ready(conn: &Connection) -> bool {
    conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='project_journal')", [], |row| row.get(0)).unwrap_or(false)
}

/// Appends `commit` (with no changes, nothing); its number.
pub fn append(conn: &Connection, commit: &NewCommit) -> Result<Option<i64>> {
    if commit.changes.is_empty() {
        return Ok(None);
    }
    conn.execute(
        "INSERT INTO project_journal (made_at_utc, author, module, action) VALUES (strftime('%Y-%m-%dT%H:%M:%fZ','now'), ?1, ?2, ?3)",
        params![commit.author, commit.module, commit.action],
    )
    .map_err(err)?;
    let id = conn.last_insert_rowid();
    let text = |value: &serde_json::Value| (!value.is_null()).then(|| value.to_string());
    for (seq, change) in commit.changes.iter().enumerate() {
        conn.execute(
            "INSERT INTO project_journal_changes (commit_id, seq, tbl, op, before_row, after_row) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![id, seq as i64, change.table, change.op, text(&change.before), text(&change.after)],
        )
        .map_err(err)?;
    }
    Ok(Some(id))
}

/// The newest `limit` commits, the newest first, with their changes.
pub fn read(conn: &Connection, limit: usize) -> Result<Vec<Commit>> {
    if !ready(conn) {
        return Ok(Vec::new());
    }
    let mut heads = conn.prepare("SELECT id, made_at_utc, author, module, action FROM project_journal ORDER BY id DESC LIMIT ?1").map_err(err)?;
    let mut commits = heads
        .query_map([limit as i64], |row| Ok(Commit { id: row.get(0)?, made_at_utc: row.get(1)?, author: row.get(2)?, module: row.get(3)?, action: row.get(4)?, changes: Vec::new() }))
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(err)?;
    let mut rows = conn.prepare("SELECT tbl, op, before_row, after_row FROM project_journal_changes WHERE commit_id = ?1 ORDER BY seq").map_err(err)?;
    let parse = |text: Option<String>| text.and_then(|text| serde_json::from_str(&text).ok()).unwrap_or(serde_json::Value::Null);
    for commit in &mut commits {
        commit.changes = rows
            .query_map([commit.id], |row| Ok(RowChange { table: row.get(0)?, op: row.get(1)?, before: parse(row.get(2)?), after: parse(row.get(3)?) }))
            .map_err(err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(err)?;
    }
    Ok(commits)
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows an edit changed are one commit, with who and what; a write that changed nothing
    /// is no commit.
    #[test]
    fn the_rows_an_edit_changed_are_one_commit() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (id TEXT PRIMARY KEY, v INTEGER);").unwrap();
        ensure_schema(&conn).unwrap();
        qnc_change_capture::install(&conn, &["t"]).unwrap();
        conn.execute("INSERT INTO t VALUES ('a', 1)", []).unwrap();
        let changes = qnc_change_capture::take(&conn).unwrap();
        let commit = NewCommit { author: "miron@laptop".into(), module: "qnc.module.x".into(), action: "create".into(), changes };
        assert!(append(&conn, &commit).unwrap().is_some());
        assert_eq!(append(&conn, &NewCommit { changes: Vec::new(), ..commit.clone() }).unwrap(), None);
        let read = read(&conn, 10).unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!((read[0].author.as_str(), read[0].action.as_str(), read[0].changes[0].after["v"].as_i64()), ("miron@laptop", "create", Some(1)));
    }
}
