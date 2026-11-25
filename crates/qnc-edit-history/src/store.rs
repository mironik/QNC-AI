//! The history table of a project database and its requests.

use crate::*;
use rusqlite::{params, Connection};

/// The table this module owns; it writes no other.
const OWNED: [&str; 1] = ["edit_history"];
/// The entries kept per clip: the oldest go beyond this.
const KEPT_PER_CLIP: i64 = 200;
const MAX_LABEL: usize = 300;
const MAX_STATE: usize = 64 * 1024;

pub(crate) struct Store {
    conn: Connection,
    schema_ready: bool,
}

impl Store {
    /// Creates the table if missing (read-write) and serves it; a read-only connection of a
    /// project without it reads no history.
    pub(crate) fn attach(mut conn: Connection, access: Access) -> Result<Self> {
        if access == Access::ReadWrite {
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(err)?;
            ensure_schema(&tx)?;
            tx.commit().map_err(err)?;
        }
        let schema_ready = object_exists(&conn, "view", "public_edit_history")?;
        if access == Access::ReadOnly {
            conn.pragma_update(None, "query_only", true).map_err(err)?;
        }
        qnc_db_broker::write_only_own_tables(&conn, access, &OWNED);
        Ok(Self { conn, schema_ready })
    }

    pub(crate) fn execute(&mut self, operation: &Operation) -> Result<Data> {
        match operation {
            Operation::Read { .. } if !self.schema_ready => Ok(Data::Entries(Vec::new())),
            Operation::Read { clip_id } => self.read(clip_id),
            Operation::Add(entry) => self.add(entry),
        }
    }

    fn add(&mut self, entry: &NewEntry) -> Result<Data> {
        valid_text(&entry.clip_id, 200)?;
        valid_text(&entry.kind, 60)?;
        valid_text(&entry.label, MAX_LABEL)?;
        let state = serde_json::to_string(&entry.state).map_err(err)?;
        if state.len() > MAX_STATE {
            return Err(qnc_ui_text::text("db.edit_history_invalid").into());
        }
        let tx = qnc_db_broker::begin_write(&mut self.conn).map_err(err)?;
        tx.execute(
            "INSERT INTO edit_history (clip_id, kind, label, state_json, made_at_utc)
             VALUES (?1, ?2, ?3, ?4, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
            params![entry.clip_id, entry.kind, entry.label, state],
        )
        .map_err(err)?;
        tx.execute(
            "DELETE FROM edit_history WHERE clip_id = ?1 AND id NOT IN
               (SELECT id FROM edit_history WHERE clip_id = ?1 ORDER BY id DESC LIMIT ?2)",
            params![entry.clip_id, KEPT_PER_CLIP],
        )
        .map_err(err)?;
        tx.commit().map_err(err)?;
        Ok(Data::Changed)
    }

    fn read(&mut self, clip_id: &str) -> Result<Data> {
        valid_text(clip_id, 200)?;
        let mut statement = self
            .conn
            .prepare("SELECT id, clip_id, kind, label, made_at_utc, state_json FROM public_edit_history WHERE clip_id = ?1 ORDER BY id")
            .map_err(err)?;
        let rows = statement
            .query_map(params![clip_id], |row| {
                let state: String = row.get(5)?;
                Ok(Entry { id: row.get(0)?, clip_id: row.get(1)?, kind: row.get(2)?, label: row.get(3)?, made_at_utc: row.get(4)?, state: serde_json::from_str(&state).unwrap_or(Value::Null) })
            })
            .map_err(err)?;
        let entries = rows.collect::<rusqlite::Result<Vec<_>>>().map_err(err)?;
        Ok(Data::Entries(entries))
    }

    /// The connection the intermediary opens a leased write's transaction on.
    pub(crate) fn connection(&self) -> &Connection {
        &self.conn
    }
}

fn valid_text(text: &str, max: usize) -> Result<()> {
    if text.is_empty() || text.len() > max || text.chars().any(char::is_control) {
        return Err(qnc_ui_text::text("db.edit_history_invalid").into());
    }
    Ok(())
}

fn ensure_schema(conn: &Connection) -> Result<()> {
    if !object_exists(conn, "table", "edit_history")? {
        conn.execute_batch(
            "CREATE TABLE edit_history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                clip_id TEXT NOT NULL,
                kind TEXT NOT NULL,
                label TEXT NOT NULL,
                state_json TEXT NOT NULL,
                made_at_utc TEXT NOT NULL
            );
            CREATE INDEX edit_history_clip ON edit_history (clip_id, id);",
        )
        .map_err(err)?;
    }
    if !object_exists(conn, "view", "public_edit_history")? {
        conn.execute_batch(
            "CREATE VIEW public_edit_history AS
             SELECT id, clip_id, kind, label, state_json, made_at_utc FROM edit_history;",
        )
        .map_err(err)?;
    }
    Ok(())
}

fn object_exists(conn: &Connection, kind: &str, name: &str) -> Result<bool> {
    conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type=?1 AND name=?2)", params![kind, name], |r| r.get(0)).map_err(err)
}

pub(crate) fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

impl qnc_db_broker::OwnedTables for Store {
    const ID: &'static str = MODULE_ID;
    type Operation = Operation;
    type Data = Data;
    fn is_write(operation: &Operation) -> bool {
        operation.is_write()
    }
    fn attach(connection: Connection, access: Access) -> Result<Self> {
        Store::attach(connection, access)
    }
    fn execute(&mut self, operation: &Operation) -> Result<Data> {
        Store::execute(self, operation)
    }
    fn connection(&self) -> &Connection {
        Store::connection(self)
    }
}
