//! The Ingest bases of the application (user 2026-11-11: "na nivou aplikacije imamo globalni
//! direktorij u kojem se kreiraju ingest baze kao samostalne jedinice; kada korisnik kreira svoj
//! projekt, taj projekt uzima tu bazu za svoju ingest bazu"). The folder is `data/ingest` of the
//! QNC-AI root. An Ingest base is made by QNC's Project block (`qnc-project-store`, copied here):
//! its own folder and database with the work settings of the selected template, registered in the
//! root's registry and made the active one, so the copied Ingest, its player and its background
//! worker work in it exactly as they do in a QNC project. Nothing else is written.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use qnc_application_catalog::{SelectedApplication, SelectionSnapshot};
use qnc_project_store::ProjectStore;

/// The folder of the Ingest bases in a QNC-AI root.
pub const BASES_DIR: &str = "data/ingest";
/// The name every Ingest base starts with.
pub const NAME_PREFIX: &str = "Ingest";

/// An Ingest base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestBase {
    pub id: String,
    pub name: String,
    pub uri: String,
    /// The project that took this base as its Ingest base (written by the Project form).
    pub project_name: Option<String>,
    /// The base Ingest works in now.
    pub active: bool,
}

/// The table of a base's own database that says which project took it (one row).
const PROJECT_TABLE: &str = "CREATE TABLE IF NOT EXISTS ingest_base_project (
    one INTEGER PRIMARY KEY CHECK (one = 1),
    project_name TEXT NOT NULL,
    written_at_utc TEXT NOT NULL
)";

fn base_of(row: qnc_project_store::ProjectRow, store: &ProjectStore) -> IngestBase {
    let project_name = project_name_in(store, &row.project_id).ok().flatten();
    IngestBase { id: row.project_id, name: row.name, uri: row.project_uri, project_name, active: row.active }
}

fn project_name_in(store: &ProjectStore, id: &str) -> Result<Option<String>, String> {
    let (file, _) = store.project_db_binding(id)?;
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
    let has: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='ingest_base_project')", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if !has {
        return Ok(None);
    }
    match conn.query_row("SELECT project_name FROM ingest_base_project WHERE one = 1", [], |row| row.get(0)) {
        Ok(name) => Ok(Some(name)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

/// Writes the name of the project that took the base `id` (the Project form calls it when a story's
/// project takes this base); a later project name replaces the earlier.
pub fn set_project_name(root: &Path, id: &str, project_name: &str) -> Result<(), String> {
    let store = ProjectStore::open(root)?;
    let (file, _) = store.project_db_binding(id)?;
    let mut conn = rusqlite::Connection::open(&file).map_err(|error| error.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|error| error.to_string())?;
    let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate).map_err(|error| error.to_string())?;
    tx.execute_batch(PROJECT_TABLE).map_err(|error| error.to_string())?;
    tx.execute(
        "INSERT INTO ingest_base_project (one, project_name, written_at_utc) VALUES (1, ?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))
         ON CONFLICT(one) DO UPDATE SET project_name = excluded.project_name, written_at_utc = excluded.written_at_utc",
        [project_name.trim()],
    )
    .map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())
}

/// Whether the project Ingest works in now is an Ingest base: a read only look at the registry
/// (`data/qnc-projects.db`, its public views), quick enough for the form's thread. No registry or
/// no active project is no base.
pub fn active_is_base(root: &Path) -> bool {
    let file = root.join("data").join("qnc-projects.db");
    let Ok(conn) = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) else { return false };
    conn.query_row(
        "SELECT name FROM public_projects WHERE project_id = (SELECT value FROM public_app_settings WHERE key = 'active_project_id')",
        [],
        |row| row.get::<_, String>(0),
    )
    .is_ok_and(|name| name.starts_with(NAME_PREFIX))
}

/// The project templates of `root` (id, name) and the index of the one selected.
pub fn templates(root: &Path) -> Result<(Vec<(String, String)>, usize), String> {
    let rows = ProjectStore::open(root)?.list_project_templates()?;
    let chosen = rows.iter().position(|row| row.selected).unwrap_or(0);
    Ok((rows.into_iter().map(|row| (row.template_id, row.name)).collect(), chosen))
}

/// The folder of the stories' projects of a QNC-AI root (not the bases' folder).
pub const PROJECTS_DIR: &str = "projects";

/// The base Ingest works in now (the active one), without making one.
pub fn active(root: &Path) -> Result<Option<IngestBase>, String> {
    let store = ProjectStore::open(root)?;
    Ok(store.list_projects()?.into_iter().find(|row| row.active && row.name.starts_with(NAME_PREFIX)).map(|row| base_of(row, &store)))
}

/// A story's project made with `name` that takes the base `base_id` as its Ingest base: the
/// project is made by the Project block in the projects folder, the base gets the project's name,
/// the project the base it took (`story_ingest_base` in its own database); the base stays the one
/// Ingest works in. Returns the project's id.
pub fn take_for_story(root: &Path, base_id: &str, name: &str, template_id: &str) -> Result<String, String> {
    let name = name.trim();
    if name.starts_with(NAME_PREFIX) {
        return Err(format!("a story's name cannot start with \"{NAME_PREFIX}\""));
    }
    let mut store = ProjectStore::open(root)?;
    let base = store.list_projects()?.into_iter().find(|row| row.project_id == base_id).ok_or_else(|| format!("no Ingest base {base_id}"))?;
    store.set_projects_root(root.join(PROJECTS_DIR))?;
    let made = store.create_project(name, template_id, None, &selection());
    // The bases' folder and the base come back whatever happened.
    let back = store.set_projects_root(folder(root)).and_then(|_| store.open_project(base_id));
    let made = made?;
    back?;
    let (file, _) = store.project_db_binding(&made.project_id)?;
    let conn = rusqlite::Connection::open(&file).map_err(|error| error.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|error| error.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS story_ingest_base (
            one INTEGER PRIMARY KEY CHECK (one = 1),
            base_id TEXT NOT NULL,
            base_uri TEXT NOT NULL,
            taken_at_utc TEXT NOT NULL
        )",
    )
    .map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT INTO story_ingest_base (one, base_id, base_uri, taken_at_utc) VALUES (1, ?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ','now'))
         ON CONFLICT(one) DO UPDATE SET base_id = excluded.base_id, base_uri = excluded.base_uri, taken_at_utc = excluded.taken_at_utc",
        [base.project_id.as_str(), base.project_uri.as_str()],
    )
    .map_err(|error| error.to_string())?;
    set_project_name(root, base_id, name)?;
    Ok(made.project_id)
}

/// Makes the base `id` the one Ingest works in.
pub fn open(root: &Path, id: &str) -> Result<IngestBase, String> {
    let store = ProjectStore::open(root)?;
    store.open_project(id)?;
    store.list_projects()?.into_iter().find(|row| row.project_id == id).map(|row| base_of(row, &store)).ok_or_else(|| format!("no Ingest base {id}"))
}

/// The folder of the Ingest bases of `root`.
pub fn folder(root: &Path) -> PathBuf {
    root.join(BASES_DIR)
}

/// The Ingest base Ingest works in now: the active one when it is an Ingest base, otherwise a new
/// one made and made active.
pub fn current(root: &Path) -> Result<IngestBase, String> {
    let mut store = ProjectStore::open(root)?;
    // The folder of the bases is set once (setting it locks it again every time).
    if Path::new(&store.projects_root_display()) != folder(root) {
        store.set_projects_root(folder(root))?;
    }
    if let Some(active) = store.list_projects()?.into_iter().find(|row| row.active && row.name.starts_with(NAME_PREFIX)) {
        return Ok(base_of(active, &store));
    }
    let made = store.create_project(&format!("{NAME_PREFIX} {}", stamp()), "", None, &selection())?;
    Ok(base_of(made, &store))
}

/// What came of a list or an open done in the background.
#[derive(Debug)]
pub enum BaseNews {
    Listed(Result<Vec<IngestBase>, String>),
    Opened(Result<IngestBase, String>),
}

/// The bases listed and opened off the form's thread (the registry and the project databases are
/// read there), one step at a time.
#[derive(Debug, Default)]
pub struct BaseBrowse {
    running: Option<std::sync::mpsc::Receiver<BaseNews>>,
}

impl BaseBrowse {
    fn run(&mut self, work: impl FnOnce() -> BaseNews + Send + 'static) {
        let (send, receive) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            let _ = send.send(work());
        });
        self.running = Some(receive);
    }

    /// Lists the bases of `root`.
    pub fn list(&mut self, root: &Path) {
        let root = root.to_path_buf();
        self.run(move || BaseNews::Listed(all(&root)));
    }

    /// Makes the base `id` of `root` the one Ingest works in.
    pub fn open(&mut self, root: &Path, id: &str) {
        let (root, id) = (root.to_path_buf(), id.to_string());
        self.run(move || BaseNews::Opened(open(&root, &id)));
    }

    pub fn busy(&self) -> bool {
        self.running.is_some()
    }

    /// The step finished since the last call.
    pub fn poll(&mut self) -> Option<BaseNews> {
        let news = self.running.as_ref()?.try_recv().ok()?;
        self.running = None;
        Some(news)
    }
}

/// The Ingest bases of `root`, newest first.
pub fn all(root: &Path) -> Result<Vec<IngestBase>, String> {
    let store = ProjectStore::open(root)?;
    let mut bases: Vec<_> = store.list_projects()?.into_iter().filter(|row| row.name.starts_with(NAME_PREFIX)).map(|row| base_of(row, &store)).collect();
    bases.sort_by(|a, b| b.name.cmp(&a.name));
    Ok(bases)
}

fn stamp() -> String {
    let seconds = SystemTime::now().duration_since(UNIX_EPOCH).map(|time| time.as_secs()).unwrap_or(0);
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    let (year, month, day) = civil(days as i64);
    format!("{year:04}-{month:02}-{day:02} {:02}:{:02}:{:02} UTC", rest / 3600, rest % 3600 / 60, rest % 60)
}

/// Days since 1970-01-01 to a date (Howard Hinnant's algorithm).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let month = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(month <= 2), month, day)
}

/// The applications of an Ingest base: Project only (it is no story).
fn selection() -> SelectionSnapshot {
    SelectionSnapshot {
        schema_version: 1,
        catalog_uri: qnc_application_catalog::DEFAULT_URI.into(),
        observed_at_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH).map(|time| time.as_millis() as u64).unwrap_or(0),
        applications: vec![SelectedApplication { application_id: "qnc.project".into(), tab_id: "project".into(), label: "project".into(), priority_group: "a".into() }],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_day_count_is_its_date() {
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_403), (2025, 11, 11));
    }

    /// In a QNC-AI root made of this repo's contracts and seed (a temporary folder, never a real
    /// one), the first base is made under data/ingest and made active, and is the current one after.
    #[test]
    fn the_first_base_is_made_in_the_folder_and_stays_current() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = std::env::temp_dir().join(format!("qnc-ingest-base-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["contracts", "seed", "catalogs"] {
            copy(&repo.join(dir), &root.join(dir));
        }
        std::fs::write(root.join("AGENTS.md"), "test\n").unwrap();
        std::fs::write(root.join("Cargo.toml"), "# test\n").unwrap();
        let base = current(&root).unwrap();
        assert!(base.name.starts_with(NAME_PREFIX));
        assert_eq!(current(&root).unwrap(), base, "the active base stays the current one");
        assert_eq!(base.project_name, None);
        set_project_name(&root, &base.id, "Prilog o luci").unwrap();
        assert_eq!(all(&root).unwrap()[0].project_name.as_deref(), Some("Prilog o luci"), "the project that took it");
        assert!(open(&root, &base.id).unwrap().active);
        let (templates, chosen) = templates(&root).unwrap();
        assert!(!templates.is_empty(), "the seed's templates");
        let story = take_for_story(&root, &base.id, "2026/11/11 Luka", &templates[chosen].0).unwrap();
        let after = active(&root).unwrap().unwrap();
        assert_eq!((after.id.as_str(), after.project_name.as_deref()), (base.id.as_str(), Some("2026/11/11 Luka")), "the base stays Ingest's and knows its project");
        assert_eq!(all(&root).unwrap().len(), 1, "the story's project is no base");
        assert!(!story.is_empty());
        assert!(std::fs::read_dir(folder(&root)).unwrap().count() >= 1);
        let store = ProjectStore::open(&root).unwrap();
        for row in store.list_projects().unwrap() {
            store.delete_project(&row.project_id).unwrap();
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    fn copy(from: &Path, to: &Path) {
        std::fs::create_dir_all(to).unwrap();
        for entry in std::fs::read_dir(from).unwrap().flatten() {
            let target = to.join(entry.file_name());
            if entry.path().is_dir() {
                copy(&entry.path(), &target);
            } else {
                std::fs::copy(entry.path(), target).unwrap();
            }
        }
    }
}
