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
}

/// The folder of the Ingest bases of `root`.
pub fn folder(root: &Path) -> PathBuf {
    root.join(BASES_DIR)
}

/// The Ingest base Ingest works in now: the active one when it is an Ingest base, otherwise a new
/// one made and made active.
pub fn current(root: &Path) -> Result<IngestBase, String> {
    let mut store = ProjectStore::open(root)?;
    store.set_projects_root(folder(root))?;
    if let Some(active) = store.list_projects()?.into_iter().find(|row| row.active && row.name.starts_with(NAME_PREFIX)) {
        return Ok(IngestBase { id: active.project_id, name: active.name, uri: active.project_uri });
    }
    let made = store.create_project(&format!("{NAME_PREFIX} {}", stamp()), "", None, &selection())?;
    Ok(IngestBase { id: made.project_id, name: made.name, uri: made.project_uri })
}

/// The Ingest bases of `root`.
pub fn all(root: &Path) -> Result<Vec<IngestBase>, String> {
    Ok(ProjectStore::open(root)?
        .list_projects()?
        .into_iter()
        .filter(|row| row.name.starts_with(NAME_PREFIX))
        .map(|row| IngestBase { id: row.project_id, name: row.name, uri: row.project_uri })
        .collect())
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
