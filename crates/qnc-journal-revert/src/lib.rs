//! Takes a commit of the project's journal back, or makes it again (user 2026-11-25: Undo and
//! Redo of the story and the script move to the journal). One thing only: on one connection,
//! every row a commit changed is put back as it was before it (or as it was after it), in the
//! order that keeps no stale row behind. A row someone changed since the commit (another edit,
//! another computer) is not overwritten: the whole step is refused, the caller's transaction
//! takes nothing. Columns a piece calls loose (what is shown, not what is edited: a selection,
//! a recording, a stamp) are neither checked nor put back.

use qnc_change_capture::RowChange;
use rusqlite::{types::Value as Sql, Connection};
use serde_json::{Map, Value};

pub type Result<T> = std::result::Result<T, String>;

/// The columns of a table that a step neither checks nor puts back.
pub type Loose<'a> = &'a [(&'a str, &'a [&'a str])];

/// Takes `changes` back: the newest first, each row back to its `before`.
pub fn revert(conn: &Connection, changes: &[RowChange], loose: Loose<'_>) -> Result<()> {
    for change in changes.iter().rev() {
        step(conn, &change.table, &change.after, &change.before, loose)?;
    }
    Ok(())
}

/// Makes `changes` again: the oldest first, each row to its `after`.
pub fn reapply(conn: &Connection, changes: &[RowChange], loose: Loose<'_>) -> Result<()> {
    for change in changes {
        step(conn, &change.table, &change.before, &change.after, loose)?;
    }
    Ok(())
}

/// The row of `table` whose key is the key in `keyed` (any row of it), as the journal keeps rows.
pub fn current(conn: &Connection, table: &str, keyed: &Value) -> Result<Option<Value>> {
    let table = Table::read(conn, table)?;
    table.row(conn, keyed)
}

/// Puts the row of `table` whose key is in `keyed` to `row` (`None`: it goes).
pub fn put(conn: &Connection, table: &str, keyed: &Value, row: Option<&Value>) -> Result<()> {
    let table = Table::read(conn, table)?;
    table.delete(conn, keyed)?;
    if let Some(row) = row {
        table.insert(conn, row)?;
    }
    Ok(())
}

/// One row from `from` (what it must be now; `Null`: not there) to `to` (`Null`: it goes).
fn step(conn: &Connection, table: &str, from: &Value, to: &Value, loose: Loose<'_>) -> Result<()> {
    let table = Table::read(conn, table)?;
    let loose: &[&str] = loose.iter().find(|(name, _)| *name == table.name).map_or(&[], |(_, columns)| columns);
    let keyed = if from.is_null() { to } else { from };
    let now = table.row(conn, keyed)?;
    let same = match (&now, from) {
        (None, Value::Null) => true,
        (Some(now), from) if !from.is_null() => {
            table.columns.iter().filter(|column| !loose.contains(&column.name.as_str())).all(|column| now.get(&column.name) == from.get(&column.name))
        }
        _ => false,
    };
    if !same {
        return Err(qnc_ui_text::fill("journal.changed_since", &[("table", &table.name)]));
    }
    match (now, to) {
        (None, Value::Null) => Ok(()),
        (Some(_), Value::Null) => table.delete(conn, keyed),
        (None, to) => table.insert(conn, to),
        (Some(_), to) => table.update(conn, to, loose),
    }
}

struct Column {
    name: String,
    blob: bool,
    key: bool,
}

struct Table {
    name: String,
    columns: Vec<Column>,
}

impl Table {
    fn read(conn: &Connection, name: &str) -> Result<Self> {
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
            return Err(format!("not a table name: {name}"));
        }
        let mut statement = conn.prepare("SELECT name, type, pk FROM pragma_table_info(?1, 'main') ORDER BY cid").map_err(err)?;
        let columns = statement
            .query_map([name], |row| {
                let kind: String = row.get(1)?;
                Ok(Column { name: row.get(0)?, blob: kind.to_ascii_uppercase().contains("BLOB"), key: row.get::<_, i64>(2)? > 0 })
            })
            .map_err(err)?
            .collect::<rusqlite::Result<Vec<_>>>()
            .map_err(err)?;
        if !columns.iter().any(|column| column.key) {
            return Err(format!("no key of the table {name}"));
        }
        Ok(Self { name: name.to_string(), columns })
    }

    fn keys(&self) -> impl Iterator<Item = &Column> {
        self.columns.iter().filter(|column| column.key)
    }

    fn key_values(&self, keyed: &Value) -> Vec<Sql> {
        self.keys().map(|column| sql(column, keyed.get(&column.name).unwrap_or(&Value::Null))).collect()
    }

    fn key_where(&self, from: usize) -> String {
        self.keys().enumerate().map(|(at, column)| format!("\"{}\" IS ?{}", column.name, at + from)).collect::<Vec<_>>().join(" AND ")
    }

    fn row(&self, conn: &Connection, keyed: &Value) -> Result<Option<Value>> {
        let pairs = self
            .columns
            .iter()
            .map(|column| format!("'{0}', CASE typeof(\"{0}\") WHEN 'blob' THEN 'x:' || hex(\"{0}\") ELSE \"{0}\" END", column.name))
            .collect::<Vec<_>>();
        let query = format!("SELECT json_object({}) FROM main.\"{}\" WHERE {}", pairs.join(", "), self.name, self.key_where(1));
        let mut statement = conn.prepare(&query).map_err(err)?;
        let mut rows = statement.query(rusqlite::params_from_iter(self.key_values(keyed))).map_err(err)?;
        let Some(row) = rows.next().map_err(err)? else { return Ok(None) };
        let text: String = row.get(0).map_err(err)?;
        serde_json::from_str(&text).map(Some).map_err(err)
    }

    fn delete(&self, conn: &Connection, keyed: &Value) -> Result<()> {
        conn.execute(&format!("DELETE FROM main.\"{}\" WHERE {}", self.name, self.key_where(1)), rusqlite::params_from_iter(self.key_values(keyed))).map_err(err)?;
        Ok(())
    }

    fn insert(&self, conn: &Connection, row: &Value) -> Result<()> {
        let Some(map) = row.as_object() else { return Err(format!("not a row of {}", self.name)) };
        let columns: Vec<&Column> = self.columns.iter().filter(|column| map.contains_key(&column.name)).collect();
        let names = columns.iter().map(|column| format!("\"{}\"", column.name)).collect::<Vec<_>>().join(", ");
        let marks = (1..=columns.len()).map(|at| format!("?{at}")).collect::<Vec<_>>().join(", ");
        let values = columns.iter().map(|column| sql(column, &map[&column.name])).collect::<Vec<_>>();
        conn.execute(&format!("INSERT INTO main.\"{}\" ({names}) VALUES ({marks})", self.name), rusqlite::params_from_iter(values)).map_err(err)?;
        Ok(())
    }

    fn update(&self, conn: &Connection, row: &Value, loose: &[&str]) -> Result<()> {
        let map: &Map<String, Value> = row.as_object().ok_or_else(|| format!("not a row of {}", self.name))?;
        let set: Vec<&Column> = self.columns.iter().filter(|column| !column.key && !loose.contains(&column.name.as_str()) && map.contains_key(&column.name)).collect();
        if set.is_empty() {
            return Ok(());
        }
        let assigns = set.iter().enumerate().map(|(at, column)| format!("\"{}\" = ?{}", column.name, at + 1)).collect::<Vec<_>>().join(", ");
        let mut values: Vec<Sql> = set.iter().map(|column| sql(column, &map[&column.name])).collect();
        values.extend(self.key_values(row));
        conn.execute(&format!("UPDATE main.\"{}\" SET {assigns} WHERE {}", self.name, self.key_where(set.len() + 1)), rusqlite::params_from_iter(values)).map_err(err)?;
        Ok(())
    }
}

/// A value as the journal keeps it, back into the database (a BLOB column's `x:` hex is bytes).
fn sql(column: &Column, value: &Value) -> Sql {
    match value {
        Value::Null => Sql::Null,
        Value::Bool(on) => Sql::Integer(i64::from(*on)),
        Value::Number(number) => number.as_i64().map_or_else(|| Sql::Real(number.as_f64().unwrap_or(0.0)), Sql::Integer),
        Value::String(text) => match text.strip_prefix("x:").filter(|_| column.blob).and_then(hex) {
            Some(bytes) => Sql::Blob(bytes),
            None => Sql::Text(text.clone()),
        },
        other => Sql::Text(other.to_string()),
    }
}

fn hex(text: &str) -> Option<Vec<u8>> {
    if text.len() % 2 != 0 {
        return None;
    }
    (0..text.len()).step_by(2).map(|at| u8::from_str_radix(text.get(at..at + 2)?, 16).ok()).collect()
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn caught(conn: &Connection, batch: &str) -> Vec<RowChange> {
        conn.execute_batch(batch).unwrap();
        qnc_change_capture::take(conn).unwrap()
    }

    fn rows(conn: &Connection) -> Vec<(String, i64, String)> {
        let mut statement = conn.prepare("SELECT id, v, s FROM t ORDER BY id").unwrap();
        statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap()
    }

    /// A commit taken back leaves the rows as they were before it, made again as after it.
    #[test]
    fn a_commit_is_taken_back_and_made_again() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (id TEXT PRIMARY KEY, v INTEGER, s TEXT, b BLOB); INSERT INTO t VALUES ('a', 1, 'x', x'00ff'), ('b', 2, 'y', NULL);").unwrap();
        qnc_change_capture::install(&conn, &["t"]).unwrap();
        let before = rows(&conn);
        let changes = caught(&conn, "UPDATE t SET v = 10, s = 'z' WHERE id = 'a'; DELETE FROM t WHERE id = 'b'; INSERT INTO t VALUES ('c', 3, 'w', x'01');");
        let after = rows(&conn);
        revert(&conn, &changes, &[]).unwrap();
        assert_eq!(rows(&conn), before);
        let blob: Vec<u8> = conn.query_row("SELECT b FROM t WHERE id = 'a'", [], |row| row.get(0)).unwrap();
        assert_eq!(blob, [0x00, 0xff], "a blob comes back as bytes");
        reapply(&conn, &changes, &[]).unwrap();
        assert_eq!(rows(&conn), after);
    }

    /// A row changed since the commit is not overwritten; a loose column is neither checked nor
    /// put back.
    #[test]
    fn a_row_changed_since_is_refused_a_loose_column_is_left() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE t (id TEXT PRIMARY KEY, v INTEGER, s TEXT); INSERT INTO t VALUES ('a', 1, 'x');").unwrap();
        qnc_change_capture::install(&conn, &["t"]).unwrap();
        let changes = caught(&conn, "UPDATE t SET v = 2 WHERE id = 'a';");
        caught(&conn, "UPDATE t SET s = 'chosen' WHERE id = 'a';");
        assert!(revert(&conn, &changes, &[]).is_err(), "s changed since");
        revert(&conn, &changes, &[("t", &["s"])]).unwrap();
        assert_eq!(rows(&conn), [("a".to_string(), 1, "chosen".to_string())], "v back, s as it is");
    }
}
