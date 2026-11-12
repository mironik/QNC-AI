//! The Ingest bases of the application (user 2026-11-11: "na nivou aplikacije imamo globalni
//! direktorij u kojem se kreiraju ingest baze kao samostalne jedinice; kada korisnik kreira svoj
//! projekt, taj projekt uzima tu bazu za svoju ingest bazu"). The folder is the one Settings chose
//! (`data/ingest-bases.json`), else `data/ingest` of the
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

/// The base Ingest works in now (the active one), without making one.
pub fn active(root: &Path) -> Result<Option<IngestBase>, String> {
    let store = ProjectStore::open(root)?;
    Ok(store.list_projects()?.into_iter().find(|row| row.active && row.name.starts_with(NAME_PREFIX)).map(|row| base_of(row, &store)))
}

/// A story's project made with `name` that takes the base `base_id` as its Ingest base: the
/// project is made by the Project block where Settings put the projects (its database in its own
/// folder, as in QNC), the base gets the project's name, the project the base it took
/// (`story_ingest_base` in its own database); the base stays the one Ingest works in. Returns the
/// project's id.
pub fn take_for_story(root: &Path, base_id: &str, name: &str, template_id: &str) -> Result<String, String> {
    let name = name.trim();
    if name.starts_with(NAME_PREFIX) {
        return Err(format!("a story's name cannot start with \"{NAME_PREFIX}\""));
    }
    let store = ProjectStore::open(root)?;
    let base = store.list_projects()?.into_iter().find(|row| row.project_id == base_id).ok_or_else(|| format!("no Ingest base {base_id}"))?;
    let made = store.create_project(name, template_id, None, &selection());
    // The base comes back as the one Ingest works in, whatever happened.
    let back = store.open_project(base_id);
    let made = made?;
    back?;
    write_story_base(&store, &made.project_id, &base)?;
    set_project_name(root, base_id, name)?;
    Ok(made.project_id)
}

/// The clips of the base `base_id` the user chose for the story `story_id` (Uvezi of the Ingest
/// form), in `story_clips` of the story's own database; the clips' source data stay in the base
/// (user 2026-11-12: the global base is a kind of media server, the project's own database keeps
/// what is done with them).
pub fn take_clips(root: &Path, story_id: &str, base_id: &str, clips: &[String]) -> Result<(), String> {
    let store = ProjectStore::open(root)?;
    let (file, _) = store.project_db_binding(story_id)?;
    let mut conn = rusqlite::Connection::open(&file).map_err(|error| error.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|error| error.to_string())?;
    let tx = conn.transaction().map_err(|error| error.to_string())?;
    tx.execute_batch(
        "CREATE TABLE IF NOT EXISTS story_clips (
            base_id TEXT NOT NULL,
            clip_id TEXT NOT NULL,
            taken_at_utc TEXT NOT NULL,
            PRIMARY KEY (base_id, clip_id)
        )",
    )
    .map_err(|error| error.to_string())?;
    for clip in clips {
        tx.execute(
            "INSERT INTO story_clips (base_id, clip_id, taken_at_utc) VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT DO NOTHING",
            [base_id, clip.as_str()],
        )
        .map_err(|error| error.to_string())?;
    }
    tx.commit().map_err(|error| error.to_string())
}

/// The clips a story took, as (base, clip).
pub fn story_clips(root: &Path, story_id: &str) -> Result<Vec<(String, String)>, String> {
    let store = ProjectStore::open(root)?;
    let (file, _) = store.project_db_binding(story_id)?;
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
    let exists: bool = conn.query_row("SELECT count(*) FROM sqlite_master WHERE name = 'story_clips'", [], |row| row.get::<_, i64>(0)).map_err(|error| error.to_string())? > 0;
    if !exists {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare("SELECT base_id, clip_id FROM story_clips ORDER BY taken_at_utc, clip_id").map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(|error| error.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|error| error.to_string())
}

/// The id of the story `name` (a project that is no base).
pub fn story_id(root: &Path, name: &str) -> Result<Option<String>, String> {
    Ok(ProjectStore::open(root)?.list_projects()?.into_iter().find(|row| row.name == name && !row.name.starts_with(NAME_PREFIX)).map(|row| row.project_id))
}

/// The story `story_id` takes the base `base` (a story may take several: every card Ingest brought
/// for it), in `story_ingest_bases` of its own database.
fn write_story_base(store: &ProjectStore, story_id: &str, base: &qnc_project_store::ProjectRow) -> Result<(), String> {
    let (file, _) = store.project_db_binding(story_id)?;
    let conn = rusqlite::Connection::open(&file).map_err(|error| error.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|error| error.to_string())?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS story_ingest_bases (
            base_id TEXT PRIMARY KEY,
            base_uri TEXT NOT NULL,
            taken_at_utc TEXT NOT NULL
        )",
    )
    .map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT INTO story_ingest_bases (base_id, base_uri, taken_at_utc) VALUES (?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ','now'))
         ON CONFLICT(base_id) DO NOTHING",
        [base.project_id.as_str(), base.project_uri.as_str()],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// The story worked on last: the project that took the newest base taken, other than `but` (its
/// id and name). User 2026-11-12: "ako projekt postoji onda korisnik može nastaviti dalje ili
/// kreirati novi projekt".
pub fn last_story(root: &Path, but: &str) -> Result<Option<(String, String)>, String> {
    let store = ProjectStore::open(root)?;
    let rows = store.list_projects()?;
    let Some(name) = all(root)?.into_iter().filter(|base| base.id != but).find_map(|base| base.project_name) else { return Ok(None) };
    Ok(rows.into_iter().find(|row| row.name == name && !row.name.starts_with(NAME_PREFIX)).map(|row| (row.project_id, row.name)))
}

/// The base `base_id` goes on the story `story_id` (the user chose the project that exists): the
/// base gets the story's name, the story the base; the base stays the one Ingest works in.
pub fn continue_story(root: &Path, base_id: &str, story_id: &str) -> Result<String, String> {
    let store = ProjectStore::open(root)?;
    let rows = store.list_projects()?;
    let base = rows.iter().find(|row| row.project_id == base_id).ok_or_else(|| format!("no Ingest base {base_id}"))?;
    let story = rows.iter().find(|row| row.project_id == story_id).ok_or_else(|| format!("no project {story_id}"))?;
    write_story_base(&store, story_id, base)?;
    set_project_name(root, base_id, &story.name)?;
    Ok(story.name.clone())
}

/// Makes the base `id` the one Ingest works in.
pub fn open(root: &Path, id: &str) -> Result<IngestBase, String> {
    let store = ProjectStore::open(root)?;
    store.open_project(id)?;
    store.list_projects()?.into_iter().find(|row| row.project_id == id).map(|row| base_of(row, &store)).ok_or_else(|| format!("no Ingest base {id}"))
}

/// Deletes the Ingest base `id` with its folder (the copied Project store lifts its protection
/// first). Only a base: a story's project is never deleted here.
pub fn forget(root: &Path, id: &str) -> Result<(), String> {
    let store = ProjectStore::open(root)?;
    let row = store.list_projects()?.into_iter().find(|row| row.project_id == id).ok_or_else(|| format!("no Ingest base {id}"))?;
    if !row.name.starts_with(NAME_PREFIX) {
        return Err(format!("{} is not an Ingest base", row.name));
    }
    store.delete_project(id).map(|_| ())
}

/// The setting of this computer that says where the Ingest bases are made (Settings writes it).
const FOLDER_SETTING: &str = "data/ingest-bases.json";

#[derive(serde::Serialize, serde::Deserialize)]
struct FolderSetting {
    folder: PathBuf,
}

/// The folder the Ingest bases of `root` are made in: the one Settings chose, else `data/ingest`.
pub fn folder(root: &Path) -> PathBuf {
    std::fs::read(root.join(FOLDER_SETTING))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<FolderSetting>(&bytes).ok())
        .map(|setting| if setting.folder.is_absolute() { setting.folder } else { root.join(setting.folder) })
        .unwrap_or_else(|| root.join(BASES_DIR))
}

/// The projects' folder is never the bases' folder: one left there (earlier versions of QNC-AI made
/// the bases by moving it) goes back to QNC's (`projects` of the root).
pub fn keep_projects_apart(root: &Path) -> Result<(), String> {
    keep_apart(&mut ProjectStore::open(root)?, root)
}

fn keep_apart(store: &mut ProjectStore, root: &Path) -> Result<(), String> {
    if Path::new(&store.projects_root_display()) == folder(root) {
        store.set_projects_root(root.join("projects"))?;
    }
    Ok(())
}

/// Settings: the Ingest bases are made in `folder` from now on (the bases made before stay where
/// they are; the registry knows each one's folder).
pub fn set_folder(root: &Path, folder: &Path) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let file = root.join(FOLDER_SETTING);
    let json = serde_json::to_vec_pretty(&FolderSetting { folder: folder.to_path_buf() }).map_err(|error| error.to_string())?;
    std::fs::write(&file, json).map_err(|error| format!("{}: {error}", file.display()))
}

/// The Ingest base Ingest works in now: the active one when it is an Ingest base no story has taken
/// yet, otherwise a new one made in the bases' folder and made active (user 2026-11-12: a base a
/// story took is that story's; the next Ingest starts the next story). The projects' folder of
/// Settings is left as it was.
pub fn current(root: &Path) -> Result<IngestBase, String> {
    let mut store = ProjectStore::open(root)?;
    let bases = folder(root);
    keep_apart(&mut store, root)?;
    if let Some(active) = store.list_projects()?.into_iter().find(|row| row.active && row.name.starts_with(NAME_PREFIX)) {
        let base = base_of(active, &store);
        if base.project_name.is_none() {
            return Ok(base);
        }
    }
    let projects = PathBuf::from(store.projects_root_display());
    store.set_projects_root(&bases)?;
    let made = store.create_project(&format!("{NAME_PREFIX} {}", stamp()), "", None, &selection());
    let back = store.set_projects_root(&projects);
    let made = made?;
    back?;
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

/// The time a base is made, on this computer's clock, and four random hex digits: bases made in
/// the same second (several computers on a LAN) are told apart.
fn stamp() -> String {
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map(|time| time.subsec_nanos()).unwrap_or(0);
    let random = (nanos ^ std::process::id().rotate_left(16)) & 0xffff;
    format!("{} {random:04x}", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"))
}

impl IngestBase {
    /// The base as it is shown: its time and number, without the word every base's name starts with.
    pub fn label(&self) -> &str {
        self.name.strip_prefix(NAME_PREFIX).map_or(self.name.as_str(), str::trim)
    }
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
    fn a_base_is_shown_by_its_time_and_number_without_the_word_ingest() {
        let name = format!("{NAME_PREFIX} {}", stamp());
        let base = IngestBase { id: "x".into(), name, uri: "u".into(), project_name: None, active: false };
        let label = base.label();
        assert!(!label.contains(NAME_PREFIX), "{label}");
        let (time, number) = label.rsplit_once(' ').unwrap();
        assert_eq!(time.len(), "2026-11-11 22:21:08".len(), "{label}");
        assert_eq!(number.len(), 4, "four random hex digits: {label}");
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
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let next = current(&root).unwrap();
        assert_ne!(next.id, base.id, "a base a story took is not the next Ingest's");
        assert_eq!(next.project_name, None);
        assert_eq!(last_story(&root, &next.id).unwrap(), Some((story.clone(), "2026/11/11 Luka".to_string())), "the story worked on last");
        assert_eq!(continue_story(&root, &next.id, &story).unwrap(), "2026/11/11 Luka");
        assert_eq!(active(&root).unwrap().unwrap().project_name.as_deref(), Some("2026/11/11 Luka"), "the next card goes on the same story");
        take_clips(&root, &story, &next.id, &["c1".into(), "c2".into()]).unwrap();
        take_clips(&root, &story, &next.id, &["c2".into()]).unwrap();
        assert_eq!(story_clips(&root, &story).unwrap(), vec![(next.id.clone(), "c1".into()), (next.id.clone(), "c2".into())], "the clips the story took, once each");
        assert_eq!(story_id(&root, "2026/11/11 Luka").unwrap().as_deref(), Some(story.as_str()));
        let store = ProjectStore::open(&root).unwrap();
        let (base_db, _) = store.project_db_binding(&base.id).unwrap();
        let (story_db, _) = store.project_db_binding(&story).unwrap();
        assert!(base_db.starts_with(folder(&root)), "the base in the bases' folder: {}", base_db.display());
        assert_eq!(all(&root).unwrap().len(), 2, "the next base beside the one taken");
        assert!(story_db.starts_with(root.join("projects")), "the story's database in its project folder: {}", story_db.display());
        assert_eq!(PathBuf::from(store.projects_root_display()), root.join("projects"), "the projects' folder is left as it was");
        let elsewhere = root.join("other bases");
        set_folder(&root, &elsewhere).unwrap();
        assert_eq!(folder(&root), elsewhere, "Settings chose another folder for the bases");
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
