//! Incoming (user 2026-11-24: "inkoming ima status inputa, kao neka vrsta virtualne kartice:
//! klipovi koji preko FTP ili Filemail usluge budu poslani na racunalo"; "inkoming moze biti i na
//! serveru koji nije na racunalu, a moze biti i u racunalu"; "daj opcije: automatski ingest ili
//! rucni ingest"). Where it is and how it is ingested is a setting of this computer
//! (`data/incoming.qncincoming`), never of a project: Incoming is a source, like a card, and
//! nothing is ever written into it. Not set: there is no Incoming, nothing is made up.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Where Incoming is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Place {
    /// A folder of this computer.
    Computer,
    /// A shared folder or QNC source on the local network.
    Lan,
    /// A shared folder or QNC source on the intranet.
    Intranet,
}

impl Place {
    pub const ALL: [Place; 3] = [Place::Computer, Place::Lan, Place::Intranet];

    pub fn code(self) -> &'static str {
        match self {
            Place::Computer => "computer",
            Place::Lan => "lan",
            Place::Intranet => "intranet",
        }
    }

    pub fn of(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|place| place.code() == code)
    }

    /// The text code of its name.
    pub fn label(self) -> &'static str {
        match self {
            Place::Computer => "incoming.place_computer",
            Place::Lan => "incoming.place_lan",
            Place::Intranet => "incoming.place_intranet",
        }
    }
}

/// How what arrives is ingested.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Ingest {
    /// The tray sees new files and ingests them as a card put in.
    Auto,
    /// Incoming is a source the user takes when they want.
    Manual,
}

impl Ingest {
    pub const ALL: [Ingest; 2] = [Ingest::Auto, Ingest::Manual];

    pub fn code(self) -> &'static str {
        match self {
            Ingest::Auto => "auto",
            Ingest::Manual => "manual",
        }
    }

    pub fn of(code: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|ingest| ingest.code() == code)
    }

    pub fn label(self) -> &'static str {
        match self {
            Ingest::Auto => "incoming.ingest_auto",
            Ingest::Manual => "incoming.ingest_manual",
        }
    }
}

/// The Incoming of this computer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Incoming {
    pub place: Place,
    /// A folder of this computer (absolute), or the folder or source on the network as its
    /// address (`\server\share\...` or `qnc://...`).
    pub location: String,
    pub ingest: Ingest,
}

impl Incoming {
    /// Why it cannot be kept, if it cannot.
    pub fn check(&self) -> Result<(), String> {
        let location = self.location.trim();
        if location.is_empty() {
            return Err("incoming: no location".into());
        }
        // A folder this computer reaches: its own, or the server's shared folder (`\\server\share`).
        if !Path::new(location).is_absolute() {
            return Err(format!("incoming: {location} is not a folder this computer reaches"));
        }
        Ok(())
    }
}

impl Incoming {
    /// Its folder: on this computer, or the shared folder of the server (`\\server\share\...`, or a
    /// drive the system mapped to it).
    pub fn folder(&self) -> PathBuf {
        PathBuf::from(self.location.trim())
    }

    /// The source Incoming is for Ingest, as a card: `qnc://local/source/incoming` on this computer,
    /// `qnc://lan/<server>/source/incoming` or `qnc://intranet/<server>/source/incoming` on a server
    /// (the server named by the shared folder's host). An address that is no folder is not one yet.
    pub fn source_uri(&self) -> Result<String, String> {
        let folder = self.folder();
        if !folder.is_absolute() {
            return Err(format!("incoming: {} is not a folder this computer reaches", self.location.trim()));
        }
        Ok(match self.place {
            Place::Computer => SOURCE_LOCAL.to_string(),
            Place::Lan => format!("qnc://lan/{}/source/incoming", host_of(&folder)),
            Place::Intranet => format!("qnc://intranet/{}/source/incoming", host_of(&folder)),
        })
    }
}

/// The source of an Incoming on this computer.
pub const SOURCE_LOCAL: &str = "qnc://local/source/incoming";

/// The server of a shared folder `\\server\share`, as a name of a source (a mapped drive: `share`).
fn host_of(folder: &Path) -> String {
    let text = folder.to_string_lossy().replace('/', "\\");
    let host = text.strip_prefix("\\\\").and_then(|rest| rest.split('\\').next()).unwrap_or_default();
    let host: String = host.chars().map(|c| if c.is_ascii_alphanumeric() || c == '_' { c.to_ascii_lowercase() } else { '-' }).collect();
    match host.trim_matches('-') {
        "" => "share".into(),
        host => host.into(),
    }
}

/// The setting's file of the QNC `root`.
pub fn file(root: &Path) -> PathBuf {
    qnc_user_settings::computer_file(root, "incoming.qncincoming")
}

/// The Incoming of this computer; none set: none. A file not read is an error.
pub fn read(root: &Path) -> Result<Option<Incoming>, String> {
    let file = file(root);
    match std::fs::read_to_string(&file) {
        Ok(text) => {
            let incoming: Incoming = serde_json::from_str(&text).map_err(|error| format!("{}: {error}", file.display()))?;
            incoming.check().map_err(|error| format!("{}: {error}", file.display()))?;
            Ok(Some(incoming))
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{}: {error}", file.display())),
    }
}

/// Keeps `incoming` as this computer's; `None` takes Incoming away.
pub fn write(root: &Path, incoming: Option<&Incoming>) -> Result<(), String> {
    let file = file(root);
    let Some(incoming) = incoming else {
        return match std::fs::remove_file(&file) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {error}", file.display())),
            _ => Ok(()),
        };
    };
    incoming.check()?;
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    }
    let text = serde_json::to_string_pretty(incoming).map_err(|error| error.to_string())?;
    std::fs::write(&file, text).map_err(|error| format!("{}: {error}", file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn incoming_is_none_until_set_and_a_computer_folder_must_be_one() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(read(root.path()).unwrap(), None, "not set: none, nothing made up");
        let lan = Incoming { place: Place::Lan, location: r"\\server\incoming".into(), ingest: Ingest::Auto };
        write(root.path(), Some(&lan)).unwrap();
        assert_eq!(read(root.path()).unwrap(), Some(lan));
        let relative = Incoming { place: Place::Computer, location: "incoming".into(), ingest: Ingest::Manual };
        assert!(write(root.path(), Some(&relative)).is_err());
        write(root.path(), None).unwrap();
        assert_eq!(read(root.path()).unwrap(), None);
    }

    #[test]
    fn incoming_is_a_source_of_its_place_named_by_its_server() {
        let here = Incoming { place: Place::Computer, location: r"D:\Incoming".into(), ingest: Ingest::Manual };
        assert_eq!(here.source_uri().unwrap(), "qnc://local/source/incoming");
        let lan = Incoming { place: Place::Lan, location: r"\\FTP-Server\incoming".into(), ingest: Ingest::Auto };
        assert_eq!(lan.source_uri().unwrap(), "qnc://lan/ftp-server/source/incoming");
        let intranet = Incoming { place: Place::Intranet, location: r"\\news.local\in".into(), ingest: Ingest::Auto };
        assert_eq!(intranet.source_uri().unwrap(), "qnc://intranet/news-local/source/incoming");
    }
}
