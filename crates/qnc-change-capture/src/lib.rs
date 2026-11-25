//! Catches the rows one write changes (the journal of the project, user 2026-11-25: "a quasi git
//! that remembers every action from the start of the edit to its end"). One thing only: on one
//! connection, for the tables it is given, every inserted, updated or deleted row is kept as it
//! was before and as it is after (JSON of all its columns), until [`take`] hands them over.
//!
//! The triggers are TEMP: they exist only on that connection and nothing else that writes the
//! same database sees them. The rows go to a TEMP table, in the same transaction as the write,
//! so a write rolled back leaves none. Nothing of the project is written here.

use rusqlite::{params, Connection};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MODULE_ID: &str = "qnc.module.change-capture";

/// The TEMP table the caught rows wait in (the writer of the connection must be let write it).
pub const PENDING: &str = "qnc_change_pending";

pub type Result<T> = std::result::Result<T, String>;

/// One row a write changed: its table, what happened and the row before and after.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RowChange {
    pub table: String,
    /// `insert`, `update` or `delete`.
    pub op: String,
    /// The row before (`Null` for an insert).
    pub before: Value,
    /// The row after (`Null` for a delete).
    pub after: Value,
}

/// Starts catching the changes of `tables` on `conn` (again on every new connection). A table
/// that does not exist is passed over.
pub fn install(conn: &Connection, tables: &[&str]) -> Result<()> {
    conn.execute_batch(&format!("CREATE TEMP TABLE IF NOT EXISTS {PENDING} (seq INTEGER PRIMARY KEY, tbl TEXT NOT NULL, op TEXT NOT NULL, before_row TEXT, after_row TEXT);"))
        .map_err(err)?;
    for table in tables {
        let columns = columns(conn, table)?;
        if columns.is_empty() {
            continue;
        }
        let row = |side: &str| {
            let pairs = columns.iter().map(|column| format!("'{column}', CASE typeof({side}.\"{column}\") WHEN 'blob' THEN 'x:' || hex({side}.\"{column}\") ELSE {side}.\"{column}\" END")).collect::<Vec<_>>();
            format!("json_object({})", pairs.join(", "))
        };
        let (old, new) = (row("OLD"), row("NEW"));
        conn.execute_batch(&format!(
            "CREATE TEMP TRIGGER IF NOT EXISTS qnc_cc_{table}_i AFTER INSERT ON main.\"{table}\" BEGIN INSERT INTO {PENDING} (tbl, op, before_row, after_row) VALUES ('{table}', 'insert', NULL, {new}); END;
             CREATE TEMP TRIGGER IF NOT EXISTS qnc_cc_{table}_u AFTER UPDATE ON main.\"{table}\" BEGIN INSERT INTO {PENDING} (tbl, op, before_row, after_row) VALUES ('{table}', 'update', {old}, {new}); END;
             CREATE TEMP TRIGGER IF NOT EXISTS qnc_cc_{table}_d AFTER DELETE ON main.\"{table}\" BEGIN INSERT INTO {PENDING} (tbl, op, before_row, after_row) VALUES ('{table}', 'delete', {old}, NULL); END;"
        ))
        .map_err(err)?;
    }
    Ok(())
}

/// The rows caught since the last call, in the order they changed; they are gone from the
/// connection after it. An update that changed nothing is left out.
pub fn take(conn: &Connection) -> Result<Vec<RowChange>> {
    let mut statement = conn.prepare(&format!("SELECT tbl, op, before_row, after_row FROM temp.{PENDING} ORDER BY seq")).map_err(err)?;
    let parse = |text: Option<String>| text.and_then(|text| serde_json::from_str(&text).ok()).unwrap_or(Value::Null);
    let rows = statement
        .query_map([], |row| Ok(RowChange { table: row.get(0)?, op: row.get(1)?, before: parse(row.get(2)?), after: parse(row.get(3)?) }))
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(err)?;
    conn.execute(&format!("DELETE FROM temp.{PENDING}"), params![]).map_err(err)?;
    Ok(rows.into_iter().filter(|change| change.op != "update" || change.before != change.after).collect())
}

/// The rows caught since the last [`take`], left for it (what the write so far changed).
pub fn peek(conn: &Connection) -> Result<Vec<RowChange>> {
    let pending: bool = conn.query_row(&format!("SELECT EXISTS(SELECT 1 FROM sqlite_temp_master WHERE type = 'table' AND name = '{PENDING}')"), [], |row| row.get(0)).map_err(err)?;
    if !pending {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare(&format!("SELECT tbl, op, before_row, after_row FROM temp.{PENDING} ORDER BY seq")).map_err(err)?;
    let parse = |text: Option<String>| text.and_then(|text| serde_json::from_str(&text).ok()).unwrap_or(Value::Null);
    let rows = statement
        .query_map([], |row| Ok(RowChange { table: row.get(0)?, op: row.get(1)?, before: parse(row.get(2)?), after: parse(row.get(3)?) }))
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(err)?;
    Ok(rows)
}

/// The column names of `table` in the main database (none when it does not exist).
fn columns(conn: &Connection, table: &str) -> Result<Vec<String>> {
    if !table.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("not a table name: {table}"));
    }
    let mut statement = conn.prepare("SELECT name FROM pragma_table_info(?1, 'main')").map_err(err)?;
    let names = statement.query_map([table], |row| row.get::<_, String>(0)).map_err(err)?.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
    Ok(names)
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The rows a write changes, before and after, until they are taken; a rolled back write
    /// leaves none and an update that changed nothing is left out.
    #[test]
    fn the_rows_a_write_changes_are_caught_before_and_after() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (id TEXT PRIMARY KEY, v INTEGER, b BLOB);").unwrap();
        install(&conn, &["t", "missing"]).unwrap();
        conn.execute("INSERT INTO t VALUES ('a', 1, x'01ff')", []).unwrap();
        conn.execute("UPDATE t SET v = 2 WHERE id = 'a'", []).unwrap();
        conn.execute("UPDATE t SET v = 2 WHERE id = 'a'", []).unwrap();
        let changes = take(&conn).unwrap();
        assert_eq!(changes.iter().map(|change| change.op.as_str()).collect::<Vec<_>>(), ["insert", "update"]);
        assert_eq!(changes[1].before["v"], 1);
        assert_eq!(changes[1].after["v"], 2);
        assert_eq!(changes[0].after["b"], "x:01FF");
        assert!(take(&conn).unwrap().is_empty(), "taken once");
        conn.execute_batch("BEGIN; DELETE FROM t; ROLLBACK;").unwrap();
        assert!(take(&conn).unwrap().is_empty(), "a rolled back write leaves none");
        conn.execute("DELETE FROM t", []).unwrap();
        let deleted = take(&conn).unwrap();
        assert_eq!((deleted[0].op.as_str(), &deleted[0].after), ("delete", &Value::Null));
    }

    /// Another connection to the same database does not catch anything (TEMP triggers).
    #[test]
    fn another_connection_catches_nothing() {
        let dir = std::env::temp_dir().join(format!("qnc-change-capture-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join("t.db");
        let first = Connection::open(&file).unwrap();
        first.execute_batch("CREATE TABLE IF NOT EXISTS t (id TEXT PRIMARY KEY, v INTEGER);").unwrap();
        install(&first, &["t"]).unwrap();
        let second = Connection::open(&file).unwrap();
        second.execute("INSERT INTO t VALUES ('x', 1)", []).unwrap();
        assert!(take(&first).unwrap().is_empty());
        drop((first, second));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
