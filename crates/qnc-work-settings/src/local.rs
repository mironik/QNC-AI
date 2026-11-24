use crate::{ReadError, WorkSettings};
use qnc_transport_resolver::{ResolvedEndpoint, ResolverConfig};
use rusqlite::{Connection, OpenFlags, OptionalExtension};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};

pub const LOCAL_REGISTRY_URI: &str = "qnc://local/db/project_registry";

/// How many times a read is tried while the database is busy (another process writes), and how
/// long between two tries (user 2026-11-16: "dodaj ponovni pokušaj kad je baza zauzeta").
const BUSY_TRIES: u32 = 10;
const BUSY_PAUSE: Duration = Duration::from_millis(300);

/// `read` tried again while the database is busy; any other error at once.
fn again_while_busy<T>(mut read: impl FnMut() -> Result<T, ReadError>) -> Result<T, ReadError> {
    let mut tried = 1;
    loop {
        match read() {
            Err(error) if error.code == BUSY && tried < BUSY_TRIES => {
                tried += 1;
                std::thread::sleep(BUSY_PAUSE);
            }
            result => return result,
        }
    }
}

/// The code of a read refused because another process held the database.
const BUSY: &str = "database_busy";

/// Whether SQLite refused because the database is busy or locked.
fn busy(error: &rusqlite::Error) -> bool {
    matches!(error.sqlite_error_code(), Some(rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked))
}

pub(crate) fn workspace_file(registry_file: &Path, id: &str) -> Result<PathBuf, ReadError> {
    again_while_busy(|| workspace_file_once(registry_file, id))
}

fn workspace_file_once(registry_file: &Path, id: &str) -> Result<PathBuf, ReadError> {
    if id.is_empty() || id.contains(['/', '\\', ':']) || matches!(id, "." | "..") {
        return Err(db_error());
    }
    let registry = open_read_only(registry_file)?;
    let path: String = registry
        .query_row(
            "SELECT s.local_path FROM project_storage_locations s
         JOIN public_projects p ON p.project_id=s.project_id WHERE p.project_id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(db_err)?;
    let directory = PathBuf::from(path);
    if !directory.is_absolute() {
        return Err(db_error());
    }
    let file = directory.join("project.db");
    let db = open_read_only(&file)?;
    let matches: bool = db
        .query_row(
            "SELECT count(*)=1 AND min(project_id)=?1 FROM public_project_settings",
            [id],
            |r| r.get(0),
        )
        .map_err(db_err)?;
    if !matches {
        return Err(db_error());
    }
    Ok(file)
}

fn open_read_only(path: &Path) -> Result<Connection, ReadError> {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| {
        ReadError::new(
            "database_unavailable",
            "The work settings database is not available.",
        )
    })?;
    conn.busy_timeout(Duration::from_secs(2))
        .map_err(db_err)?;
    // Connection-local guard only; never create, repair or configure journal files.
    conn.pragma_update(None, "query_only", true)
        .map_err(db_err)?;
    Ok(conn)
}

fn db_error() -> ReadError {
    ReadError::new(
        "database_contract",
        "The database lacks the public view or cannot be read.",
    )
}

fn bound_path(resolver: &ResolverConfig, uri: &str) -> Result<PathBuf, ReadError> {
    match resolver.resolve(uri).map_err(db_err)?.endpoint {
        ResolvedEndpoint::LocalPath(path) => Ok(path),
        _ => Err(db_error()),
    }
}

pub(crate) fn read(registry_file: &Path, context_uri: &str, pinned: Option<&str>) -> Result<WorkSettings, ReadError> {
    again_while_busy(|| read_once(registry_file, context_uri, pinned))
}

fn read_once(registry_file: &Path, context_uri: &str, pinned: Option<&str>) -> Result<WorkSettings, ReadError> {
    let context = crate::registry_context(context_uri)?;
    let resolver =
        ResolverConfig::new(PathBuf::new()).with_local_binding(LOCAL_REGISTRY_URI, registry_file);
    let mut registry = open_read_only(&bound_path(&resolver, LOCAL_REGISTRY_URI)?)?;
    let tx = registry.transaction().map_err(db_err)?;
    // A process given the project it works on (QNC-AI: the ingest of one card into its own base,
    // chosen by the tray from the bases' registry, user 2026-11-14) reads that project; every
    // other process the one the database marks active.
    let active: Option<String> = match pinned {
        Some(pinned) => Some(pinned.to_string()),
        None => tx
            .query_row(
                "SELECT value FROM public_app_settings WHERE key = 'active_project_id'",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(db_err)?,
    };
    let active = active.filter(|id| !id.is_empty()).ok_or_else(|| {
        ReadError::new("no_active_project", "No active project is chosen in the database.")
    })?;
    if active.contains(['/', '\\', ':']) || active == "." || active == ".." {
        return Err(db_error());
    }
    let (name, project_uri): (String, String) = tx
        .query_row(
            "SELECT name, project_uri FROM public_projects WHERE project_id = ?1",
            [&active],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(db_err)?;
    let public_uri = qnc_contracts::parse_qnc_uri(&project_uri).map_err(db_err)?;
    if public_uri.resource_kind != "project" || public_uri.resource_id != active {
        return Err(db_error());
    }

    // Private owner-provided filesystem binding, used only inside the local transport adapter.
    // It is not returned to a consumer and is never used to choose/activate a project.
    let directory: String = tx
        .query_row(
            "SELECT local_path FROM project_storage_locations WHERE project_id = ?1",
            [&active],
            |r| r.get(0),
        )
        .map_err(|_| {
            ReadError::new(
                "missing_binding",
                "The transport location of the chosen database is missing.",
            )
        })?;
    let directory = PathBuf::from(directory);
    if !directory.is_absolute() {
        return Err(ReadError::new(
            "missing_binding",
            "The transport location of the database is incomplete.",
        ));
    }
    let local_db_uri = format!("qnc://local/db/project_workspace/{active}");
    let resolver = resolver.with_local_binding(&local_db_uri, directory.join("project.db"));
    let db = open_read_only(&bound_path(&resolver, &local_db_uri)?)?;
    let settings: String = db
        .query_row(
            "SELECT settings_json FROM public_project_settings WHERE project_id = ?1",
            [&active],
            |r| r.get(0),
        )
        .map_err(|error| match busy(&error) {
            true => db_err(error),
            false => ReadError::new("settings_not_public", "The public view of the database holds no work settings."),
        })?;
    let settings = serde_json::from_str(&settings).map_err(db_err)?;
    let snapshot = WorkSettings::from_saved(
        active.clone(),
        name,
        format!("{context}/db/project_workspace/{active}"),
        format!("{context}/project/{active}"),
        settings,
    )?;
    tx.commit().map_err(db_err)?;
    Ok(snapshot)
}

/// As [`db_error`], with what the database said (user 2026-11-16: the worker logged only "lacks
/// the public view", the real cause was thrown away).
/// A busy or locked database is `database_busy` (tried again); anything else is the contract error.
fn db_err(error: impl std::fmt::Display + 'static) -> ReadError {
    let sqlite = (&error as &dyn std::any::Any).downcast_ref::<rusqlite::Error>();
    if sqlite.is_some_and(busy) {
        return ReadError::new(BUSY, &format!("The database is busy (another process writes): {error}"));
    }
    ReadError::new("database_contract", &format!("The database lacks the public view or cannot be read: {error}"))
}

#[cfg(test)]
mod busy_tests {
    use super::*;

    #[test]
    fn a_busy_database_is_tried_again_another_error_is_not() {
        let mut calls = 0;
        let read = again_while_busy(|| {
            calls += 1;
            if calls < 3 { Err(ReadError::new(BUSY, "busy")) } else { Ok(calls) }
        });
        assert_eq!(read.unwrap(), 3);
        let mut calls = 0;
        let read: Result<(), ReadError> = again_while_busy(|| {
            calls += 1;
            Err(ReadError::new("database_contract", "no view"))
        });
        assert!(read.is_err());
        assert_eq!(calls, 1, "only a busy database is tried again");
        let busy_error = rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY), None);
        assert_eq!(db_err(busy_error).code, BUSY);
    }
}
