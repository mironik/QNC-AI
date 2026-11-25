//! The local executor: one connection per table module, all on the same file.

use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use rusqlite::{Connection, OpenFlags, OptionalExtension};
use serde_json::Value;

use crate::{project_id, Access, Request, Result, TableModule, TableModuleFactory, VERSION};

pub struct ProjectDb {
    file: PathBuf,
    uri: String,
    access: Access,
    factories: Vec<Arc<dyn TableModuleFactory>>,
    modules: BTreeMap<&'static str, Box<dyn TableModule>>,
    /// The draft story file the modules write instead, the project database attached.
    draft: Option<PathBuf>,
}

/// The files of the S-Story versions beside the project database: `story-draft-<n>.db`, one per
/// version (user 2026-10-31: every version of the S-Story is a copy of the Story of its own).
pub const DRAFT_PREFIX: &str = "story-draft-";
/// The file of the AI results beside the project database (docs/102 step 7): the tables of the AI
/// modules, apart from the project, so deleting it leaves the project as it was.
pub const AI_FILE: &str = "ai.db";

/// The file of version `version` of the S-Story.
pub fn draft_file(version: u32) -> String {
    format!("{DRAFT_PREFIX}{version}.db")
}

/// The highest version whose story file lies beside the database `conn` is a connection of
/// (0: none). A new version is numbered past it too, so it never lands in an old story's file.
pub fn highest_draft(conn: &Connection) -> u32 {
    let Some(folder) = conn.path().and_then(|path| Path::new(path).parent().map(Path::to_path_buf)) else { return 0 };
    let Ok(entries) = std::fs::read_dir(folder) else { return 0 };
    entries
        .filter_map(|entry| entry.ok()?.file_name().into_string().ok())
        .filter_map(|name| name.strip_prefix(DRAFT_PREFIX)?.strip_suffix(".db")?.parse::<u32>().ok())
        .max()
        .unwrap_or(0)
}
/// The name the project database is attached under on a draft connection.
pub const PROJECT_SCHEMA: &str = "project";

/// Whether `conn` is a connection of the draft story (the project database attached).
pub fn is_draft(conn: &Connection) -> bool {
    // The main file is the draft's (a pragma read is refused by the table modules' authorizer).
    conn.path().and_then(|path| Path::new(path).file_name()).and_then(|name| name.to_str()).is_some_and(|name| name.starts_with(DRAFT_PREFIX))
}

/// How every Ingest base is named (`qnc-ingest-base` `NAME_PREFIX` and a space); a story's name never
/// starts with it.
pub const INGEST_BASE_PREFIX: &str = "Ingest ";

/// Whether the database of `conn` (the project database, or one attached to a draft as `project`)
/// is an Ingest base's (user 2026-11-24: "ingestu pripada samo ono sto naprave radnici
/// QNC-Ingest-Wizard, sve drugo pripada projektu"): its own origin says the name. A base holds only
/// what the wizard made and the statements; a story, a script and their drafts are a project's.
pub fn is_ingest_base(conn: &Connection) -> bool {
    ["main", "project"].iter().any(|schema| {
        conn.query_row(&format!("SELECT project_name FROM {schema}.public_project_origin LIMIT 1"), [], |row| row.get::<_, String>(0))
            .is_ok_and(|name| name.starts_with(INGEST_BASE_PREFIX))
    })
}

/// The refusal of a project's write into an Ingest base.
pub fn ingest_base_refusal() -> String {
    qnc_ui_text::text("db.ingest_base_project_only").to_string()
}

impl std::fmt::Debug for ProjectDb {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectDb")
            .field("uri", &self.uri)
            .finish_non_exhaustive()
    }
}

impl ProjectDb {
    /// The database of project `uri` at the private `file` binding the project gives.
    /// It must exist and belong to that project; nothing creates it here.
    pub fn open(
        file: &Path,
        uri: &str,
        access: Access,
        factories: Vec<Arc<dyn TableModuleFactory>>,
    ) -> Result<Self> {
        let id = project_id(uri)?;
        if !file.is_file() {
            return Err(qnc_ui_text::text("db.project_database_does_exist").into());
        }
        let check = connect(file, Access::ReadOnly)?;
        let owner: Option<String> = check
            .query_row(
                "SELECT min(project_id) FROM public_project_settings HAVING count(*) = 1",
                [],
                |r| r.get(0),
            )
            .optional()
            .map_err(|_| qnc_ui_text::text("db.file_project_database").to_string())?;
        if owner.as_deref() != Some(id.as_str()) {
            return Err(qnc_ui_text::text("db.database_belongs_another_project").into());
        }
        Ok(Self {
            file: file.to_path_buf(),
            uri: uri.into(),
            access,
            factories,
            modules: BTreeMap::new(),
            draft: None,
        })
    }

    /// The draft story of this project (user 2026-10-31: S-Story, a story of its
    /// own beside the Story): the modules' tables in a companion file beside the project
    /// database, created empty when missing; the project database is attached read through, so
    /// the catalog, the settings and the leases are the project's. Unqualified names find the
    /// draft's tables first, so a module writes only there.
    /// The AI results of the project (`ai.db`) are such a companion file too.
    pub fn with_companion(mut self, name: &str) -> Result<Self> {
        let file = self.file.with_file_name(name);
        // A draft story is a project's, never made beside an Ingest base (its AI file is the wizard's).
        if !file.is_file() && name.starts_with(DRAFT_PREFIX) {
            let base = Connection::open_with_flags(&self.file, OpenFlags::SQLITE_OPEN_READ_ONLY).is_ok_and(|conn| is_ingest_base(&conn));
            if base {
                return Err(ingest_base_refusal());
            }
        }
        if !file.is_file() {
            Connection::open_with_flags(&file, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_CREATE)
                .map_err(|e| e.to_string())?;
        }
        self.draft = Some(file);
        Ok(self)
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn execute(&mut self, request: &Request) -> Result<Value> {
        if request.version != VERSION || request.db_uri != self.uri {
            return Err(qnc_ui_text::text("db.invalid_request_project_database").into());
        }
        let factory = self
            .factories
            .iter()
            .find(|factory| factory.id() == request.module)
            .cloned()
            .ok_or_else(|| {
                qnc_ui_text::fill("db.unknown_module", &[("module", &request.module)])
            })?;
        if factory.is_write(&request.payload) && self.access == Access::ReadOnly {
            return Err(qnc_ui_text::text("db.access_read_only").into());
        }
        if !self.modules.contains_key(factory.id()) {
            let module = match &self.draft {
                Some(draft) => {
                    // The module makes its tables on the draft alone, then the project database
                    // is attached: no schema step of the module ever reaches the Story.
                    let module = factory.attach(connect(draft, self.access)?, self.access)?;
                    let conn = module.connection();
                    conn.execute("ATTACH DATABASE ?1 AS project", [self.file.to_string_lossy()])
                        .map_err(|e| e.to_string())?;
                    if self.access == Access::ReadWrite {
                        keep_wal_file(conn, c"project");
                    }
                    module
                }
                None => factory.attach(connect(&self.file, self.access)?, self.access)?,
            };
            if self.access == Access::ReadWrite && !factory.journaled().is_empty() {
                self.start_journal(module.as_ref(), factory.journaled())?;
            }
            self.modules.insert(factory.id(), module);
        }
        let module = self
            .modules
            .get_mut(factory.id())
            .ok_or(qnc_ui_text::text("db.project_database_module_open"))?;
        let answer = match &request.lease {
            Some(lease) if factory.is_write(&request.payload) => {
                leased(module.as_mut(), lease, request.payload.clone())
            }
            _ => module.execute(request.payload.clone()),
        }?;
        if self.access == Access::ReadWrite && !factory.journaled().is_empty() && factory.is_write(&request.payload) {
            let step = module.journal_step();
            if factory.journals(&request.payload) || step.is_some() {
                let author = request.author.clone().unwrap_or_else(crate::local_author);
                let action = request.payload.get("kind").and_then(|kind| kind.as_str()).unwrap_or("write").to_string();
                let kept = Kept { module: factory.id(), action, undoable: factory.undoable(&request.payload), step };
                commit(module.connection(), author, kept);
            } else {
                // A write that is no edit (a selection): the rows it changed are not a commit.
                let _ = qnc_change_capture::take(module.connection());
            }
        }
        Ok(answer)
    }

    /// The journal of the project for a module whose writes are edits (user 2026-11-25): its tables
    /// in the database the module writes (the project's, or a draft story's), made on a plain
    /// connection, and the module's connection catching the rows of its edit tables.
    fn start_journal(&self, module: &dyn TableModule, tables: &[&str]) -> Result<()> {
        let file = self.draft.as_deref().unwrap_or(&self.file);
        qnc_project_journal::ensure_schema(&connect(file, Access::ReadWrite)?)?;
        qnc_change_capture::install(module.connection(), tables)
    }
}

/// The rows the write just made changed, appended as one commit of the journal. The edit is
/// already written: a journal that cannot be appended to is said in the log and the edit stays.
/// What a write is in the journal: its piece, its action, whether Undo may take it back, and the
/// step of undo it was (then its action is `undo` or `redo`, with the commit it refers to).
struct Kept {
    module: &'static str,
    action: String,
    undoable: bool,
    step: Option<qnc_project_journal::Step>,
}

fn commit(conn: &Connection, author: String, kept: Kept) {
    let appended = (|| -> Result<()> {
        let changes = qnc_change_capture::take(conn)?;
        if changes.is_empty() {
            return Ok(());
        }
        let (action, undoable, refers) = match kept.step {
            Some(step) => (step.action().to_string(), false, Some(step.refers())),
            None => (kept.action, kept.undoable, None),
        };
        conn.execute_batch("BEGIN IMMEDIATE").map_err(|e| e.to_string())?;
        let made = qnc_project_journal::append(conn, &qnc_project_journal::NewCommit { author, module: kept.module.to_string(), action, undoable, refers, changes });
        let end = if made.is_ok() { "COMMIT" } else { "ROLLBACK" };
        conn.execute_batch(end).map_err(|e| e.to_string())?;
        made.map(|_| ())
    })();
    if let Err(error) = appended {
        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("project-journal not kept: {error}"));
    }
}

/// A write made under a lease (external audit 2026-10-17): the lease is checked and the
/// write made in one transaction, so a worker that lost its lease writes nothing.
fn leased(
    module: &mut dyn TableModule,
    lease: &crate::WriteLease,
    payload: Value,
) -> Result<Value> {
    module
        .connection()
        .execute_batch("BEGIN IMMEDIATE")
        .map_err(|e| e.to_string())?;
    let result = lease
        .check(module.connection())
        .and_then(|()| module.execute(payload));
    let end = if result.is_ok() { "COMMIT" } else { "ROLLBACK" };
    match module.connection().execute_batch(end) {
        Ok(()) => result,
        Err(error) => {
            let _ = module.connection().execute_batch("ROLLBACK");
            result.and(Err(error.to_string()))
        }
    }
}

/// A connection with the policy of the project database: waits for other writers,
/// keeps the rollback journal file (the project directory denies deletion) unless
/// the project is in WAL, never changes the identity of the file.
fn connect(file: &Path, access: Access) -> Result<Connection> {
    let flags = if access == Access::ReadOnly {
        OpenFlags::SQLITE_OPEN_READ_ONLY
    } else {
        OpenFlags::SQLITE_OPEN_READ_WRITE
    };
    let conn = Connection::open_with_flags(file, flags).map_err(|e| e.to_string())?;
    conn.busy_timeout(Duration::from_secs(5))
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "trusted_schema", "OFF")
        .map_err(|e| e.to_string())?;
    conn.pragma_update(None, "foreign_keys", "ON")
        .map_err(|e| e.to_string())?;
    if access == Access::ReadWrite {
        let mode: String = conn
            .pragma_query_value(None, "journal_mode", |r| r.get(0))
            .map_err(|e| e.to_string())?;
        if !mode.eq_ignore_ascii_case("wal") {
            conn.pragma_update(None, "journal_mode", "PERSIST")
                .map_err(|e| e.to_string())?;
        }
        conn.pragma_update(None, "synchronous", "FULL")
            .map_err(|e| e.to_string())?;
        keep_wal_file(&conn, c"main");
    }
    Ok(conn)
}

/// The WAL file stays when this connection closes. The project directory denies deletion
/// (the delete lock of the project), so SQLite's delete of the WAL file at the close of
/// the last writing connection was refused and retried for ~3 s while it held the
/// database; every reader and writer waited (live 2026-10-12: a Sync play and a clip
/// waited 1.5-3 s). Closing now takes ~1 ms; the file is reused, never removed.
fn keep_wal_file(conn: &Connection, schema: &std::ffi::CStr) {
    let mut on: std::os::raw::c_int = 1;
    // SAFETY: a file control of this open connection's main database; it only sets a flag.
    unsafe {
        rusqlite::ffi::sqlite3_file_control(
            conn.handle(),
            schema.as_ptr(),
            rusqlite::ffi::SQLITE_FCNTL_PERSIST_WAL,
            (&mut on as *mut std::os::raw::c_int).cast(),
        );
    }
}

#[cfg(test)]
mod base_tests {
    use super::*;

    #[test]
    fn an_ingest_base_is_known_by_its_own_name_and_gets_no_draft_story() {
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("project.db");
        let conn = Connection::open(&file).unwrap();
        conn.execute_batch("CREATE TABLE public_project_origin (project_name TEXT); CREATE TABLE project_settings (project_id TEXT); INSERT INTO project_settings VALUES ('p'); CREATE VIEW public_project_settings AS SELECT project_id FROM project_settings;").unwrap();
        conn.execute("INSERT INTO public_project_origin VALUES ('Ingest Sony ILME-FX6V 2026-11-24 14:06:29 3370')", []).unwrap();
        assert!(is_ingest_base(&conn));
        let db = ProjectDb::open(&file, "qnc://local/db/project_db/p", Access::ReadWrite, Vec::new()).unwrap();
        assert!(db.with_companion(&draft_file(1)).is_err(), "no S-Story beside a base");
        assert!(!dir.path().join(draft_file(1)).exists());
        conn.execute("UPDATE public_project_origin SET project_name = 'Poplava u Mostaru'", []).unwrap();
        assert!(!is_ingest_base(&conn));
    }
}
