//! The Ingest bases a story's project links, as sources (user 2026-11-27: "projekt pristupa bazi
//! po LINK"; the records copied from a base address their media in the base's own folder,
//! `qnc://local/source/project-<base>/file/...`, and the media stay where the import put them).
//! One thing only: for the project `project_id`, each base its own database names
//! (`story_ingest_bases`) as a source binding of this computer: its media address and its folder.
//! The callers add them to the sources they already have, as a card's; nothing else changes. A
//! base no longer listed is left out. Read only.

use std::path::Path;

use qnc_source_bindings::SourceBinding;

/// The media address of the project (or base) `id` on this computer, as the settings of a local
/// project make it (`qnc-work-settings` `project_media_source_uri`).
pub fn source_uri(id: &str) -> String {
    format!("qnc://local/source/project-{id}")
}

/// The bases the project `project_id` links, as sources: their media address, their folder.
pub fn of(root: &Path, project_id: &str) -> Result<Vec<SourceBinding>, String> {
    let store = qnc_project_store::ProjectStore::open(root)?;
    let (file, _) = store.project_db_binding(project_id)?;
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
    let has: bool = conn
        .query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = 'story_ingest_bases')", [], |row| row.get(0))
        .map_err(|error| error.to_string())?;
    if !has {
        return Ok(Vec::new());
    }
    let mut statement = conn.prepare("SELECT base_id FROM story_ingest_bases").map_err(|error| error.to_string())?;
    let bases: Vec<String> = statement.query_map([], |row| row.get(0)).map_err(|error| error.to_string())?.collect::<Result<_, _>>().map_err(|error| error.to_string())?;
    let listed: Vec<String> = store.list_projects()?.into_iter().map(|row| row.project_id).collect();
    Ok(bases
        .into_iter()
        .filter(|base| listed.contains(base))
        .filter_map(|base| store.project_folder(&base).ok().map(|folder| SourceBinding { uri: source_uri(&base), file: Some(folder), endpoint: None, token_env: None }))
        .collect())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_base_is_addressed_as_its_project_source() {
        assert_eq!(super::source_uri("ingest_x"), "qnc://local/source/project-ingest_x");
    }
}
