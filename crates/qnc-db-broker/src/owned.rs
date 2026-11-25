//! What every table module repeated (user 2026-10-26, step 1 of the architecture cleanup,
//! before AI tables come): joining the intermediary with typed requests, and the authorizer
//! that lets a module write only its own tables.

use std::marker::PhantomData;
use std::sync::Arc;

use rusqlite::Connection;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::{Access, Result, TableModule, TableModuleFactory};

/// A table module: its id, its typed requests and answers, how it joins a project database
/// (creating its tables when it may write) and how it serves a request.
pub trait OwnedTables: Send + Sized + 'static {
    const ID: &'static str;
    /// The tables whose writes are the user's edits (the journal of the project keeps them).
    const JOURNALED: &'static [&'static str] = &[];
    type Operation: DeserializeOwned;
    type Data: Serialize;
    fn is_write(operation: &Self::Operation) -> bool;
    fn attach(connection: Connection, access: Access) -> Result<Self>;
    fn execute(&mut self, operation: &Self::Operation) -> Result<Self::Data>;
    fn connection(&self) -> &Connection;
    /// Whether a write is kept in the journal of the project (a selection is not).
    fn journals(_operation: &Self::Operation) -> bool {
        true
    }
    /// Whether a write is a step the module's Undo may take back.
    fn undoable(_operation: &Self::Operation) -> bool {
        false
    }
    /// The step of undo the last write was, taken once.
    fn journal_step(&mut self) -> Option<qnc_project_journal::Step> {
        None
    }
}

/// How the module `M` joins the intermediary.
pub fn owned_tables<M: OwnedTables>() -> Arc<dyn TableModuleFactory> {
    Arc::new(Factory::<M>(PhantomData))
}

struct Factory<M>(PhantomData<fn() -> M>);

impl<M: OwnedTables> TableModuleFactory for Factory<M> {
    fn id(&self) -> &'static str {
        M::ID
    }
    fn is_write(&self, payload: &Value) -> bool {
        serde_json::from_value::<M::Operation>(payload.clone()).is_ok_and(|operation| M::is_write(&operation))
    }
    fn attach(&self, connection: Connection, access: Access) -> Result<Box<dyn TableModule>> {
        Ok(Box::new(Attached(M::attach(connection, access)?)))
    }
    fn journaled(&self) -> &'static [&'static str] {
        M::JOURNALED
    }
    fn journals(&self, payload: &Value) -> bool {
        serde_json::from_value::<M::Operation>(payload.clone()).is_ok_and(|operation| M::journals(&operation))
    }
    fn undoable(&self, payload: &Value) -> bool {
        serde_json::from_value::<M::Operation>(payload.clone()).is_ok_and(|operation| M::undoable(&operation))
    }
}

struct Attached<M>(M);

impl<M: OwnedTables> TableModule for Attached<M> {
    fn execute(&mut self, payload: Value) -> Result<Value> {
        let operation: M::Operation = serde_json::from_value(payload).map_err(|e| e.to_string())?;
        let data = self.0.execute(&operation)?;
        serde_json::to_value(data).map_err(|e| e.to_string())
    }
    fn connection(&self) -> &Connection {
        self.0.connection()
    }
    fn journal_step(&mut self) -> Option<qnc_project_journal::Step> {
        self.0.journal_step()
    }
}

/// The connection reads anything and, when it may write, writes only the tables in `owned`.
/// Set it after the module has created its tables.
pub fn write_only_own_tables(conn: &Connection, access: Access, owned: &'static [&'static str]) {
    conn.authorizer(Some(move |ctx: rusqlite::hooks::AuthContext<'_>| {
        use rusqlite::hooks::{AuthAction as A, Authorization as R};
        match ctx.action {
            A::Insert { table_name } | A::Delete { table_name } | A::Update { table_name, .. }
                if access == Access::ReadWrite && owned.contains(&table_name) =>
            {
                R::Allow
            }
            // The journal of the project (user 2026-11-25): the rows an edit changed are caught on this
            // connection by TEMP triggers and appended as a commit; the journal is only appended to.
            A::Insert { table_name } | A::Delete { table_name } if access == Access::ReadWrite && table_name == qnc_change_capture::PENDING => R::Allow,
            A::Insert { table_name } if access == Access::ReadWrite && qnc_project_journal::TABLES.contains(&table_name) => R::Allow,
            A::CreateTempTable { .. } | A::CreateTempTrigger { .. } if access == Access::ReadWrite => R::Allow,
            // A TEMP trigger on a table of the database is checked as a row of its schema table (SQLite
            // `sqlite3BeginTrigger`), though it is kept in the TEMP schema. The schema itself stays
            // shut: every statement that changes it (CREATE TABLE, INDEX, VIEW, ALTER, a trigger that is
            // not TEMP) has its own action, denied below, and SQL cannot insert into it directly.
            A::Insert { table_name: "sqlite_master" | "sqlite_temp_master" } | A::Update { table_name: "sqlite_master" | "sqlite_temp_master", .. }
                if access == Access::ReadWrite =>
            {
                R::Allow
            }
            A::Pragma { pragma_name: "table_info", .. } => R::Allow,
            A::Read { .. }
            | A::Select
            | A::Transaction { .. }
            | A::Savepoint { .. }
            | A::Function { .. }
            | A::Recursive => R::Allow,
            // The intermediary attaches the project database to a draft story's connection
            // (`ProjectDb::with_companion`), after the module made its tables.
            A::Attach { .. } | A::Unknown { code: rusqlite::ffi::SQLITE_ATTACH, .. } => R::Allow,
            _ => R::Deny,
        }
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_module_writes_only_its_own_tables_and_reads_any() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE mine (v INTEGER); CREATE TABLE theirs (v INTEGER); INSERT INTO theirs VALUES (1);")
            .unwrap();
        write_only_own_tables(&conn, Access::ReadWrite, &["mine"]);
        conn.execute("INSERT INTO mine VALUES (2)", []).unwrap();
        assert!(conn.execute("INSERT INTO theirs VALUES (3)", []).is_err());
        assert!(conn.execute("DELETE FROM theirs", []).is_err());
        let read: i64 = conn.query_row("SELECT v FROM theirs", [], |row| row.get(0)).unwrap();
        assert_eq!(read, 1);
    }

    /// The journal's allowance opens no schema change and no change of the journal: only TEMP
    /// triggers, the pending rows and appending to the journal.
    #[test]
    fn the_journal_opens_no_schema_change_and_no_rewrite_of_the_journal() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE mine (v INTEGER);").unwrap();
        qnc_project_journal::ensure_schema(&conn).unwrap();
        write_only_own_tables(&conn, Access::ReadWrite, &["mine"]);
        qnc_change_capture::install(&conn, &["mine"]).unwrap();
        conn.execute("INSERT INTO mine VALUES (1)", []).unwrap();
        assert!(conn.execute_batch("CREATE TABLE other (v INTEGER)").is_err());
        assert!(conn.execute_batch("CREATE INDEX i ON mine (v)").is_err());
        assert!(conn.execute_batch("CREATE TRIGGER t AFTER INSERT ON mine BEGIN SELECT 1; END").is_err(), "a trigger that is not TEMP");
        assert!(conn.execute_batch("DELETE FROM project_journal").is_err());
        assert!(conn.execute_batch("UPDATE project_journal SET author = 'x'").is_err());
    }

    #[test]
    fn a_read_only_module_writes_not_even_its_own() {
        let conn = Connection::open_in_memory().unwrap();
        conn.execute_batch("CREATE TABLE mine (v INTEGER);").unwrap();
        write_only_own_tables(&conn, Access::ReadOnly, &["mine"]);
        assert!(conn.execute("INSERT INTO mine VALUES (1)", []).is_err());
    }
}
