//! What the history reads of the project database: its moments, its owner and the pieces with
//! commits after a moment. A table module of the intermediary that owns no table and writes none.

use qnc_db_broker::{Access, ProjectDbTarget, TableModuleFactory};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::{Moment, Result};

pub const MODULE_ID: &str = "qnc.module.project-history";

/// How many moments the list shows.
pub(crate) const MOMENTS: usize = 500;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case", deny_unknown_fields)]
pub(crate) enum Operation {
    Moments { limit: usize },
    Owner,
    ModulesAfter { to: i64 },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub(crate) enum Data {
    Moments(Vec<Moment>),
    Owner(Option<String>),
    Modules(Vec<String>),
}

pub(crate) struct Reader {
    conn: Connection,
}

impl qnc_db_broker::OwnedTables for Reader {
    const ID: &'static str = MODULE_ID;
    type Operation = Operation;
    type Data = Data;
    fn is_write(_operation: &Operation) -> bool {
        false
    }
    fn attach(conn: Connection, access: Access) -> Result<Self> {
        if access == Access::ReadOnly {
            conn.pragma_update(None, "query_only", true).map_err(err)?;
        }
        qnc_db_broker::write_only_own_tables(&conn, access, &[]);
        Ok(Self { conn })
    }
    fn execute(&mut self, operation: &Operation) -> Result<Data> {
        match operation {
            Operation::Moments { limit } => {
                let commits = qnc_project_journal::moments(&self.conn, *limit)?;
                Ok(Data::Moments(commits.into_iter().map(|commit| Moment { id: commit.id, at: commit.made_at_utc, author: commit.author, module: commit.module, action: commit.action, refers: commit.refers }).collect()))
            }
            Operation::Owner => Ok(Data::Owner(owner(&self.conn)?)),
            Operation::ModulesAfter { to } => Ok(Data::Modules(qnc_project_journal::modules_after(&self.conn, *to)?)),
        }
    }
    fn connection(&self) -> &Connection {
        &self.conn
    }
}

/// The QNC user who made the project (none for a project made before it was kept).
fn owner(conn: &Connection) -> Result<Option<String>> {
    let kept: bool = conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'public_project_owner')", [], |row| row.get(0)).map_err(err)?;
    if !kept {
        return Ok(None);
    }
    let owner: Option<String> = conn.query_row("SELECT qnc_user FROM public_project_owner LIMIT 1", [], |row| row.get(0)).optional().map_err(err)?;
    Ok(owner.filter(|owner| !owner.trim().is_empty()))
}

pub(crate) fn factory() -> Arc<dyn TableModuleFactory> {
    qnc_db_broker::owned_tables::<Reader>()
}

pub(crate) fn ask(target: &ProjectDbTarget, operation: &Operation) -> Result<Data> {
    let mut client = target.open(Access::ReadOnly, vec![factory()])?;
    let answer = client.execute(MODULE_ID, serde_json::to_value(operation).map_err(err)?)?;
    serde_json::from_value(answer).map_err(err)
}

fn err(error: impl std::fmt::Display) -> String {
    error.to_string()
}
