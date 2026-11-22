//! The card readers on the network (user 2026-11-22: "putanja importa iz, ili kroz čitač kartica
//! spojen na udaljenoj lokaciji"; "mrežna dijeljena mapa"): a card reader on another computer (a
//! Jetson) shares its card as a folder, here its path (`\\192.168.100.28\kartica`) and a name. A
//! setting of this computer, never of a project: `card-readers.json` beside its other settings.
//! Each reader is a LAN source of Ingest, `qnc://lan/<host>/source/reader-<name>`, read as any card
//! (the folder is only read).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The setting of this computer, beside its other settings.
pub const FILE: &str = "card-readers.json";

/// One card reader on the network: its name and the path of the folder it shares.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CardReader {
    pub name: String,
    pub path: PathBuf,
}

#[derive(Debug, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct File {
    readers: Vec<CardReader>,
}

fn file_path(root: &Path) -> PathBuf {
    qnc_dev_diagnostics::config_path(root).with_file_name(FILE)
}

impl CardReader {
    /// The name in the source URI: its letters and digits, lower case, the rest `-`.
    pub fn id(&self) -> String {
        let id: String = self.name.trim().chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
        id.trim_matches('-').to_string()
    }

    /// The host the folder is shared from (`\\host\share`), else `share`; as a source URI names
    /// it: letters, digits, `-` and `_` (a dot of an address is `-`: `192-168-100-28`).
    pub fn host(&self) -> String {
        let text = self.path.to_string_lossy().replace('/', "\\");
        let host = text.strip_prefix("\\\\").and_then(|rest| rest.split('\\').next()).unwrap_or_default();
        let host: String = host.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c.to_ascii_lowercase() } else { '-' }).collect();
        match host.trim_matches('-') {
            "" => "share".into(),
            host => host.into(),
        }
    }

    /// The LAN source of Ingest this reader is.
    pub fn source_uri(&self) -> String {
        format!("qnc://lan/{}/source/reader-{}", self.host(), self.id())
    }

    fn check(&self) -> Result<(), String> {
        if self.id().is_empty() {
            return Err(format!("card reader '{}': a name of letters or digits", self.name));
        }
        if !self.path.is_absolute() {
            return Err(format!("card reader '{}': the path of its shared folder (\\\\host\\folder)", self.name));
        }
        Ok(())
    }
}

/// The card readers of this computer; none when the setting is not there. A setting that cannot
/// be read is an error.
pub fn readers_of(root: &Path) -> Result<Vec<CardReader>, String> {
    let path = file_path(root);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let file: File = serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    for reader in &file.readers {
        reader.check()?;
    }
    Ok(file.readers)
}

/// Writes the card readers of this computer; none removes the setting. A reader without a name,
/// with a path that is not absolute or with the name of another is refused, nothing is written.
pub fn set_readers_of(root: &Path, readers: &[CardReader]) -> Result<(), String> {
    let path = file_path(root);
    if readers.is_empty() {
        return match std::fs::remove_file(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {error}", path.display())),
            _ => Ok(()),
        };
    }
    let mut ids = std::collections::BTreeSet::new();
    for reader in readers {
        reader.check()?;
        if !ids.insert(reader.source_uri()) {
            return Err(format!("card reader '{}': another has its name", reader.name));
        }
    }
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    }
    let text = serde_json::to_string_pretty(&File { readers: readers.to_vec() }).map_err(|error| error.to_string())?;
    std::fs::write(&path, text).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reader(name: &str, path: &str) -> CardReader {
        CardReader { name: name.into(), path: path.into() }
    }

    #[test]
    fn a_reader_is_a_lan_source_named_by_its_host_and_name() {
        let jetson = reader("Jetson kartica", r"\\192.168.100.28\kartica");
        assert_eq!(jetson.source_uri(), "qnc://lan/192-168-100-28/source/reader-jetson-kartica");
    }

    #[test]
    fn the_readers_written_are_read_back_and_a_wrong_one_writes_nothing() {
        let root = tempfile::tempdir().unwrap();
        assert!(readers_of(root.path()).unwrap().is_empty());
        let readers = vec![reader("Jetson", r"\\192.168.100.28\kartica")];
        set_readers_of(root.path(), &readers).unwrap();
        assert_eq!(readers_of(root.path()).unwrap(), readers);
        assert!(set_readers_of(root.path(), &[reader("", r"\\h\k")]).is_err());
        assert!(set_readers_of(root.path(), &[reader("A", r"\\h\k"), reader("a", r"\\h\x")]).is_err());
        assert_eq!(readers_of(root.path()).unwrap(), readers);
        set_readers_of(root.path(), &[]).unwrap();
        assert!(readers_of(root.path()).unwrap().is_empty());
    }
}
