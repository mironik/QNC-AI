//! The index of a base as a card (user 2026-11-15: "tray-ingest stvara kvazi karticu koju mi
//! nazivamo baza; ta kvazi kartica je u biti standardizirana kvazi QNC-kartica"): what the tray's
//! import copied into the base, written beside its database as `QNCCARD.JSON` with each clip's
//! finished record in `QNCCARD/`, in the format of `qnc-camera-qnc-base`. QNC's Select then reads
//! the base as any card and a project links its media. Only the tray writes it; a project never
//! writes a base.

use std::path::{Path, PathBuf};

use qnc_camera_qnc_base::{record_path, BaseCardClip, BaseCardIndex, INDEX_FILE, RECORD_FOLDER};
use qnc_project_store::ProjectStore;

/// A clip of the base as its database keeps it.
struct Kept {
    id: String,
    name: String,
    media_uri: String,
    poster_uri: Option<String>,
    record: String,
}

/// Writes the card index of the base `base_id` from what its import made: every clip copied into
/// the base with a finished record. Gives how many clips the index lists.
pub fn write_card_index(root: &Path, base_id: &str) -> Result<usize, String> {
    let store = ProjectStore::open(root)?;
    let (file, _) = store.project_db_binding(base_id)?;
    write_card_index_in(file.parent().ok_or("the base has no folder")?)
}

/// [`write_card_index`] of the base in `folder` (its database `project.db` there).
pub fn write_card_index_in(folder: &Path) -> Result<usize, String> {
    let kept = read_kept(&folder.join("project.db"))?;
    let mut clips = Vec::new();
    let records = folder.join(RECORD_FOLDER);
    std::fs::create_dir_all(&records).map_err(|error| format!("{}: {error}", records.display()))?;
    for clip in kept {
        let Some(original) = inside(&folder, &clip.media_uri) else { continue };
        let record = record_path(&clip.id);
        let at = folder.join(&record);
        std::fs::write(&at, &clip.record).map_err(|error| format!("{}: {error}", at.display()))?;
        clips.push(BaseCardClip {
            poster: clip.poster_uri.as_deref().and_then(|uri| inside(&folder, uri)),
            id: clip.id,
            name: clip.name,
            original,
            proxy: None,
            record,
        });
    }
    let count = clips.len();
    let index = folder.join(INDEX_FILE);
    std::fs::write(&index, BaseCardIndex::new(clips).text()).map_err(|error| format!("{}: {error}", index.display()))?;
    Ok(count)
}

/// The base `base_id` as the card a project Selects: its source (its folder,
/// `qnc://local/source/project-<folder>`) and the file names of `clips` there, as Select names the
/// clips of that card. A base made before it had an index gets one first.
pub fn card_of(root: &Path, base_id: &str, clips: &[String]) -> Result<(String, Vec<String>), String> {
    let store = ProjectStore::open(root)?;
    let (file, _) = store.project_db_binding(base_id)?;
    let folder = file.parent().ok_or("the base has no folder")?;
    if !folder.join(INDEX_FILE).is_file() {
        write_card_index_in(folder)?;
    }
    let name = folder.file_name().ok_or("the base has no folder")?.to_string_lossy().to_string();
    let files = read_kept(&file)?
        .into_iter()
        .filter(|kept| clips.contains(&kept.id))
        .filter_map(|kept| inside(folder, &kept.media_uri))
        .filter_map(|relative| relative.rsplit('/').next().map(str::to_owned))
        .collect();
    Ok((format!("qnc://local/source/project-{name}"), files))
}

/// The clips of the base's database copied into the base, with their finished record.
fn read_kept(file: &Path) -> Result<Vec<Kept>, String> {
    let conn = rusqlite::Connection::open_with_flags(file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
    conn.busy_timeout(std::time::Duration::from_secs(5)).map_err(|error| error.to_string())?;
    let mut statement = conn
        .prepare(
            "SELECT c.clip_id, c.name, c.imported_media_uri, c.thumbnail_uri, p.probe_json
             FROM clips c JOIN probe_records p ON p.clip_id = c.clip_id
             WHERE c.final = 1 AND c.import_status = 'imported' AND c.imported_media_uri IS NOT NULL
             ORDER BY c.name, c.clip_id",
        )
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| Ok(Kept { id: row.get(0)?, name: row.get(1)?, media_uri: row.get(2)?, poster_uri: row.get(3)?, record: row.get(4)? }))
        .map_err(|error| error.to_string())?;
    rows.collect::<Result<_, _>>().map_err(|error| error.to_string())
}

/// The path of a file under the base's folder that the URI names, relative to the folder; none
/// for a file elsewhere (the card it came from).
fn inside(folder: &Path, uri: &str) -> Option<String> {
    let name = folder.file_name()?.to_string_lossy().to_string();
    // A product of the base (`qnc://local/project/<folder>/products/...`), or a medium of its own
    // source (`.../source/project-<folder>/file/...`).
    let relative = match uri.strip_prefix(&format!("qnc://local/project/{name}/")) {
        Some(rest) => rest.to_string(),
        None => {
            let reference = qnc_source_contract::SourceReference::from_uri(uri).ok()?;
            if !reference.source_uri().eq_ignore_ascii_case(&format!("qnc://local/source/project-{name}")) {
                return None;
            }
            reference.relative_path().to_string()
        }
    };
    let path: PathBuf = relative.split('/').collect();
    folder.join(&path).is_file().then_some(relative)
}
