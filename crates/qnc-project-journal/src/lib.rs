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
    /// Its action (the kind of the piece's request; `undo` or `redo` for a step of undo).
    pub action: String,
    /// A step the piece's Undo may take back (a change of the user's work, not a recording).
    #[serde(default)]
    pub undoable: bool,
    /// The commit an `undo` took back or a `redo` made again.
    #[serde(default)]
    pub refers: Option<i64>,
    pub changes: Vec<RowChange>,
}

/// A step of undo a piece made: the commit it took back or made again.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Step {
    Undo(i64),
    Redo(i64),
    /// The piece taken back to the moment right after a commit (user 2026-11-25: going back to a
    /// moment of the project); Undo may take it back again.
    Restore(i64),
}

impl Step {
    pub fn action(self) -> &'static str {
        match self {
            Self::Undo(_) => "undo",
            Self::Redo(_) => "redo",
            Self::Restore(_) => "restore",
        }
    }
    pub fn refers(self) -> i64 {
        match self {
            Self::Undo(id) | Self::Redo(id) | Self::Restore(id) => id,
        }
    }
    /// A going back to a moment is a step of the piece's Undo; an undo or a redo is not.
    pub fn undoable(self) -> bool {
        matches!(self, Self::Restore(_))
    }
}

/// What a piece's Undo and Redo would take: the commits Undo may take back (the newest last) and
/// the ones it took back that Redo may make again (the last taken last).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Stacks {
    pub undo: Vec<i64>,
    pub redo: Vec<i64>,
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
    pub undoable: bool,
    pub refers: Option<i64>,
    pub changes: Vec<RowChange>,
}

/// Creates the journal's tables when missing (on a connection that may create tables); a journal
/// of the first day gets the columns of undo.
pub fn ensure_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS project_journal (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            made_at_utc TEXT NOT NULL,
            author TEXT NOT NULL,
            module TEXT NOT NULL,
            action TEXT NOT NULL,
            undoable INTEGER NOT NULL DEFAULT 0,
            refers INTEGER
        );
        CREATE TABLE IF NOT EXISTS project_journal_changes (
            commit_id INTEGER NOT NULL REFERENCES project_journal(id),
            seq INTEGER NOT NULL,
            tbl TEXT NOT NULL,
            op TEXT NOT NULL CHECK (op IN ('insert', 'update', 'delete')),
            before_row TEXT,
            after_row TEXT,
            PRIMARY KEY (commit_id, seq)
        );",
    )
    .map_err(err)?;
    let columns: Vec<String> = {
        let mut statement = conn.prepare("SELECT name FROM pragma_table_info('project_journal')").map_err(err)?;
        let names = statement.query_map([], |row| row.get(0)).map_err(err)?;
        names.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?
    };
    if !columns.iter().any(|column| column == "undoable") {
        conn.execute_batch("ALTER TABLE project_journal ADD COLUMN undoable INTEGER NOT NULL DEFAULT 0; ALTER TABLE project_journal ADD COLUMN refers INTEGER;").map_err(err)?;
    }
    conn.execute_batch(
        "CREATE INDEX IF NOT EXISTS project_journal_module ON project_journal (module, id);
        DROP VIEW IF EXISTS public_project_journal;
        CREATE VIEW public_project_journal AS
            SELECT id, made_at_utc, author, module, action, undoable, refers FROM project_journal;",
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
        "INSERT INTO project_journal (made_at_utc, author, module, action, undoable, refers) VALUES (strftime('%Y-%m-%dT%H:%M:%fZ','now'), ?1, ?2, ?3, ?4, ?5)",
        params![commit.author, commit.module, commit.action, commit.undoable, commit.refers],
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

const HEAD: &str = "SELECT id, made_at_utc, author, module, action, undoable, refers FROM project_journal";

fn head(row: &rusqlite::Row<'_>) -> rusqlite::Result<Commit> {
    Ok(Commit { id: row.get(0)?, made_at_utc: row.get(1)?, author: row.get(2)?, module: row.get(3)?, action: row.get(4)?, undoable: row.get(5)?, refers: row.get(6)?, changes: Vec::new() })
}

fn parse(text: Option<String>) -> serde_json::Value {
    text.and_then(|text| serde_json::from_str(&text).ok()).unwrap_or(serde_json::Value::Null)
}

fn changes_of(conn: &Connection, id: i64) -> Result<Vec<RowChange>> {
    let mut rows = conn.prepare_cached("SELECT tbl, op, before_row, after_row FROM project_journal_changes WHERE commit_id = ?1 ORDER BY seq").map_err(err)?;
    let changes = rows
        .query_map([id], |row| Ok(RowChange { table: row.get(0)?, op: row.get(1)?, before: parse(row.get(2)?), after: parse(row.get(3)?) }))
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(err)?;
    Ok(changes)
}

/// The newest `limit` commits, the newest first, with their changes.
pub fn read(conn: &Connection, limit: usize) -> Result<Vec<Commit>> {
    if !ready(conn) {
        return Ok(Vec::new());
    }
    let mut heads = conn.prepare(&format!("{HEAD} ORDER BY id DESC LIMIT ?1")).map_err(err)?;
    let mut commits = heads.query_map([limit as i64], head).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    for commit in &mut commits {
        commit.changes = changes_of(conn, commit.id)?;
    }
    Ok(commits)
}

/// One commit with its changes (`None`: there is none of that number).
pub fn commit(conn: &Connection, id: i64) -> Result<Option<Commit>> {
    if !ready(conn) {
        return Ok(None);
    }
    let mut statement = conn.prepare(&format!("{HEAD} WHERE id = ?1")).map_err(err)?;
    let Some(mut found) = statement.query_map([id], head).map_err(err)?.next().transpose().map_err(err)? else { return Ok(None) };
    found.changes = changes_of(conn, id)?;
    Ok(Some(found))
}

/// The Undo and Redo of `module`, from its commits in their order: an undoable commit goes onto
/// Undo and empties Redo; an `undo` moves the commit it took back onto Redo; a `redo` moves it
/// back onto Undo. A commit that is neither (a recording) changes neither.
pub fn stacks(conn: &Connection, module: &str) -> Result<Stacks> {
    let mut stacks = Stacks::default();
    if !ready(conn) {
        return Ok(stacks);
    }
    let mut statement = conn.prepare("SELECT id, action, undoable, refers FROM project_journal WHERE module = ?1 ORDER BY id").map_err(err)?;
    let rows = statement.query_map([module], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, bool>(2)?, row.get::<_, Option<i64>>(3)?))).map_err(err)?;
    for row in rows {
        let (id, action, undoable, refers) = row.map_err(err)?;
        match (action.as_str(), refers) {
            ("undo", Some(target)) => {
                stacks.undo.retain(|kept| *kept != target);
                stacks.redo.push(target);
            }
            ("redo", Some(target)) => {
                stacks.redo.retain(|kept| *kept != target);
                stacks.undo.push(target);
            }
            _ if undoable => {
                stacks.undo.push(id);
                stacks.redo.clear();
            }
            _ => {}
        }
    }
    Ok(stacks)
}

/// The commits of `module` after the commit `to` (all of them for `0`), the oldest first, with
/// their changes: what going back to the moment right after `to` takes back.
pub fn after(conn: &Connection, module: &str, to: i64) -> Result<Vec<Commit>> {
    if !ready(conn) {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare(&format!("{HEAD} WHERE module = ?1 AND id > ?2 ORDER BY id")).map_err(err)?;
    let mut commits = statement.query_map(params![module, to], head).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    for commit in &mut commits {
        commit.changes = changes_of(conn, commit.id)?;
    }
    Ok(commits)
}

/// The pieces with commits after the commit `to`.
pub fn modules_after(conn: &Connection, to: i64) -> Result<Vec<String>> {
    if !ready(conn) {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare("SELECT DISTINCT module FROM project_journal WHERE id > ?1 ORDER BY module").map_err(err)?;
    let modules = statement.query_map([to], |row| row.get(0)).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    Ok(modules)
}

/// The newest `limit` commits without their changes, the newest first, the time of each as the
/// computer reading it keeps local time (`YYYY-MM-DD HH:MM:SS`): the moments of the project.
pub fn moments(conn: &Connection, limit: usize) -> Result<Vec<Commit>> {
    if !ready(conn) {
        return Ok(Vec::new());
    }
    let mut statement = conn
        .prepare("SELECT id, COALESCE(datetime(made_at_utc, 'localtime'), made_at_utc), author, module, action, undoable, refers FROM project_journal ORDER BY id DESC LIMIT ?1")
        .map_err(err)?;
    let commits = statement.query_map([limit as i64], head).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    Ok(commits)
}

/// The history of one row: the newest `limit` commits that changed a row of `table` whose column
/// `key` is `value` (before or after), the oldest first, each with only the changes of that row.
pub fn row_history(conn: &Connection, table: &str, key: &str, value: &str, limit: usize) -> Result<Vec<Commit>> {
    if !ready(conn) {
        return Ok(Vec::new());
    }
    if !key.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("not a column name: {key}"));
    }
    let path = format!("$.{key}");
    let mut statement = conn
        .prepare(
            "SELECT j.id, j.made_at_utc, j.author, j.module, j.action, j.undoable, j.refers, c.op, c.before_row, c.after_row
             FROM project_journal_changes c JOIN project_journal j ON j.id = c.commit_id
             WHERE c.commit_id IN (
                 SELECT DISTINCT commit_id FROM project_journal_changes
                 WHERE tbl = ?1 AND (json_extract(after_row, ?2) = ?3 OR json_extract(before_row, ?2) = ?3)
                 ORDER BY commit_id DESC LIMIT ?4)
               AND c.tbl = ?1 AND (json_extract(c.after_row, ?2) = ?3 OR json_extract(c.before_row, ?2) = ?3)
             ORDER BY c.commit_id, c.seq",
        )
        .map_err(err)?;
    let rows = statement
        .query_map(params![table, path, value, limit as i64], |row| {
            let head = head(row)?;
            Ok((head, RowChange { table: table.to_string(), op: row.get(7)?, before: parse(row.get(8)?), after: parse(row.get(9)?) }))
        })
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(err)?;
    let mut commits: Vec<Commit> = Vec::new();
    for (head, change) in rows {
        match commits.last_mut() {
            Some(last) if last.id == head.id => last.changes.push(change),
            _ => commits.push(Commit { changes: vec![change], ..head }),
        }
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
        let commit = NewCommit { author: "miron@laptop".into(), module: "qnc.module.x".into(), action: "create".into(), undoable: true, refers: None, changes };
        assert!(append(&conn, &commit).unwrap().is_some());
        assert_eq!(append(&conn, &NewCommit { changes: Vec::new(), ..commit.clone() }).unwrap(), None);
        let read = read(&conn, 10).unwrap();
        assert_eq!(read.len(), 1);
        assert_eq!((read[0].author.as_str(), read[0].action.as_str(), read[0].changes[0].after["v"].as_i64()), ("miron@laptop", "create", Some(1)));
    }

    /// The history of one row: only the commits and changes of that row, the oldest first.
    #[test]
    fn the_history_of_one_row_is_its_commits_the_oldest_first() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (id TEXT PRIMARY KEY, v INTEGER);").unwrap();
        ensure_schema(&conn).unwrap();
        qnc_change_capture::install(&conn, &["t"]).unwrap();
        let commit = |sql: &str| {
            conn.execute_batch(sql).unwrap();
            let changes = qnc_change_capture::take(&conn).unwrap();
            append(&conn, &NewCommit { author: "u".into(), module: "m".into(), action: "set".into(), undoable: true, refers: None, changes }).unwrap();
        };
        commit("INSERT INTO t VALUES ('a', 1); INSERT INTO t VALUES ('b', 9);");
        commit("UPDATE t SET v = 2 WHERE id = 'a';");
        commit("UPDATE t SET v = 8 WHERE id = 'b';");
        let a = row_history(&conn, "t", "id", "a", 10).unwrap();
        assert_eq!(a.iter().map(|commit| commit.changes.iter().map(|change| change.after["v"].as_i64().unwrap()).collect::<Vec<_>>()).collect::<Vec<_>>(), [vec![1], vec![2]]);
        assert_eq!(row_history(&conn, "t", "id", "a", 1).unwrap()[0].changes[0].after["v"], 2, "the newest ones");
    }

    /// Undo takes the newest undoable commit back onto Redo, Redo makes it again, a new edit
    /// empties Redo; a recording (not undoable) is on neither.
    #[test]
    fn undo_and_redo_follow_the_order_of_the_commits() {
        let conn = Connection::open_in_memory().unwrap();
        ensure_schema(&conn).unwrap();
        let change = vec![RowChange { table: "t".into(), op: "insert".into(), before: serde_json::Value::Null, after: serde_json::json!({ "id": 1 }) }];
        let add = |action: &str, undoable: bool, refers: Option<i64>| {
            append(&conn, &NewCommit { author: "u".into(), module: "m".into(), action: action.into(), undoable, refers, changes: change.clone() }).unwrap().unwrap()
        };
        let (a, b) = (add("edit", true, None), add("edit", true, None));
        add("record", false, None);
        assert_eq!(stacks(&conn, "m").unwrap(), Stacks { undo: vec![a, b], redo: vec![] });
        add("undo", false, Some(b));
        assert_eq!(stacks(&conn, "m").unwrap(), Stacks { undo: vec![a], redo: vec![b] });
        add("redo", false, Some(b));
        assert_eq!(stacks(&conn, "m").unwrap(), Stacks { undo: vec![a, b], redo: vec![] });
        add("undo", false, Some(b));
        let c = add("edit", true, None);
        assert_eq!(stacks(&conn, "m").unwrap(), Stacks { undo: vec![a, c], redo: vec![] }, "a new edit empties Redo");
        assert!(stacks(&conn, "other").unwrap().undo.is_empty(), "per piece");
        assert_eq!(commit(&conn, c).unwrap().unwrap().changes.len(), 1);
    }
}
