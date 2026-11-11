//! The temporary work place where the cards of a story are looked at before the story has a
//! project (user 2026-11-11, choice B: "privremena baza za pregled"). It is a QNC root of its own
//! in the system's temp folder: the contracts and catalogs of the QNC root it is made from, that
//! root's computer settings (camera catalog, sources, language, diagnostics) and one helper project
//! made by QNC's Project block (`qnc-project-store`), so QNC's Ingest runs in it with all it does
//! (Select, posters, play, IN/OUT). Nothing of a real project is touched. When the story is done
//! with it the helper project is deleted by the same Project block (which takes its protection
//! off) and the folder goes; a work place left by a program that stopped is cleaned at the next
//! start.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use qnc_application_catalog::{SelectedApplication, SelectionSnapshot};
use qnc_project_store::ProjectStore;

/// The name of the helper project (it is never a story).
pub const HELPER_NAME: &str = "QNC AI card preview";
/// The prefix of a work place folder.
const PREFIX: &str = "work-";
/// The computer settings a work place takes from its QNC root.
const SETTINGS: [&str; 4] = ["ingest-transport.json", "ui-language.json", "dev-diagnostics.json", "voice-input.json"];

/// A work place: its root and its helper project.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Workspace {
    pub root: PathBuf,
    pub project_id: String,
}

/// The folder the work places of this computer are in.
pub fn base() -> PathBuf {
    std::env::temp_dir().join("QNC-AI")
}

impl Workspace {
    /// Makes a work place in `base` from the QNC root `qnc_root`.
    pub fn make(qnc_root: &Path, base: &Path) -> Result<Self, String> {
        let stamp = SystemTime::now().duration_since(UNIX_EPOCH).map(|time| time.as_millis()).unwrap_or(0);
        let root = base.join(format!("{PREFIX}{stamp}-{}", std::process::id()));
        std::fs::create_dir_all(root.join("data")).map_err(|error| format!("{}: {error}", root.display()))?;
        let made = (|| {
            copy_tree(&qnc_root.join("contracts"), &root.join("contracts"))?;
            copy_tree(&qnc_root.join("catalogs"), &root.join("catalogs"))?;
            std::fs::write(root.join("AGENTS.md"), "Temporary work place of the QNC AI wizard: it is deleted when the story no longer needs it.\n")
                .map_err(|error| error.to_string())?;
            std::fs::write(root.join("Cargo.toml"), "# temporary QNC AI work place\n").map_err(|error| error.to_string())?;
            for name in SETTINGS {
                let from = qnc_root.join("data").join(name);
                if from.is_file() {
                    std::fs::copy(&from, root.join("data").join(name)).map_err(|error| format!("{}: {error}", from.display()))?;
                }
            }
            let store = ProjectStore::open(&root)?;
            let project = store.create_project(HELPER_NAME, "", None, &selection())?;
            Ok(project.project_id)
        })();
        match made {
            Ok(project_id) => Ok(Self { root, project_id }),
            Err(error) => {
                let _ = std::fs::remove_dir_all(&root);
                Err(error)
            }
        }
    }

    /// Deletes the work place: its helper project through the Project block, then its folder.
    pub fn remove(&self) -> Result<(), String> {
        remove_root(&self.root)
    }
}

/// Removes every work place left in `base` (by a program that stopped); the ones in use now
/// (a program still has their files open) stay and are named in the answer.
pub fn clean(base: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(base) else { return Vec::new() };
    entries
        .flatten()
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(PREFIX))
        .filter_map(|entry| remove_root(&entry.path()).err().map(|error| format!("{}: {error}", entry.path().display())))
        .collect()
}

fn remove_root(root: &Path) -> Result<(), String> {
    if root.join("data").join("qnc-projects.db").is_file() {
        let store = ProjectStore::open(root)?;
        for project in store.list_projects()? {
            store.delete_project(&project.project_id)?;
        }
    }
    std::fs::remove_dir_all(root).map_err(|error| format!("{}: {error}", root.display()))
}

/// The applications of the helper project: Project only (the work place is for Ingest's look).
fn selection() -> SelectionSnapshot {
    SelectionSnapshot {
        schema_version: 1,
        catalog_uri: qnc_application_catalog::DEFAULT_URI.into(),
        observed_at_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH).map(|time| time.as_millis() as u64).unwrap_or(0),
        applications: vec![SelectedApplication { application_id: "qnc.project".into(), tab_id: "project".into(), label: "project".into(), priority_group: "a".into() }],
    }
}

fn copy_tree(from: &Path, to: &Path) -> Result<(), String> {
    std::fs::create_dir_all(to).map_err(|error| format!("{}: {error}", to.display()))?;
    for entry in std::fs::read_dir(from).map_err(|error| format!("{}: {error}", from.display()))?.flatten() {
        let (source, target) = (entry.path(), to.join(entry.file_name()));
        if source.is_dir() {
            copy_tree(&source, &target)?;
        } else {
            std::fs::copy(&source, &target).map_err(|error| format!("{}: {error}", source.display()))?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A work place made from the QNC root beside this repo (skipped where there is none) is a
    /// QNC root with an active helper project, and it is removed with its project.
    #[test]
    fn a_work_place_is_made_with_its_helper_project_and_removed_with_it() {
        let qnc = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../QNC");
        if !qnc.join("contracts").is_dir() {
            eprintln!("no QNC root beside QNC-AI: skipped");
            return;
        }
        let base = std::env::temp_dir().join(format!("qnc-card-workspace-test-{}", std::process::id()));
        let work = Workspace::make(&qnc, &base).unwrap();
        assert!(work.root.join("contracts").is_dir() && work.root.join("data/qnc-projects.db").is_file());
        let store = ProjectStore::open(&work.root).unwrap();
        let projects = store.list_projects().unwrap();
        assert_eq!(projects.len(), 1);
        assert!(projects[0].active, "the helper project is the active one");
        work.remove().unwrap();
        assert!(!work.root.exists(), "the work place is gone, its protected project folder too");
        assert!(clean(&base).is_empty());
        let _ = std::fs::remove_dir_all(&base);
    }
}
