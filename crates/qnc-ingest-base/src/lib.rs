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

mod card_index;
pub use card_index::{card_of, write_card_index, write_card_index_in};

/// The name every Ingest base starts with.
pub const NAME_PREFIX: &str = "Ingest";

/// An Ingest base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IngestBase {
    pub id: String,
    pub name: String,
    pub uri: String,
    /// The project that took this base last, read from the projects (user 2026-11-15: "baze
    /// kartica ne bi trebale biti vezane s projektima, nego projekti s bazom"): a project keeps the
    /// bases it took in its own database (`story_ingest_bases`); the base writes nothing of them,
    /// and one base may give several projects.
    pub project_name: Option<String>,
    /// The base Ingest works in now.
    pub active: bool,
    /// Its name and description as the user wrote them (empty: none yet).
    pub title: String,
    /// Where it was shot, as the user wrote it (a camera's GPS may fill it later).
    pub location: String,
    pub description: String,
    /// What the shot analysis says it holds (each term and caption once; empty: not yet).
    pub analysis: String,
    /// Who shot it: from the camera when it writes it, else from the story sheet of a project that
    /// took the base (empty: not known yet).
    pub operator: String,
}

/// Whether `row` is the Ingest base this process works in: the one the process was given (the
/// ingest of one card in its own base, user 2026-11-14), else the active one.
fn working(row: &qnc_project_store::ProjectRow) -> bool {
    row.name.starts_with(NAME_PREFIX)
        && match qnc_work_settings::pinned_project() {
            Some(id) => row.project_id == id,
            None => row.active,
        }
}

/// The table of a base's own database that says it is being deleted (one row): the processes
/// working in it end and let it go (with a base per card, closing the active project no longer
/// stops them).
const CLOSING_TABLE: &str = "CREATE TABLE IF NOT EXISTS ingest_base_closing (
    one INTEGER PRIMARY KEY CHECK (one = 1),
    asked_at_utc TEXT NOT NULL
)";

/// Says in the base `id` that it is being deleted.
pub fn mark_closing(root: &Path, id: &str) -> Result<(), String> {
    let store = ProjectStore::open(root)?;
    // Only a base: a story's project is never marked (user 2026-11-27: the delete of a card's
    // partial import marked the story the header made and closed it).
    require_base(&store, id)?;
    let (file, _) = store.project_db_binding(id)?;
    let conn = rusqlite::Connection::open(&file).map_err(|error| error.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|error| error.to_string())?;
    conn.execute_batch(CLOSING_TABLE).map_err(|error| error.to_string())?;
    conn.execute("INSERT OR REPLACE INTO ingest_base_closing (one, asked_at_utc) VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))", [])
        .map_err(|error| error.to_string())?;
    Ok(())
}

/// Whether the base `id` is being deleted, or is gone: its processes end. A base that cannot be
/// read is not said to close (a passing lock).
pub fn closing(root: &Path, id: &str) -> bool {
    let Ok(store) = ProjectStore::open(root) else { return false };
    let Ok(rows) = store.list_projects() else { return false };
    if !rows.iter().any(|row| row.project_id == id) {
        return true;
    }
    let Ok((file, _)) = store.project_db_binding(id) else { return false };
    let Ok(conn) = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) else { return false };
    conn.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='ingest_base_closing')", [], |row| row.get::<_, bool>(0))
        .unwrap_or(false)
}

/// Who took each base, read from the projects (a project knows its bases, a base knows none):
/// base id -> (when it was taken last, the project's name). A project that cannot be read is left
/// out.
type Takers = std::collections::HashMap<String, (String, String)>;

fn takers(store: &ProjectStore) -> Takers {
    let mut takers = Takers::new();
    let Ok(rows) = store.list_projects() else { return takers };
    for row in rows.iter().filter(|row| !row.name.starts_with(NAME_PREFIX)) {
        for (base, at) in story_bases(store, &row.project_id).unwrap_or_default() {
            if takers.get(&base).is_none_or(|(known, _)| *known < at) {
                takers.insert(base, (at, row.name.clone()));
            }
        }
    }
    takers
}

/// The bases the story `story_id` took, with when (its `story_ingest_bases`), newest first.
/// The Ingest bases the story `story_id` took (their ids), newest first.
pub fn bases_of_story(root: &Path, story_id: &str) -> Result<Vec<String>, String> {
    Ok(story_bases(&ProjectStore::open(root)?, story_id)?.into_iter().map(|(base, _)| base).collect())
}

fn story_bases(store: &ProjectStore, story_id: &str) -> Result<Vec<(String, String)>, String> {
    let (file, _) = store.project_db_binding(story_id)?;
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
    let has: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='story_ingest_bases')", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if !has {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare("SELECT base_id, taken_at_utc FROM story_ingest_bases ORDER BY taken_at_utc DESC").map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).map_err(|error| error.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|error| error.to_string())
}

/// The table of a base's own database with what the base is (user 2026-11-15: "bazi trebamo dodati
/// pored datuma i vremena kreiranja i opis"; "polje koje smo koristili za naziv projekta koristit
/// ćemo za naziv baze"): its name and description, as the user wrote them in the ingest window.
/// They say what the card holds, nothing of a project.
const INFO_TABLE: &str = "CREATE TABLE IF NOT EXISTS ingest_base_info (
    one INTEGER PRIMARY KEY CHECK (one = 1),
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    written_at_utc TEXT NOT NULL
)";

/// What the shot analysis says the base holds (user 2026-11-15: "velike kuće ne brišu baze, one
/// ostaju arhiva; u opis dodaj podatke koje kreira alat za analizu kadrova, bez ponavljanja istih
/// pojmova i opisa"): a column of its own beside the user's description, so neither overwrites
/// the other.
const ANALYSIS_COLUMN: &str = "analysis";
/// Where the card was shot (user 2026-11-15: "u bazu bi trebalo dodati još jedno polje, mjesto-
/// lokacija; neke kamere imaju GPS"): typed by the user now, a camera's GPS may fill it later.
const LOCATION_COLUMN: &str = "location";
/// The columns added to the table after it was first made; a base made before gets each when it
/// is first written, and reads as empty until then.
/// Who shot the card (user 2026-11-18: the journalist names the camera operator in the story
/// sheet; a camera that writes it names it first): written once, never overwritten.
const OPERATOR_COLUMN: &str = "operator";
const ADDED_COLUMNS: [&str; 3] = [ANALYSIS_COLUMN, LOCATION_COLUMN, OPERATOR_COLUMN];

/// The info table of a base's database open for writing, with every added column.
fn info_writer(root: &Path, id: &str) -> Result<rusqlite::Connection, String> {
    info_writer_in(&ProjectStore::open(root)?, id)
}

/// As `info_writer`, with the store already open.
fn info_writer_in(store: &ProjectStore, id: &str) -> Result<rusqlite::Connection, String> {
    let (file, _) = store.project_db_binding(id)?;
    let conn = rusqlite::Connection::open(&file).map_err(|error| error.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|error| error.to_string())?;
    conn.execute_batch(INFO_TABLE).map_err(|error| error.to_string())?;
    for column in ADDED_COLUMNS {
        let has: bool = conn
            .query_row("SELECT EXISTS(SELECT 1 FROM pragma_table_info('ingest_base_info') WHERE name = ?1)", [column], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        if !has {
            conn.execute_batch(&format!("ALTER TABLE ingest_base_info ADD COLUMN {column} TEXT")).map_err(|error| error.to_string())?;
        }
    }
    Ok(conn)
}

/// Writes what the shot analysis says the base `id` holds; what the user wrote stays.
pub fn set_analysis(root: &Path, id: &str, analysis: &str) -> Result<(), String> {
    info_writer(root, id)?
        .execute(
            "INSERT INTO ingest_base_info (one, title, description, written_at_utc, analysis) VALUES (1, '', '', strftime('%Y-%m-%dT%H:%M:%fZ','now'), ?1)
             ON CONFLICT(one) DO UPDATE SET analysis = excluded.analysis",
            [analysis.trim()],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// What a base's info table says (empty where nothing is written).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Info {
    title: String,
    location: String,
    description: String,
    analysis: String,
    operator: String,
}

/// The info of the base `id`, read only.
fn info_in(store: &ProjectStore, id: &str) -> Info {
    let read = || -> Result<Info, String> {
        let (file, _) = store.project_db_binding(id)?;
        let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
        let columns: Vec<String> = conn
            .prepare("SELECT name FROM pragma_table_info('ingest_base_info')")
            .and_then(|mut statement| statement.query_map([], |row| row.get(0))?.collect())
            .map_err(|error| error.to_string())?;
        if columns.is_empty() {
            return Ok(Info::default());
        }
        let added = |column: &str| if columns.iter().any(|name| name == column) { format!("COALESCE({column}, '')") } else { "''".into() };
        conn.query_row(
            &format!("SELECT title, description, {}, {}, {} FROM ingest_base_info WHERE one = 1", added(LOCATION_COLUMN), added(ANALYSIS_COLUMN), added(OPERATOR_COLUMN)),
            [],
            |row| Ok(Info { title: row.get(0)?, description: row.get(1)?, location: row.get(2)?, analysis: row.get(3)?, operator: row.get(4)? }),
        )
        .map_err(|error| error.to_string())
    };
    read().unwrap_or_default()
}

/// Writes the name, the place and the description of the base `id` (the user's, in the ingest
/// window); the analysis stays.
pub fn set_info(root: &Path, id: &str, title: &str, location: &str, description: &str) -> Result<(), String> {
    info_writer(root, id)?
        .execute(
            "INSERT INTO ingest_base_info (one, title, description, written_at_utc, location) VALUES (1, ?1, ?2, strftime('%Y-%m-%dT%H:%M:%fZ','now'), ?3)
             ON CONFLICT(one) DO UPDATE SET title = excluded.title, description = excluded.description, location = excluded.location, written_at_utc = excluded.written_at_utc",
            [title.trim(), description.trim(), location.trim()],
        )
        .map(|_| ())
        .map_err(|error| error.to_string())
}

/// Writes `operator` as who shot the base `id` when it has no one yet (user 2026-11-18: the name a
/// camera wrote or one written before stays). Gives whether it wrote.
pub fn set_operator_if_empty(store: &ProjectStore, id: &str, operator: &str) -> Result<bool, String> {
    let operator = operator.trim();
    if operator.is_empty() {
        return Ok(false);
    }
    let conn = info_writer_in(store, id)?;
    conn.execute(
        "INSERT INTO ingest_base_info (one, title, description, written_at_utc, operator) VALUES (1, '', '', strftime('%Y-%m-%dT%H:%M:%fZ','now'), ?1)
         ON CONFLICT(one) DO UPDATE SET operator = excluded.operator WHERE COALESCE(ingest_base_info.operator, '') = ''",
        [operator],
    )
    .map(|changed| changed > 0)
    .map_err(|error| error.to_string())
}

/// The camera operator of the story `story_id` (its story sheet's `camera_operator`) given to every
/// base it took that has no one yet (user 2026-11-18: "novinar u Scriptu treba i tako navesti
/// snimatelja, pa veza baze i projekta može biti povezana"). Gives the bases written.
pub fn share_operator(store: &ProjectStore, story_id: &str) -> Result<usize, String> {
    let (file, _) = store.project_db_binding(story_id)?;
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
    let has: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE name = 'public_project_story_values')", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if !has {
        return Ok(0);
    }
    let value: Option<String> = conn
        .query_row("SELECT value_json FROM public_project_story_values WHERE code = ?1", [STORY_OPERATOR_FIELD], |row| row.get(0))
        .ok();
    drop(conn);
    let operator = value.and_then(|json| serde_json::from_str::<serde_json::Value>(&json).ok()).and_then(|value| value.as_str().map(str::to_string)).unwrap_or_default();
    if operator.trim().is_empty() {
        return Ok(0);
    }
    let mut written = 0;
    for (base_id, _) in story_bases(store, story_id)? {
        if set_operator_if_empty(store, &base_id, &operator)? {
            written += 1;
        }
    }
    Ok(written)
}

/// The story sheet field with the camera operator (the prepared sheets' `camera_operator`).
const STORY_OPERATOR_FIELD: &str = "camera_operator";

fn base_of(row: qnc_project_store::ProjectRow, takers: &Takers, store: &ProjectStore) -> IngestBase {
    let project_name = takers.get(&row.project_id).map(|(_, name)| name.clone());
    let Info { title, location, description, analysis, operator } = info_in(store, &row.project_id);
    IngestBase { id: row.project_id, name: row.name, uri: row.project_uri, project_name, active: row.active, title, location, description, analysis, operator }
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

/// Whether a story's project (a project, not an Ingest base) is active: the same read only look.
pub fn active_is_story(root: &Path) -> bool {
    let file = root.join("data").join("qnc-projects.db");
    let Ok(conn) = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY) else { return false };
    conn.query_row(
        "SELECT name FROM public_projects WHERE project_id = (SELECT value FROM public_app_settings WHERE key = 'active_project_id')",
        [],
        |row| row.get::<_, String>(0),
    )
    .is_ok_and(|name| !name.starts_with(NAME_PREFIX))
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
    Ok(store.list_projects()?.into_iter().find(|row| working(row)).map(|row| base_of(row, &takers(&store), &store)))
}

/// A story's project made with `name` that takes the base `base_id` as its Ingest base: the
/// project is made by the Project block where Settings put the projects (its database in its own
/// folder, as in QNC) and links the base (`story_ingest_bases` in its own database). The project is
/// the active one from now on: everything made in it goes into its own database, the base keeps
/// only what its import made (user 2026-11-15). Returns the project's id.
pub fn take_for_story(root: &Path, base_id: &str, name: &str, template_id: &str) -> Result<String, String> {
    let name = name.trim();
    if name.starts_with(NAME_PREFIX) {
        return Err(format!("a story's name cannot start with \"{NAME_PREFIX}\""));
    }
    let store = ProjectStore::open(root)?;
    let base = store.list_projects()?.into_iter().find(|row| row.project_id == base_id).ok_or_else(|| format!("no Ingest base {base_id}"))?;
    let made = store.create_project(name, template_id, None, &selection())?;
    write_story_base(&store, &made.project_id, &base)?;
    // A sheet already filled names the operator of the bases it took.
    let _ = share_operator(&store, &made.project_id);
    Ok(made.project_id)
}

/// The project of a story written on the base `base_id` (user 2026-11-24: "u brejking nevs ne pravi
/// se projekt na pocetku, projekt nastaje kada se otvara forma Script; naziv priloga postaje naziv
/// projekta"): made with `name` where Settings put the projects, from the selected template, it
/// takes the base and is the active one from now on. Returns its id and the base's card (the
/// source Select takes it from, as any card: the clips link to the base).
pub fn project_of_base(root: &Path, base_id: &str, name: &str) -> Result<(String, String), String> {
    let name = name.trim();
    if name.is_empty() {
        return Err("a story needs its name".into());
    }
    let story = take_for_story(root, base_id, name, "")?;
    let (card, _) = card_of(root, base_id, &[])?;
    Ok((story, card))
}

/// The base `base_id` named and described when the story `name` is made from it (user 2026-11-27:
/// "naziv baze i naziv projekta mogu biti isti ako naziv baze nije kreiran prilikom uvoza"; "Opis:
/// nalazi SigLIP-a, filtrirani da nema ponavljanja"): a base without a name the user wrote (empty,
/// or only its camera and time given when the ingest window closed) takes the story's name; an
/// empty description takes what the shot analysis found in it. What the user wrote stays.
pub fn name_base_by_story(root: &Path, base_id: &str, name: &str) -> Result<(), String> {
    let store = ProjectStore::open(root)?;
    require_base(&store, base_id)?;
    let row = store.list_projects()?.into_iter().find(|row| row.project_id == base_id).ok_or_else(|| format!("no Ingest base {base_id}"))?;
    let base = base_of(row, &Takers::new(), &store);
    let named = !base.title.trim().is_empty() && !base.name.contains(base.title.trim());
    let title = if named { base.title.clone() } else { name.trim().to_string() };
    let description = if base.description.trim().is_empty() { base.analysis.trim().to_string() } else { base.description.clone() };
    if title == base.title && description == base.description {
        return Ok(());
    }
    set_info(root, base_id, &title, &base.location, &description)
}

/// The tables of a base's own database a story's project takes when it is made (user 2026-11-27:
/// "u Ingestu je source materijal i rezultati AI analiza... bazu možeš kopirati, a kadrovi ostaju na
/// lokaciji uvoza"): the clips and their media records, the filmstrips and waves, the transcripts
/// and the statements. Not the base's settings, journal, runtime or work queues.
const BASE_TABLES: &[&str] = &[
    "clips",
    "clip_proxy",
    "clip_sources",
    "probe_records",
    "source_records",
    "media_acquisitions",
    "media_heads",
    "media_references",
    "media_snapshots",
    "evidence_documents",
    "snapshot_documents",
    "support_references",
    "filmstrip_artifacts",
    "filmstrip_frames",
    "wave_artifacts",
    "clip_transcripts",
    "virtual_shots",
    "virtual_shot_parts",
    "virtual_shot_numbers",
];
/// The tables of a base's AI file (`ai.db`) the story takes: the analysis of its shots and what
/// they show.
const BASE_AI_TABLES: &[&str] = &["clip_shots", "clip_content", "shot_label_vectors"];

/// The base `base_id`'s records copied into the story `story_id`'s own databases (its `project.db`
/// and `ai.db`): what the base holds of its source and its AI work; the media stay where the
/// import put them, the story links them. The base is only read. A row the story has already is
/// kept. Gives how many rows were copied.
pub fn copy_base_into_story(root: &Path, base_id: &str, story_id: &str) -> Result<usize, String> {
    let store = ProjectStore::open(root)?;
    require_base(&store, base_id)?;
    if store.list_projects()?.iter().any(|row| row.project_id == story_id && row.name.starts_with(NAME_PREFIX)) {
        return Err(format!("{story_id} is an Ingest base, not a story"));
    }
    let (base_file, _) = store.project_db_binding(base_id)?;
    let (story_file, _) = store.project_db_binding(story_id)?;
    copy_base_records(&base_file, &story_file)
}

/// The records of the base database `base_file` (and its `ai.db` beside it) copied into the story
/// database `story_file` (and its `ai.db`), as [`copy_base_into_story`] does.
pub fn copy_base_records(base_file: &Path, story_file: &Path) -> Result<usize, String> {
    let mut copied = copy_tables(base_file, story_file, BASE_TABLES)?;
    let base_ai = base_file.with_file_name("ai.db");
    if base_ai.is_file() {
        copied += copy_tables(&base_ai, &story_file.with_file_name("ai.db"), BASE_AI_TABLES)?;
    }
    Ok(copied)
}

/// The tables `tables` of `from` (opened read only) copied into `into`, in one transaction: a table
/// `into` lacks is made as `from` makes it (with its indexes), the rows go by the columns both have,
/// a row with a key `into` has already is left as it is.
fn copy_tables(from: &Path, into: &Path, tables: &[&str]) -> Result<usize, String> {
    let mut conn = rusqlite::Connection::open(into).map_err(|error| error.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(10)).map_err(|error| error.to_string())?;
    let source = format!("file:{}?mode=ro", from.to_string_lossy().replace('\\', "/"));
    conn.execute("ATTACH DATABASE ?1 AS base", [&source]).map_err(|error| error.to_string())?;
    // The tables reference each other (the media records): the rows go in as they are, unchecked.
    let _ = conn.execute_batch("PRAGMA foreign_keys = OFF");
    let result = (|| {
        let tx = conn.transaction().map_err(|error| error.to_string())?;
        let mut copied = 0;
        // First every table the story lacks, with its indexes, then the rows.
        for table in tables {
            let made: Option<String> = tx
                .query_row("SELECT sql FROM base.sqlite_master WHERE type = 'table' AND name = ?1", [table], |row| row.get(0))
                .ok();
            let Some(made) = made else { continue };
            let here: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM main.sqlite_master WHERE type = 'table' AND name = ?1)", [table], |row| row.get(0)).map_err(|error| error.to_string())?;
            if !here {
                tx.execute_batch(&made).map_err(|error| format!("{table}: {error}"))?;
                let mut indexes = tx.prepare("SELECT sql FROM base.sqlite_master WHERE type = 'index' AND tbl_name = ?1 AND sql IS NOT NULL").map_err(|error| error.to_string())?;
                let made_indexes: Vec<String> = indexes.query_map([table], |row| row.get(0)).map_err(|error| error.to_string())?.collect::<Result<_, _>>().map_err(|error| error.to_string())?;
                drop(indexes);
                for index in made_indexes {
                    tx.execute_batch(&index.replacen("CREATE INDEX", "CREATE INDEX IF NOT EXISTS", 1).replacen("CREATE UNIQUE INDEX", "CREATE UNIQUE INDEX IF NOT EXISTS", 1)).map_err(|error| format!("{table} index: {error}"))?;
                }
            }
        }
        for table in tables {
            let in_base: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM base.sqlite_master WHERE type = 'table' AND name = ?1)", [table], |row| row.get(0)).map_err(|error| error.to_string())?;
            if !in_base {
                continue;
            }
            let columns = |schema: &str| -> Result<Vec<String>, String> {
                let mut statement = tx.prepare(&format!("SELECT name FROM {schema}.pragma_table_info('{table}')")).map_err(|error| error.to_string())?;
                let names = statement.query_map([], |row| row.get::<_, String>(0)).map_err(|error| error.to_string())?;
                names.collect::<Result<Vec<_>, _>>().map_err(|error| error.to_string())
            };
            let theirs = columns("base")?;
            let shared: Vec<String> = columns("main")?.into_iter().filter(|column| theirs.contains(column)).map(|column| format!("\"{column}\"")).collect();
            if shared.is_empty() {
                continue;
            }
            let list = shared.join(", ");
            copied += tx.execute(&format!("INSERT OR IGNORE INTO main.\"{table}\" ({list}) SELECT {list} FROM base.\"{table}\""), []).map_err(|error| format!("{table}: {error}"))?;
        }
        tx.commit().map_err(|error| error.to_string())?;
        Ok(copied)
    })();
    let _ = conn.execute("DETACH DATABASE base", []);
    result
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

/// The story `story_id` takes the base `base` and the bases it links to (a card an earlier base
/// already had, user 2026-11-13).
fn write_story_base(store: &ProjectStore, story_id: &str, base: &qnc_project_store::ProjectRow) -> Result<(), String> {
    write_one_story_base(store, story_id, base)?;
    let links = links_in(store, &base.project_id)?;
    for row in store.list_projects()?.into_iter().filter(|row| links.contains(&row.project_id)) {
        write_one_story_base(store, story_id, &row)?;
    }
    Ok(())
}

/// The story `story_id` takes the base `base` (a story may take several: every card Ingest brought
/// for it), in `story_ingest_bases` of its own database.
fn write_one_story_base(store: &ProjectStore, story_id: &str, base: &qnc_project_store::ProjectRow) -> Result<(), String> {
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
         ON CONFLICT(base_id) DO UPDATE SET taken_at_utc = excluded.taken_at_utc",
        [base.project_id.as_str(), base.project_uri.as_str()],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// The story worked on last: the project that took a base last, a base other than `but` (its id
/// and name), read from the projects. User 2026-11-12: "ako projekt postoji onda korisnik može
/// nastaviti dalje ili kreirati novi projekt".
pub fn last_story(root: &Path, but: &str) -> Result<Option<(String, String)>, String> {
    let store = ProjectStore::open(root)?;
    let rows = store.list_projects()?;
    let Some((_, name)) = takers(&store).into_iter().filter(|(base, _)| base != but).map(|(_, taken)| taken).max() else { return Ok(None) };
    Ok(rows.into_iter().find(|row| row.name == name && !row.name.starts_with(NAME_PREFIX)).map(|row| (row.project_id, row.name)))
}

/// The base `base_id` goes on the story `story_id` (the user chose the project that exists): the
/// story links the base (again: the base it linked last) and becomes the active project; the base
/// writes nothing of the story.
pub fn continue_story(root: &Path, base_id: &str, story_id: &str) -> Result<String, String> {
    let store = ProjectStore::open(root)?;
    let rows = store.list_projects()?;
    let base = rows.iter().find(|row| row.project_id == base_id).ok_or_else(|| format!("no Ingest base {base_id}"))?;
    let story = rows.iter().find(|row| row.project_id == story_id).ok_or_else(|| format!("no project {story_id}"))?;
    write_story_base(&store, story_id, base)?;
    // The project is the active one: what is made in it goes into its own database.
    store.open_project(story_id)?;
    Ok(story.name.clone())
}

/// Refuses `id` when it is a listed project that is not an Ingest base (a story's project): nothing
/// meant for a base (closing, deleting) may touch it. An id not listed passes ('no Ingest base' then).
fn require_base(store: &ProjectStore, id: &str) -> Result<(), String> {
    match store.list_projects()?.into_iter().find(|row| row.project_id == id) {
        Some(row) if !row.name.starts_with(NAME_PREFIX) => Err(format!("{} is not an Ingest base", row.name)),
        _ => Ok(()),
    }
}

/// The base `id`, as it is (nothing is made active).
pub fn by_id(root: &Path, id: &str) -> Result<IngestBase, String> {
    let store = ProjectStore::open(root)?;
    require_base(&store, id)?;
    store.list_projects()?.into_iter().find(|row| row.project_id == id).map(|row| base_of(row, &takers(&store), &store)).ok_or_else(|| format!("no Ingest base {id}"))
}

/// Makes the base `id` the one Ingest works in.
pub fn open(root: &Path, id: &str) -> Result<IngestBase, String> {
    let store = ProjectStore::open(root)?;
    store.open_project(id)?;
    store.list_projects()?.into_iter().find(|row| row.project_id == id).map(|row| base_of(row, &takers(&store), &store)).ok_or_else(|| format!("no Ingest base {id}"))
}

/// Deletes the Ingest base `id` with its folder (the copied Project store lifts its protection
/// first). Only a base: a story's project is never deleted here.
pub fn forget(root: &Path, id: &str) -> Result<(), String> {
    // Only its owner or the administrator (the tray says why before it asks).
    may_delete(root, id).map_err(|refusal| format!("{refusal:?}"))?;
    let store = ProjectStore::open(root)?;
    let row = store.list_projects()?.into_iter().find(|row| row.project_id == id).ok_or_else(|| format!("no Ingest base {id}"))?;
    if !row.name.starts_with(NAME_PREFIX) {
        return Err(format!("{} is not an Ingest base", row.name));
    }
    store.delete_project(id).map(|_| ())
}

/// The folder of the bases is the setting of the computer the shared block reads and writes
/// (`data/ingest-bases.json`; classic QNC reads the same, user 2026-11-12).
pub use qnc_base_cards::{folder, set_folder};

/// The projects' folder is never the bases' folder (earlier versions of QNC-AI made the bases by
/// moving it): when they are one, the user is told to choose the projects' folder in Settings.
pub fn keep_projects_apart(root: &Path) -> Result<(), String> {
    keep_apart(&mut ProjectStore::open(root)?, root)
}

fn keep_apart(store: &mut ProjectStore, root: &Path) -> Result<(), String> {
    match folder(root) {
        Ok(bases) if Path::new(&store.projects_root_display()) == bases => {
            Err("the folder of the projects is the folder of the Ingest bases: choose another for the projects in Settings".into())
        }
        _ => Ok(()),
    }
}

/// The Ingest base Ingest works in now: the active one when it is an Ingest base no story has taken
/// yet, otherwise a new one made in the bases' folder and made active (user 2026-11-12: a base a
/// story took is that story's; the next Ingest starts the next story). The projects' folder of
/// Settings is left as it was.
pub fn current(root: &Path) -> Result<IngestBase, String> {
    current_for(root, "")
}

/// As `current`; a new base is named by `what` it takes (the camera of the card, user
/// 2026-11-14: "bazi umjesto teksta Ingest daj naziv kamere"): "Sony ILME-FX3A 2026-11-14 00:01:45 7c4c".
pub fn current_for(root: &Path, what: &str) -> Result<IngestBase, String> {
    let mut store = ProjectStore::open(root)?;
    let bases = folder(root)?;
    not_indexed(&bases);
    keep_apart(&mut store, root)?;
    if let Some(active) = store.list_projects()?.into_iter().find(|row| working(row)) {
        let base = base_of(active, &takers(&store), &store);
        if base.project_name.is_none() {
            return Ok(base);
        }
    }
    make(&mut store, &bases, what)
}

/// A new base for a card or folder, named by `what` it takes (its camera, "Folder"): each source
/// goes into its own base (user 2026-11-14: "različite kartice se trebaju uvoziti u svoje baze").
/// A base is a card QNC works with, not a project: the project is made later, in Ingest.
/// The project active before stays active: the base is worked in by the processes given it, and
/// what the user has open in QNC is not changed.
pub fn new_base(root: &Path, what: &str) -> Result<IngestBase, String> {
    let mut store = ProjectStore::open(root)?;
    let bases = folder(root)?;
    not_indexed(&bases);
    keep_apart(&mut store, root)?;
    let before = store.list_projects()?.into_iter().find(|row| row.active).map(|row| row.project_id);
    let mut made = make(&mut store, &bases, what)?;
    if let Some(before) = before {
        store.open_project(&before)?;
        made.active = false;
    }
    Ok(made)
}

/// The bases' folder marked "not content indexed", which what is made in it takes: the system's
/// search does not read every copied clip and picture while an ingest runs (audit 2026-11-18:
/// Windows Search took ~0.65 of a core during an ingest into C:\Users\Public). It is an attribute
/// of the bases' own folder, not a setting of the system; a folder it cannot mark stays as it is.
fn not_indexed(folder: &Path) {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{GetFileAttributesW, SetFileAttributesW, FILE_ATTRIBUTE_NOT_CONTENT_INDEXED, INVALID_FILE_ATTRIBUTES};
        let wide: Vec<u16> = folder.as_os_str().encode_wide().chain(Some(0)).collect();
        unsafe {
            let attributes = GetFileAttributesW(wide.as_ptr());
            if attributes != INVALID_FILE_ATTRIBUTES && attributes & FILE_ATTRIBUTE_NOT_CONTENT_INDEXED == 0 {
                SetFileAttributesW(wide.as_ptr(), attributes | FILE_ATTRIBUTE_NOT_CONTENT_INDEXED);
            }
        }
    }
    #[cfg(not(windows))]
    let _ = folder;
}

/// How the wizard imports a card into its base: a verified copy of its originals.
const WIZARD_COPIES: &str = "original";

/// A new base made in the bases' folder and made active, named by `what` it takes (empty: the
/// stamp alone). Its name keeps the prefix that makes it an Ingest base; what is shown is the label.
fn make(store: &mut ProjectStore, bases: &Path, what: &str) -> Result<IngestBase, String> {
    let projects = PathBuf::from(store.projects_root_display());
    store.set_projects_root(&bases)?;
    // A template may name its own projects' folder; a base is always in the bases' folder.
    let mut settings = store.selected_template_settings()?.settings;
    if let Some(storage) = settings.get_mut("storage").and_then(serde_json::Value::as_object_mut) {
        storage.insert("projects_root".into(), serde_json::Value::String(bases.to_string_lossy().into_owned()));
        // The wizard always copies the card to the disk, whatever the template says (user
        // 2026-11-16: "wizard importira karticu na disk, a aplikacija se linka na tu kopiju; drugi
        // put je aplikacija se linka na source karticu"): a link to the card is the Ingest form's.
        storage.insert("ingest_media".into(), serde_json::Value::String(WIZARD_COPIES.into()));
    }
    let what = what.trim();
    let name = match what.is_empty() {
        true => format!("{NAME_PREFIX} {}", stamp()),
        false => format!("{NAME_PREFIX} {what} {}", stamp()),
    };
    let made = store.create_ingest_base(&name, &settings, &selection());
    let back = store.set_projects_root(&projects);
    let made = made?;
    back?;
    Ok(base_of(made, &Takers::new(), store))
}

/// The table of a base's own database with the folders it was Selected from (not cards).
const SOURCES_TABLE: &str = "CREATE TABLE IF NOT EXISTS ingest_base_sources (
    label TEXT PRIMARY KEY,
    added_at_utc TEXT NOT NULL
)";

/// Writes that the base Ingest works in takes a folder (`label`, its path as the browser shows it;
/// user 2026-11-14: the tray tells where it imports from).
pub fn add_source(root: &Path, label: &str) -> Result<(), String> {
    let store = ProjectStore::open(root)?;
    let active = store.list_projects()?.into_iter().find(|row| working(row)).ok_or("no active Ingest base")?;
    let (file, _) = store.project_db_binding(&active.project_id)?;
    let conn = rusqlite::Connection::open(&file).map_err(|error| error.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|error| error.to_string())?;
    conn.execute_batch(SOURCES_TABLE).map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT INTO ingest_base_sources (label, added_at_utc) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(label) DO NOTHING",
        [label.trim()],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// The folders the base Ingest works in takes, in the order they were added; none without a base.
pub fn active_sources(root: &Path) -> Result<Vec<String>, String> {
    let store = ProjectStore::open(root)?;
    let Some(active) = store.list_projects()?.into_iter().find(|row| working(row)) else { return Ok(Vec::new()) };
    let (file, _) = store.project_db_binding(&active.project_id)?;
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
    let has: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='ingest_base_sources')", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if !has {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare("SELECT label FROM ingest_base_sources ORDER BY added_at_utc").map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| row.get::<_, String>(0)).map_err(|error| error.to_string())?;
    rows.collect::<rusqlite::Result<Vec<_>>>().map_err(|error| error.to_string())
}

/// A card inserted again that a base already has (user 2026-11-12: "treba traj testirati je li ta
/// kartica s tim klipovima već importirana, ako je, ima li novih klipova koji nisu u bazi";
/// "glupo je ponovo uvoziti ono što je uvezeno").
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KnownCard {
    /// The newest base with clips of this card.
    pub base: IngestBase,
    /// How many of the card's originals the base has.
    pub had: usize,
    /// The originals on the card the base has not (QNC URIs).
    pub new: Vec<String>,
    /// How many of the card's originals the base has only Selected, not imported (its ingest was
    /// stopped before the end).
    pub unfinished: usize,
}

/// The newest base that has clips of the card `source_uri` whose originals are among those on the
/// card now (`card`, QNC URIs), with what the card has new. `None`: no base has any of them (the
/// card is new, or formatted and shot again). Read only.
pub fn known_card(root: &Path, source_uri: &str, card: &[String]) -> Result<Option<KnownCard>, String> {
    known_among(root, source_uri, card, false)
}

/// The newest base other than the one Ingest works in now that has clips of the card
/// `source_uri` (a card put in while a base is being made, which an earlier base already has).
pub fn known_card_elsewhere(root: &Path, source_uri: &str, card: &[String]) -> Result<Option<KnownCard>, String> {
    known_among(root, source_uri, card, true)
}

fn known_among(root: &Path, source_uri: &str, card: &[String], but_active: bool) -> Result<Option<KnownCard>, String> {
    let store = ProjectStore::open(root)?;
    for base in all(root)?.into_iter().filter(|base| !(but_active && base.active)) {
        let Ok((file, _)) = store.project_db_binding(&base.id) else { continue };
        let clips: Vec<_> = qnc_content_read::clip_originals_of_file(&file)?.into_iter().filter(|clip| clip.source_uri == source_uri).collect();
        let had: std::collections::BTreeSet<&str> = clips.iter().map(|clip| clip.original_uri.as_str()).collect();
        let imported: std::collections::BTreeSet<&str> = clips.iter().filter(|clip| clip.imported).map(|clip| clip.original_uri.as_str()).collect();
        let common = card.iter().filter(|original| had.contains(original.as_str())).count();
        if common == 0 {
            continue;
        }
        let new = card.iter().filter(|original| !had.contains(original.as_str())).cloned().collect();
        let unfinished = card.iter().filter(|original| had.contains(original.as_str()) && !imported.contains(original.as_str())).count();
        return Ok(Some(KnownCard { base, had: common, new, unfinished }));
    }
    Ok(None)
}

/// The table of a base's own database with the bases it links to: an earlier base that already
/// has a card put in now (user 2026-11-13: "dodaje se samo linkovi na prethodno uvezenu bazu,
/// ponovo se ne uvozi"). A story that takes the base takes the linked bases too.
const LINKS_TABLE: &str = "CREATE TABLE IF NOT EXISTS ingest_base_links (
    base_id TEXT PRIMARY KEY,
    linked_at_utc TEXT NOT NULL
)";

/// The base Ingest works in now links to the base `to` (once).
pub fn link_active_to(root: &Path, to: &str) -> Result<(), String> {
    let store = ProjectStore::open(root)?;
    let active = store.list_projects()?.into_iter().find(|row| working(row)).ok_or("no active Ingest base")?;
    if active.project_id == to {
        return Ok(());
    }
    let (file, _) = store.project_db_binding(&active.project_id)?;
    let conn = rusqlite::Connection::open(&file).map_err(|error| error.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|error| error.to_string())?;
    conn.execute_batch(LINKS_TABLE).map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT INTO ingest_base_links (base_id, linked_at_utc) VALUES (?1, strftime('%Y-%m-%dT%H:%M:%fZ','now')) ON CONFLICT(base_id) DO NOTHING",
        [to],
    )
    .map_err(|error| error.to_string())?;
    Ok(())
}

/// The bases the base `id` links to (none when it links to none).
pub fn links(root: &Path, id: &str) -> Result<Vec<String>, String> {
    links_in(&ProjectStore::open(root)?, id)
}

fn links_in(store: &ProjectStore, id: &str) -> Result<Vec<String>, String> {
    let (file, _) = store.project_db_binding(id)?;
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
    let has: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='ingest_base_links')", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if !has {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare("SELECT base_id FROM ingest_base_links ORDER BY linked_at_utc").map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| row.get(0)).map_err(|error| error.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|error| error.to_string())
}

/// The table of a base's own database that says it takes only what base `from_base` has not (one row).
const ONLY_NEW_TABLE: &str = "CREATE TABLE IF NOT EXISTS ingest_base_only_new (
    one INTEGER PRIMARY KEY CHECK (one = 1),
    from_base TEXT NOT NULL,
    written_at_utc TEXT NOT NULL
)";

/// A new base, made active, that takes from the cards only the clips the base `from` has not.
/// It is named by `what` it takes (the camera), as `current_for`.
pub fn new_with_only_new(root: &Path, from: &IngestBase, what: &str) -> Result<IngestBase, String> {
    let mut store = ProjectStore::open(root)?;
    let bases = folder(root)?;
    not_indexed(&bases);
    keep_apart(&mut store, root)?;
    let made = make(&mut store, &bases, what)?;
    let (file, _) = store.project_db_binding(&made.id)?;
    let conn = rusqlite::Connection::open(&file).map_err(|error| error.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|error| error.to_string())?;
    conn.execute_batch(ONLY_NEW_TABLE).map_err(|error| error.to_string())?;
    conn.execute(
        "INSERT OR REPLACE INTO ingest_base_only_new (one, from_base, written_at_utc) VALUES (1, ?1, strftime('%Y-%m-%dT%H:%M:%fZ','now'))",
        [&from.id],
    )
    .map_err(|error| error.to_string())?;
    Ok(made)
}

/// The clips of the active base that are not taken (not imported): those whose original another
/// base already has, when the active base was made to take only the new ones. Empty otherwise.
pub fn not_taken(root: &Path) -> Result<std::collections::BTreeSet<String>, String> {
    let store = ProjectStore::open(root)?;
    let Some(active) = store.list_projects()?.into_iter().find(|row| working(row)) else {
        return Ok(Default::default());
    };
    not_taken_in(&store, &active.project_id)
}

/// `not_taken` of the base `id` (the tray reads every base being ingested).
pub fn not_taken_of(root: &Path, id: &str) -> Result<std::collections::BTreeSet<String>, String> {
    not_taken_in(&ProjectStore::open(root)?, id)
}

fn not_taken_in(store: &ProjectStore, id: &str) -> Result<std::collections::BTreeSet<String>, String> {
    let (file, _) = store.project_db_binding(id)?;
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
    let from: Option<String> = match conn.query_row("SELECT from_base FROM ingest_base_only_new WHERE one = 1", [], |row| row.get(0)) {
        Ok(from) => Some(from),
        Err(rusqlite::Error::QueryReturnedNoRows | rusqlite::Error::SqliteFailure(..)) => None,
        Err(error) => return Err(error.to_string()),
    };
    drop(conn);
    // The base it came from and the bases it links to: what they have is not taken again. A base
    // gone (deleted) takes nothing away.
    let mut had = std::collections::BTreeSet::new();
    for base in from.into_iter().chain(links_in(store, id)?) {
        let Ok((other, _)) = store.project_db_binding(&base) else { continue };
        had.extend(qnc_content_read::clip_originals_of_file(&other)?.into_iter().map(|clip| clip.original_uri));
    }
    if had.is_empty() {
        return Ok(Default::default());
    }
    Ok(qnc_content_read::clip_originals_of_file(&file)?.into_iter().filter(|clip| had.contains(&clip.original_uri)).map(|clip| clip.clip_id).collect())
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
    let taken = takers(&store);
    let mut bases: Vec<_> = store.list_projects()?.into_iter().filter(|row| row.name.starts_with(NAME_PREFIX)).map(|row| base_of(row, &taken, &store)).collect();
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

    /// The base as a list shows it: its name the user wrote, then its camera, date and time
    /// (user 2026-11-15), then who shot it when known (user 2026-11-18); without a name only the
    /// camera, date and time.
    pub fn shown(&self) -> String {
        let shown = match self.title.trim() {
            "" => self.label().to_string(),
            title => format!("{title} · {}", self.label()),
        };
        match self.operator.trim() {
            "" => shown,
            operator => format!("{shown} · {operator}"),
        }
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
        let base = IngestBase { id: "x".into(), name, uri: "u".into(), project_name: None, active: false, title: String::new(), location: String::new(), description: String::new(), analysis: String::new(), operator: String::new() };
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
        std::fs::create_dir_all(root.join("data")).unwrap();
        assert!(current(&root).unwrap_err().contains("not set"), "no folder of the bases is guessed");
        set_folder(&root, &root.join("bases")).unwrap();
        let base = current(&root).unwrap();
        assert!(base.name.starts_with(NAME_PREFIX));
        assert_eq!(current(&root).unwrap(), base, "the active base stays the current one");
        assert_eq!(base.project_name, None);
        set_info(&root, &base.id, " Konavle ", " Cavtat ", "javna rasprava").unwrap();
        let named = all(&root).unwrap().remove(0);
        assert_eq!((named.title.as_str(), named.location.as_str(), named.description.as_str()), ("Konavle", "Cavtat", "javna rasprava"), "the base's own name, place and description");
        set_analysis(&root, &base.id, "U kadrovima: Luka").unwrap();
        let described = all(&root).unwrap().remove(0);
        assert_eq!((described.analysis.as_str(), described.title.as_str()), ("U kadrovima: Luka", "Konavle"), "the analysis beside, the user's text kept");
        assert_eq!(named.shown(), format!("Konavle · {}", named.label()));
        assert!(open(&root, &base.id).unwrap().active);
        let (templates, chosen) = templates(&root).unwrap();
        assert!(!templates.is_empty(), "the seed's templates");
        let story = take_for_story(&root, &base.id, "2026/11/11 Luka", &templates[chosen].0).unwrap();
        let active_id = || ProjectStore::open(&root).unwrap().list_projects().unwrap().into_iter().find(|row| row.active).map(|row| row.project_id);
        assert_eq!(active_id().as_deref(), Some(story.as_str()), "the new project is the active one, with its own database (user 2026-11-15)");
        assert_eq!(all(&root).unwrap()[0].project_name.as_deref(), Some("2026/11/11 Luka"), "the base read as linked by it");
        assert_eq!(all(&root).unwrap().len(), 1, "the story's project is no base");
        // The camera operator the journalist names in the story sheet goes to the base; one written
        // before stays (user 2026-11-18).
        {
            let store = ProjectStore::open(&root).unwrap();
            let (story_db, _) = store.project_db_binding(&story).unwrap();
            let conn = rusqlite::Connection::open(&story_db).unwrap();
            conn.execute_batch(
                "CREATE TABLE IF NOT EXISTS project_story_values (code TEXT PRIMARY KEY, value_json TEXT NOT NULL);
                 CREATE VIEW IF NOT EXISTS public_project_story_values AS SELECT code, value_json FROM project_story_values;
                 INSERT INTO project_story_values (code, value_json) VALUES ('camera_operator', '\"Ivo Ivić\"');",
            )
            .unwrap();
            drop(conn);
            assert_eq!(share_operator(&store, &story).unwrap(), 1);
            assert_eq!(all(&root).unwrap()[0].operator, "Ivo Ivić", "the base knows who shot it");
            assert!(!set_operator_if_empty(&store, &base.id, "Drugi").unwrap(), "a name written before stays");
            assert_eq!(all(&root).unwrap()[0].operator, "Ivo Ivić");
        }
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let next = current(&root).unwrap();
        assert_ne!(next.id, base.id, "a base a story took is not the next Ingest's");
        assert_eq!(next.project_name, None);
        assert_eq!(last_story(&root, &next.id).unwrap(), Some((story.clone(), "2026/11/11 Luka".to_string())), "the story worked on last");
        let store = ProjectStore::open(&root).unwrap();
        let (next_db, _) = store.project_db_binding(&next.id).unwrap();
        assert_eq!(continue_story(&root, &next.id, &story).unwrap(), "2026/11/11 Luka");
        assert_eq!(active_id().as_deref(), Some(story.as_str()), "going on, the project is the active one");
        take_clips(&root, &story, &next.id, &["c1".into(), "c2".into()]).unwrap();
        take_clips(&root, &story, &next.id, &["c2".into()]).unwrap();
        assert_eq!(story_clips(&root, &story).unwrap(), vec![(next.id.clone(), "c1".into()), (next.id.clone(), "c2".into())], "the clips the story took, once each");
        assert_eq!(story_id(&root, "2026/11/11 Luka").unwrap().as_deref(), Some(story.as_str()));
        // A base gives several projects and writes nothing of them.
        let second = take_for_story(&root, &next.id, "2026/11/15 Druga", &templates[chosen].0).unwrap();
        assert_eq!(active_id().as_deref(), Some(second.as_str()));
        assert_eq!(last_story(&root, "").unwrap(), Some((second.clone(), "2026/11/15 Druga".to_string())));
        assert_eq!(open_story(&root, &story).unwrap(), "2026/11/11 Luka");
        assert_eq!(active_id().as_deref(), Some(story.as_str()), "a project opened is itself the active one");
        {
            let conn = rusqlite::Connection::open(&next_db).unwrap();
            let written: i64 = conn.query_row("SELECT count(*) FROM sqlite_master WHERE name IN ('ingest_base_project', 'story_clips', 'story_ingest_bases')", [], |row| row.get(0)).unwrap();
            assert_eq!(written, 0, "the base keeps nothing of a project");
            let (story_db, _) = store.project_db_binding(&story).unwrap();
            let conn = rusqlite::Connection::open(&story_db).unwrap();
            let linked: i64 = conn.query_row("SELECT count(*) FROM story_ingest_bases", [], |row| row.get(0)).unwrap();
            assert_eq!(linked, 2, "the story links both bases");
        }
        let store = ProjectStore::open(&root).unwrap();
        let (base_db, _) = store.project_db_binding(&base.id).unwrap();
        let (story_db, _) = store.project_db_binding(&story).unwrap();
        assert!(base_db.starts_with(folder(&root).unwrap()), "the base in the bases' folder: {}", base_db.display());
        assert_eq!(all(&root).unwrap().len(), 2, "the next base beside the one taken");
        assert!(story_db.starts_with(root.join("projects")), "the story's database in its project folder: {}", story_db.display());
        assert_eq!(PathBuf::from(store.projects_root_display()), root.join("projects"), "the projects' folder is left as it was");
        let elsewhere = root.join("other bases");
        set_folder(&root, &elsewhere).unwrap();
        assert_eq!(folder(&root).unwrap(), elsewhere, "Settings chose another folder for the bases");
        for row in store.list_projects().unwrap() {
            store.delete_project(&row.project_id).unwrap();
        }
        let _ = std::fs::remove_dir_all(&root);
    }

    /// A card a base has is known with what is new on it; a base made to take only the new clips
    /// leaves the clips that base has (a temporary root, never a real one).
    #[test]
    fn a_card_a_base_has_is_known_and_a_base_of_its_new_clips_leaves_the_old() {
        let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let root = std::env::temp_dir().join(format!("qnc-ingest-base-known-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for dir in ["contracts", "seed", "catalogs"] {
            copy(&repo.join(dir), &root.join(dir));
        }
        std::fs::write(root.join("AGENTS.md"), "test\n").unwrap();
        std::fs::write(root.join("Cargo.toml"), "# test\n").unwrap();
        std::fs::create_dir_all(root.join("data")).unwrap();
        set_folder(&root, &root.join("bases")).unwrap();
        let card = "qnc://local/source/volume-1";
        let file = |name: &str| format!("{card}/file/Clip/{name}");
        let clips = |id: &str, rows: &[(&str, &str)]| {
            let (db, _) = ProjectStore::open(&root).unwrap().project_db_binding(id).unwrap();
            let conn = rusqlite::Connection::open(db).unwrap();
            conn.execute_batch(
                "DROP VIEW IF EXISTS public_clips; CREATE TABLE test_clips (clip_id TEXT, source_uri TEXT, original_uri TEXT, clip_kind TEXT, import_status TEXT);
                 CREATE VIEW public_clips AS SELECT clip_id, source_uri, original_uri, clip_kind, import_status FROM test_clips;",
            )
            .unwrap();
            for (clip, original) in rows {
                conn.execute("INSERT INTO test_clips VALUES (?1, ?2, ?3, 'camera', CASE WHEN ?1 = 'b' THEN 'detected' ELSE 'imported' END)", [*clip, card, original]).unwrap();
            }
        };
        let first = current(&root).unwrap();
        clips(&first.id, &[("a", &file("A.MXF")), ("b", &file("B.MXF"))]);
        let on_card = vec![file("A.MXF"), file("B.MXF"), file("C.MXF")];
        let known = known_card(&root, card, &on_card).unwrap().unwrap();
        assert_eq!((known.base.id.as_str(), known.had, known.new.clone()), (first.id.as_str(), 2, vec![file("C.MXF")]));
        assert_eq!(known.unfinished, 1, "B is in the base only Selected: its ingest was stopped");
        assert_eq!(known_card(&root, card, &[file("X.MXF")]).unwrap(), None, "a card shot again after formatting is new");
        assert!(not_taken(&root).unwrap().is_empty(), "a base made as usual takes every clip");
        std::thread::sleep(std::time::Duration::from_millis(1100));
        let only_new = new_with_only_new(&root, &first, "Sony ILME-FX3A").unwrap();
        assert_eq!(only_new.label().split(' ').take(2).collect::<Vec<_>>(), ["Sony", "ILME-FX3A"], "named by its camera");
        assert!(only_new.active && only_new.id != first.id);
        clips(&only_new.id, &[("a2", &file("A.MXF")), ("b2", &file("B.MXF")), ("c2", &file("C.MXF"))]);
        assert_eq!(not_taken(&root).unwrap().into_iter().collect::<Vec<_>>(), vec!["a2".to_string(), "b2".to_string()], "what the first base has is not imported again");
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

/// Who made a base: the user and the computer of its origin (written by Project when the base was
/// made). `None` when the base has no origin.
pub fn owner(root: &Path, id: &str) -> Result<Option<(String, String)>, String> {
    let file = root.join("data").join("qnc-projects.db");
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
    match conn.query_row(
        "SELECT COALESCE(user_name, ''), COALESCE(workstation_name, '') FROM public_project_origin WHERE project_id = ?1",
        [id],
        |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
    ) {
        Ok(owner) => Ok(Some(owner)),
        Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
        Err(error) => Err(error.to_string()),
    }
}

/// Whether this user may delete the base `id` (user 2026-11-12: "baza je javna i mogu je koristiti
/// i drugi korisnici; vlasnik je onaj koji ju je kreirao, a brisati može jedino pored vlasnika
/// administrator"): the user and computer that made it, or a program running with the
/// administrator's rights. A base without an origin only the administrator deletes.
pub fn may_delete(root: &Path, id: &str) -> Result<(), Refusal> {
    if qnc_workstation_identity::is_administrator() {
        return Ok(());
    }
    match owner(root, id).map_err(Refusal::Unread)? {
        Some(owner) if is_mine(&owner) => Ok(()),
        Some((user, computer)) => Err(Refusal::NotOwner { user, computer }),
        None => Err(Refusal::NoOwner),
    }
}

/// Whether the owner (user, computer) is the user of this program on this computer.
fn is_mine((user, computer): &(String, String)) -> bool {
    let me = qnc_workstation_identity::read_local_identity();
    me.user_name.as_deref().is_some_and(|name| !name.is_empty() && name.eq_ignore_ascii_case(user))
        && me.workstation_name.as_deref().is_some_and(|name| !name.is_empty() && name.eq_ignore_ascii_case(computer))
}

/// The newest base that no story took yet and that is no link to another base (a card brought
/// again only links its first base): what a new project starts from (user 2026-11-15: a base of
/// 103 clips had no project after the start of QNC was declined, and the projects could not reach
/// it). `None` when every base has its story.
pub fn newest_without_story(root: &Path) -> Result<Option<IngestBase>, String> {
    let mut free = Vec::new();
    for base in all(root)? {
        if base.project_name.is_none() && links(root, &base.id)?.is_empty() {
            free.push(base);
        }
    }
    // The ids carry the time they were made.
    Ok(free.into_iter().max_by(|a, b| a.id.cmp(&b.id)))
}

/// The stories (projects that are no base) the user of this computer made, by their ids (user
/// 2026-11-12: a closed project the user goes on with is picked from the projects of the user).
pub fn my_stories(root: &Path) -> Result<std::collections::BTreeSet<String>, String> {
    let rows = ProjectStore::open(root)?.list_projects()?;
    let mut mine = std::collections::BTreeSet::new();
    for row in rows.into_iter().filter(|row| !row.name.starts_with(NAME_PREFIX)) {
        if owner(root, &row.project_id)?.is_some_and(|owner| is_mine(&owner)) {
            mine.insert(row.project_id);
        }
    }
    Ok(mine)
}

/// The story this user goes on with on the base `base_id` (user 2026-11-27: "ista baza može biti
/// izvor za više projekata... moraš gledati i aktivnog korisnika i status projekta"): of the
/// projects that took the base, the one taken last that this user on this computer made and that
/// is not being deleted. `None`: the user has no story on this base yet (the header's form is
/// asked). A project that cannot be read is left out.
pub fn my_story_of_base(root: &Path, base_id: &str) -> Result<Option<(String, String)>, String> {
    let store = ProjectStore::open(root)?;
    let mut found: Option<(String, String, String)> = None;
    for row in store.list_projects()?.into_iter().filter(|row| !row.name.starts_with(NAME_PREFIX)) {
        let Some(at) = story_bases(&store, &row.project_id).unwrap_or_default().into_iter().find(|(base, _)| base == base_id).map(|(_, at)| at) else { continue };
        if !owner(root, &row.project_id)?.is_some_and(|owner| is_mine(&owner)) || closing(root, &row.project_id) {
            continue;
        }
        if found.as_ref().is_none_or(|(known, _, _)| *known < at) {
            found = Some((at, row.project_id, row.name));
        }
    }
    Ok(found.map(|(_, id, name)| (id, name)))
}

/// The story `story_id` is worked on again: it becomes the active project (its own database, as
/// in QNC); the card bases it links stay as they are. Gives the story's name.
pub fn open_story(root: &Path, story_id: &str) -> Result<String, String> {
    // The project itself is opened: its own database holds what was made in it, the card bases
    // it links only what their import made (user 2026-11-15).
    let store = ProjectStore::open(root)?;
    let story = store.list_projects()?.into_iter().find(|row| row.project_id == story_id).ok_or_else(|| format!("no project {story_id}"))?;
    store.open_project(story_id)?;
    Ok(story.name)
}

/// Why this user may not delete a base.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// Another user or computer made it.
    NotOwner { user: String, computer: String },
    /// It has no origin: only the administrator deletes it.
    NoOwner,
    /// Its origin could not be read.
    Unread(String),
}
