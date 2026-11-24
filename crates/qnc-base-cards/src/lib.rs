//! The Ingest bases the tray made (QNC-AI), read only (user 2026-11-12: "u QNC ingestu treba
//! samo pored LAN, Intranet, Racunalo dodati izbor ulaza Baza koji vodi kad korisnik klikne na
//! ono sto je tray napravio"; "to se tretira kao kartica"; "klasicni QNC kupi podatke i sprema u
//! svoju bazu, kako inace radi"). A base is a folder with its own database in the folder of the
//! bases; this block says where that folder is (the setting `data/ingest-bases.json` of the
//! computer, never a folder written here) and, for each base, the cards its clips were Selected
//! from. Classic QNC opens such a card and Selects it into its own project as from any card.
//! Nothing is written but the setting.

use std::path::{Path, PathBuf};

/// The setting of this computer that says where the Ingest bases are made (Settings writes it).
pub const FOLDER_SETTING: &str = "data/ingest-bases.json";
/// The database of a base, as of every project.
const DATABASE: &str = "project.db";

#[derive(serde::Serialize, serde::Deserialize)]
struct FolderSetting {
    folder: PathBuf,
}

/// The folder the Ingest bases of `root` are in: the one Settings chose (user 2026-11-12:
/// "direktoriji ne smiju biti hardkodirani"). Not chosen is an error: no folder is guessed.
pub fn folder(root: &Path) -> Result<PathBuf, String> {
    let file = root.join(FOLDER_SETTING);
    let bytes = std::fs::read(&file).map_err(|_| format!("the folder of the Ingest bases is not set ({} missing; choose it in Settings)", file.display()))?;
    let setting: FolderSetting = serde_json::from_slice(&bytes).map_err(|error| format!("{}: {error}", file.display()))?;
    Ok(if setting.folder.is_absolute() { setting.folder } else { root.join(setting.folder) })
}

/// Settings: the Ingest bases are in `folder` from now on.
pub fn set_folder(root: &Path, folder: &Path) -> Result<(), String> {
    std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let file = root.join(FOLDER_SETTING);
    if let Some(parent) = file.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(&FolderSetting { folder: folder.to_path_buf() }).map_err(|error| error.to_string())?;
    std::fs::write(&file, json).map_err(|error| format!("{}: {error}", file.display()))
}

/// One card of a base: the source its clips were Selected from and how many.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BaseCard {
    /// The base: the name of its folder.
    pub base: String,
    /// The QNC URI of the source (the card or folder Select read).
    pub source_uri: String,
    pub clips: usize,
}

/// The cards of every base in the folder of the bases of `root`, newest base first. A base whose
/// database cannot be read is left out with its reason in `skipped`.
pub fn cards(root: &Path) -> Result<Listed, String> {
    cards_in(&folder(root)?)
}

/// What [`cards`] found.
#[derive(Debug, Default)]
pub struct Listed {
    pub cards: Vec<BaseCard>,
    pub skipped: Vec<String>,
}

pub fn cards_in(folder: &Path) -> Result<Listed, String> {
    let entries = std::fs::read_dir(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let mut bases: Vec<(std::time::SystemTime, String, PathBuf)> = entries
        .flatten()
        .filter(|entry| entry.path().join(DATABASE).is_file())
        .map(|entry| {
            let made = entry.metadata().and_then(|meta| meta.modified()).unwrap_or(std::time::UNIX_EPOCH);
            (made, entry.file_name().to_string_lossy().into_owned(), entry.path().join(DATABASE))
        })
        .collect();
    bases.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    let mut listed = Listed::default();
    for (_, base, database) in bases {
        match qnc_content_read::sources_of_file(&database) {
            Ok(sources) => listed.cards.extend(sources.into_iter().map(|(source_uri, clips)| BaseCard { base: base.clone(), source_uri, clips })),
            Err(error) => listed.skipped.push(format!("{base}: {error}")),
        }
    }
    Ok(listed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_folder_is_guessed_and_the_chosen_one_is_kept() {
        let root = tempfile::tempdir().unwrap();
        assert!(folder(root.path()).unwrap_err().contains("not set"));
        let bases = root.path().join("bases");
        set_folder(root.path(), &bases).unwrap();
        assert_eq!(folder(root.path()).unwrap(), bases);
    }

    #[test]
    fn a_base_gives_the_cards_its_clips_came_from() {
        let folder = tempfile::tempdir().unwrap();
        let base = folder.path().join("Ingest 1");
        std::fs::create_dir_all(&base).unwrap();
        let conn = rusqlite_connection(&base.join(DATABASE));
        conn.execute_batch(
            "CREATE TABLE clips (clip_id TEXT, source_uri TEXT, clip_kind TEXT);
             CREATE VIEW public_clips AS SELECT clip_id, source_uri, clip_kind FROM clips;
             INSERT INTO clips VALUES ('a', 'qnc://local/source/volume-1', 'camera'), ('b', 'qnc://local/source/volume-1', 'camera'),
                ('c', 'qnc://local/source/volume-2', 'camera'), ('v', 'qnc://local/source/project-x', 'voice');",
        )
        .unwrap();
        drop(conn);
        std::fs::create_dir_all(folder.path().join("not a base")).unwrap();
        let listed = cards_in(folder.path()).unwrap();
        assert_eq!(
            listed.cards,
            vec![
                BaseCard { base: "Ingest 1".into(), source_uri: "qnc://local/source/volume-1".into(), clips: 2 },
                BaseCard { base: "Ingest 1".into(), source_uri: "qnc://local/source/volume-2".into(), clips: 1 },
            ]
        );
        assert!(listed.skipped.is_empty());
    }

    fn rusqlite_connection(path: &Path) -> rusqlite::Connection {
        rusqlite::Connection::open(path).unwrap()
    }
}
