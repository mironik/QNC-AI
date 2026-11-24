//! The card readers on the network (user 2026-11-22: "putanja importa iz, ili kroz čitač kartica
//! spojen na udaljenoj lokaciji"; "mrežna dijeljena mapa"): a card reader on another computer (a
//! Jetson) shares its card as a folder, here its path (`\\192.168.100.28\kartica`) and a name. A
//! setting of this computer, never of a project: `card-readers.qncreaders` (JSON) beside its other settings.
//! Each reader is a LAN source of Ingest, `qnc://lan/<host>/source/reader-<name>`, read as any card
//! (the folder is only read).

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The setting of this computer, beside its other settings.
pub const FILE: &str = "card-readers.qncreaders";

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

/// The cards in the card readers on the network of this computer (user 2026-11-22: "i dalje ne
/// zna koja je kamera, koji je serijski broj kartice"): a card reader mounts each card in a folder
/// named by the serial of its volume (`A815-5186`, the exFAT UUID; the same number this computer
/// reads when the card is in its own reader), so a card is the same card wherever it is put in.
/// Each is a volume of this computer read over the network. A reader not reachable now has none.
///
/// It never waits for the network (2026-11-24 log: a Jetson that did not answer kept the tray's
/// menu waiting 64 s, every read of the share waiting for the network's timeout): each reader is
/// read by a thread of its own, again every [`READ_EVERY`], and this gives what each last found.
/// A reader asked for the first time is given [`FIRST_WAIT`] to answer.
pub fn cards(root: &Path) -> Vec<qnc_source_volumes::Volume> {
    let Ok(readers) = readers_of(root) else { return Vec::new() };
    readers.iter().flat_map(|reader| background::found(reader)).collect()
}

/// How often a reader on the network is read again.
pub const READ_EVERY: std::time::Duration = std::time::Duration::from_secs(2);
/// How long a reader asked for the first time is waited for.
pub const FIRST_WAIT: std::time::Duration = std::time::Duration::from_millis(300);

/// The cards a reader's folder holds now (the network is waited for).
fn read_reader(reader: &CardReader) -> Vec<qnc_source_volumes::Volume> {
    let Ok(entries) = std::fs::read_dir(&reader.path) else { return Vec::new() };
    let mut cards = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().to_string();
        let Some(serial) = volume_serial(&name) else { continue };
        if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            cards.push(qnc_source_volumes::Volume { root: entry.path(), serial_number: serial, volume_name: Some(reader.name.clone()) });
        }
    }
    cards
}

mod background {
    use std::collections::BTreeMap;
    use std::sync::{Arc, Mutex, OnceLock};
    use std::time::Instant;

    use super::{read_reader, CardReader, FIRST_WAIT, READ_EVERY};

    /// What a reader last found: `None` until its first read is done.
    type Found = Arc<Mutex<Option<Vec<qnc_source_volumes::Volume>>>>;

    /// The readers being read in this process, by name and path.
    fn readers() -> &'static Mutex<BTreeMap<(String, std::path::PathBuf), Found>> {
        static READERS: OnceLock<Mutex<BTreeMap<(String, std::path::PathBuf), Found>>> = OnceLock::new();
        READERS.get_or_init(Default::default)
    }

    /// What `reader` last found; its thread is started the first time it is asked for.
    pub(super) fn found(reader: &CardReader) -> Vec<qnc_source_volumes::Volume> {
        let key = (reader.name.clone(), reader.path.clone());
        let (found, new) = {
            let mut readers = readers().lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            match readers.get(&key) {
                Some(found) => (found.clone(), false),
                None => {
                    let found: Found = Arc::default();
                    readers.insert(key, found.clone());
                    (found, true)
                }
            }
        };
        if new {
            let (shared, reader) = (found.clone(), reader.clone());
            let _ = std::thread::Builder::new().name("qnc-card-reader".into()).spawn(move || loop {
                let cards = read_reader(&reader);
                *shared.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(cards);
                std::thread::sleep(READ_EVERY);
            });
            let since = Instant::now();
            while since.elapsed() < FIRST_WAIT && found.lock().map(|found| found.is_none()).unwrap_or(false) {
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        let latest = found.lock().unwrap_or_else(|poisoned| poisoned.into_inner()).clone();
        latest.unwrap_or_default()
    }
}

/// The disks attached to this computer and the cards in its card readers on the network.
pub fn mounted_and_cards(root: &Path) -> Vec<qnc_source_volumes::Volume> {
    let mut all = qnc_source_volumes::mounted();
    for card in cards(root) {
        if !all.iter().any(|volume| volume.serial_number == card.serial_number) {
            all.push(card);
        }
    }
    all
}

/// The volume serial a folder named `XXXX-XXXX` (hex) stands for, as this computer writes it
/// (lower case, no dash).
fn volume_serial(name: &str) -> Option<String> {
    let (high, low) = name.split_once('-')?;
    let hex = |part: &str| part.len() == 4 && part.chars().all(|c| c.is_ascii_hexdigit());
    (hex(high) && hex(low)).then(|| format!("{high}{low}").to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_folder_named_by_a_volume_serial_is_that_card() {
        assert_eq!(super::volume_serial("A815-5186").as_deref(), Some("a8155186"));
        assert_eq!(super::volume_serial("kartica"), None);
        assert_eq!(super::volume_serial("A815-518"), None);
    }

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

    #[test]
    fn the_cards_of_a_reader_are_read_in_the_background_and_never_wait_for_the_network() {
        let root = tempfile::tempdir().unwrap();
        let share = tempfile::tempdir().unwrap();
        std::fs::create_dir(share.path().join("A815-5186")).unwrap();
        std::fs::create_dir(share.path().join("not a card")).unwrap();
        // An address that answers nothing (TEST-NET-1) and a folder that is there.
        let readers = vec![reader("Away", r"\\192.0.2.1\kartice"), reader("Here", &share.path().to_string_lossy())];
        set_readers_of(root.path(), &readers).unwrap();
        let since = std::time::Instant::now();
        let cards = cards(root.path());
        assert!(since.elapsed() < FIRST_WAIT * 3, "the network is not waited for: {:?}", since.elapsed());
        assert_eq!(cards.iter().map(|card| card.serial_number.as_str()).collect::<Vec<_>>(), vec!["a8155186"]);
        let again = std::time::Instant::now();
        assert_eq!(super::cards(root.path()).len(), 1);
        assert!(again.elapsed() < std::time::Duration::from_millis(50), "asked again, nothing is waited for");
    }
}
