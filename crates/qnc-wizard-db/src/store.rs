//! The wizard table of a project database and its requests.

use crate::*;
use rusqlite::{params, Connection};

/// The table this module owns; it writes no other.
const OWNED: [&str; 1] = ["wizard_steps"];

pub(crate) struct Store {
    conn: Connection,
    schema_ready: bool,
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
                CREATE VIEW IF NOT EXISTS public_wizard_steps AS SELECT step, done_at_utc FROM wizard_steps;",
            )
            .map_err(err)?;
            tx.commit().map_err(err)?;
        }
        let schema_ready = conn
            .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='view' AND name='public_wizard_steps')", [], |r| r.get(0))
            .map_err(err)?;
        if access == Access::ReadOnly {
            conn.pragma_update(None, "query_only", true).map_err(err)?;
        }
        qnc_db_broker::write_only_own_tables(&conn, access, &OWNED);
        Ok(Self { conn, schema_ready })
    }

    pub(crate) fn execute(&mut self, operation: &Operation) -> Result<Data> {
        match operation {
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
