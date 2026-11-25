//! The marks table of a project database and its requests; the history is the journal's.

use crate::*;
use rusqlite::{params, Connection, OptionalExtension};

/// The table this module owns; it writes no other.
const OWNED: [&str; 1] = ["clip_marks"];
/// The positions of a clip's history read from the journal: the newest ones.
const HISTORY_LIMIT: usize = 200;

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
        let schema_ready = object_exists(&conn, "view", "public_clip_marks")?;
        if access == Access::ReadOnly {
            conn.pragma_update(None, "query_only", true).map_err(err)?;
        }
        qnc_db_broker::write_only_own_tables(&conn, access, &OWNED);
        Ok(Self { conn, schema_ready })
    }

    pub(crate) fn execute(&mut self, operation: &Operation) -> Result<Data> {
        match operation {
            Operation::History { .. } if !self.schema_ready => Ok(Data::Entries(Vec::new())),
            Operation::History { clip_id } => self.history(clip_id),
            Operation::Keep { clip_id, from, to } => self.write(clip_id, Some(*from), *to),
            Operation::Back { clip_id, to } => self.write(clip_id, None, *to),
        }
    }

    /// The clip's row set to `to`; a clip with no row first gets `from`, its position before its
    /// first edit, so its start can always be gone back to (two positions of one commit).
    fn write(&mut self, clip_id: &str, from: Option<(u64, u64)>, to: (u64, u64)) -> Result<Data> {
        valid_clip(clip_id)?;
        for (in_frame, out_frame) in from.into_iter().chain([to]) {
            if out_frame <= in_frame || out_frame > i64::MAX as u64 {
                return Err(qnc_ui_text::text("db.clip_marks_invalid").into());
            }
        }
        let tx = qnc_db_broker::begin_write(&mut self.conn).map_err(err)?;
        let known: Option<i64> = tx.query_row("SELECT 1 FROM clip_marks WHERE clip_id = ?1", params![clip_id], |row| row.get(0)).optional().map_err(err)?;
        if known.is_none() {
            let (in_frame, out_frame) = from.unwrap_or(to);
            tx.execute("INSERT INTO clip_marks (clip_id, in_frame, out_frame) VALUES (?1, ?2, ?3)", params![clip_id, in_frame as i64, out_frame as i64]).map_err(err)?;
        }
        tx.execute("UPDATE clip_marks SET in_frame = ?2, out_frame = ?3 WHERE clip_id = ?1", params![clip_id, to.0 as i64, to.1 as i64]).map_err(err)?;
        tx.commit().map_err(err)?;
        Ok(Data::Changed)
    }

    /// The history of the clip: every write of its row the journal of the project keeps.
    fn history(&mut self, clip_id: &str) -> Result<Data> {
        valid_clip(clip_id)?;
        let commits = qnc_project_journal::row_history(&self.conn, OWNED[0], "clip_id", clip_id, HISTORY_LIMIT)?;
        let mut entries = Vec::new();
        for commit in commits {
            for (seq, change) in commit.changes.iter().enumerate() {
                let frame = |name: &str| change.after[name].as_u64();
                let (Some(in_frame), Some(out_frame)) = (frame("in_frame"), frame("out_frame")) else { continue };
                entries.push(Entry {
                    id: commit.id * 16 + seq as i64,
                    clip_id: clip_id.to_string(),
                    in_frame,
                    out_frame,
                    back: commit.action == "back",
                    author: commit.author.clone(),
                    made_at_utc: commit.made_at_utc.clone(),
                });
            }
        }
        Ok(Data::Entries(entries))
    }

    /// The connection the intermediary opens a leased write's transaction on.
    pub(crate) fn connection(&self) -> &Connection {
        &self.conn
    }
}

fn valid_clip(clip_id: &str) -> Result<()> {
    if clip_id.is_empty() || clip_id.len() > 200 || clip_id.chars().any(char::is_control) {
        return Err(qnc_ui_text::text("db.clip_marks_invalid").into());
    }
    Ok(())
}

fn ensure_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS clip_marks (
            clip_id TEXT PRIMARY KEY,
            in_frame INTEGER NOT NULL,
            out_frame INTEGER NOT NULL
        );
        CREATE VIEW IF NOT EXISTS public_clip_marks AS SELECT clip_id, in_frame, out_frame FROM clip_marks;",
    )
    .map_err(err)
}

fn object_exists(conn: &Connection, kind: &str, name: &str) -> Result<bool> {
    conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type=?1 AND name=?2)", params![kind, name], |r| r.get(0)).map_err(err)
}

pub(crate) fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}

impl qnc_db_broker::OwnedTables for Store {
    const ID: &'static str = MODULE_ID;
    const JOURNALED: &'static [&'static str] = &OWNED;
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
