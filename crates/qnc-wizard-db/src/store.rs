//! The wizard table of a project database and its requests.

use crate::*;
use rusqlite::{params, Connection};

/// The tables this module owns; it writes no other.
const OWNED: [&str; 2] = ["wizard_steps", "wizard_cards"];
/// At most this many cards in a story.
const MAX_CARDS: usize = 16;

pub(crate) struct Store {
    conn: Connection,
    schema_ready: bool,
    cards_ready: bool,
}

impl Store {
    pub(crate) fn attach(mut conn: Connection, access: Access) -> Result<Self> {
        if access == Access::ReadWrite {
            let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(err)?;
            tx.execute_batch(
                "CREATE TABLE IF NOT EXISTS wizard_steps (
                    step TEXT PRIMARY KEY,
                    done_at_utc TEXT NOT NULL
                );
                CREATE VIEW IF NOT EXISTS public_wizard_steps AS SELECT step, done_at_utc FROM wizard_steps;
                CREATE TABLE IF NOT EXISTS wizard_cards (
                    position INTEGER PRIMARY KEY,
                    serial TEXT NOT NULL
                );
                CREATE VIEW IF NOT EXISTS public_wizard_cards AS SELECT position, serial FROM wizard_cards;",
            )
            .map_err(err)?;
            tx.commit().map_err(err)?;
        }
        let view = |name: &str| -> Result<bool> {
            conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='view' AND name=?1)", params![name], |r| r.get(0)).map_err(err)
        };
        let (schema_ready, cards_ready) = (view("public_wizard_steps")?, view("public_wizard_cards")?);
        if access == Access::ReadOnly {
            conn.pragma_update(None, "query_only", true).map_err(err)?;
        }
        qnc_db_broker::write_only_own_tables(&conn, access, &OWNED);
        Ok(Self { conn, schema_ready, cards_ready })
    }

    pub(crate) fn execute(&mut self, operation: &Operation) -> Result<Data> {
        match operation {
            Operation::ReadCards if !self.cards_ready => Ok(Data::Cards(Vec::new())),
            Operation::ReadCards => {
                let mut statement = self.conn.prepare("SELECT serial FROM public_wizard_cards ORDER BY position").map_err(err)?;
                let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(err)?;
                Ok(Data::Cards(rows.collect::<rusqlite::Result<_>>().map_err(err)?))
            }
            Operation::SetCards { serials } => {
                if serials.len() > MAX_CARDS || serials.iter().any(|serial| serial.is_empty() || serial.len() > 64 || !serial.chars().all(|c| c.is_ascii_alphanumeric())) {
                    return Err(qnc_wizard_text::text("wizard.db.bad_card"));
                }
                let tx = qnc_db_broker::begin_write(&mut self.conn).map_err(err)?;
                tx.execute("DELETE FROM wizard_cards", []).map_err(err)?;
                for (position, serial) in serials.iter().enumerate() {
                    tx.execute("INSERT INTO wizard_cards (position, serial) VALUES (?1, ?2)", params![position as i64, serial]).map_err(err)?;
                }
                tx.commit().map_err(err)?;
                Ok(Data::Changed)
            }
            Operation::Read if !self.schema_ready => Ok(Data::Done(BTreeSet::new())),
            Operation::Read => {
                let mut statement = self.conn.prepare("SELECT step FROM public_wizard_steps").map_err(err)?;
                let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(err)?;
                Ok(Data::Done(rows.collect::<rusqlite::Result<_>>().map_err(err)?))
            }
            Operation::MarkDone { step } => {
                valid(step)?;
                let tx = qnc_db_broker::begin_write(&mut self.conn).map_err(err)?;
                tx.execute(
                    "INSERT INTO wizard_steps (step, done_at_utc) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))
                     ON CONFLICT(step) DO UPDATE SET done_at_utc = excluded.done_at_utc",
                    params![step],
                )
                .map_err(err)?;
                tx.commit().map_err(err)?;
                Ok(Data::Changed)
            }
            Operation::Reopen { step } => {
                valid(step)?;
                let tx = qnc_db_broker::begin_write(&mut self.conn).map_err(err)?;
                tx.execute("DELETE FROM wizard_steps WHERE step = ?1", params![step]).map_err(err)?;
                tx.commit().map_err(err)?;
                Ok(Data::Changed)
            }
        }
    }

    pub(crate) fn connection(&self) -> &Connection {
        &self.conn
    }
}

fn valid(step: &str) -> Result<()> {
    if qnc_wizard_steps::is_step(step) {
        Ok(())
    } else {
        Err(qnc_wizard_text::text("wizard.db.bad_step"))
    }
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
