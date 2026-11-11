//! A disk attached since the last look and what it is (docs/01-wizard-plan.md, step 1-2). The
//! attached disks come from QNC's `qnc-source-volumes`; a new one is looked at with QNC's camera
//! catalog (`qnc-camera-detector`, card roots from the disk's root): a card of a camera QNC
//! knows, or not. A disk attached before the first look starts nothing. It reads only: it never
//! writes to a disk, never probes, never opens a project. What it remembers is the disks of its
//! last look, in the memory of its process.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use qnc_camera_detector::{Limits, SourceScope};
use qnc_camera_patterns::Catalog;
use qnc_source_volumes::Volume;

/// What a newly attached disk is.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Found {
    /// A card of a camera QNC knows: the disk, the cameras whose card it is (manufacturer and
    /// family from the catalog) and their pattern ids.
    Card { volume: Volume, cameras: Vec<String>, patterns: Vec<String> },
    /// A disk with no card of a known camera on it.
    Unknown { volume: Volume },
    /// A disk that could not be looked at (a card still waking up, a read error).
    Unreadable { volume: Volume, error: String },
}

/// The disks of the last look.
#[derive(Debug, Default)]
pub struct CardWatch {
    seen: Option<BTreeSet<String>>,
}

impl CardWatch {
    pub fn new() -> Self {
        Self::default()
    }

    /// The disks of `attached` that were not there at the last look (none at the first look).
    pub fn new_disks(&mut self, attached: Vec<Volume>) -> Vec<Volume> {
        let now: BTreeSet<String> = attached.iter().map(key).collect();
        let fresh = match &self.seen {
            None => Vec::new(),
            Some(seen) => attached.into_iter().filter(|volume| !seen.contains(&key(volume))).collect(),
        };
        self.seen = Some(now);
        fresh
    }

    /// One look at the disks attached now.
    pub fn look(&mut self, catalog: &Catalog) -> Vec<Found> {
        self.new_disks(qnc_source_volumes::mounted()).into_iter().map(|volume| what_is(catalog, volume)).collect()
    }

    /// Forgets a disk, so it is looked at again on the next look (a card that could not be read yet).
    pub fn forget(&mut self, volume: &Volume) {
        if let Some(seen) = &mut self.seen {
            seen.remove(&key(volume));
        }
    }
}

/// A disk is the same while its letter and its volume serial are.
fn key(volume: &Volume) -> String {
    format!("{}|{}", volume.root.display(), volume.serial_number)
}

/// The camera catalog this computer's QNC configuration names (`data/ingest-transport.json`).
pub fn catalog(root: &Path) -> Result<Catalog, String> {
    let bindings = qnc_source_bindings::load(root)?;
    let binding = bindings.catalog.ok_or("the QNC configuration of this computer names no camera catalog")?;
    qnc_camera_patterns::read_uri(&binding.resolver()?, &binding.uri, binding.token()?.as_deref())
}

/// What the disk `volume` is, by the camera catalog.
pub fn what_is(catalog: &Catalog, volume: Volume) -> Found {
    match card_patterns(catalog, &volume.root, &volume.source_uri()) {
        Ok(patterns) if patterns.is_empty() => Found::Unknown { volume },
        Ok(patterns) => {
            let cameras = patterns
                .iter()
                .filter_map(|id| catalog.patterns.iter().find(|pattern| &pattern.id == id))
                .map(|pattern| format!("{} {}", pattern.manufacturer, pattern.family))
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            Found::Card { volume, cameras, patterns }
        }
        Err(error) => Found::Unreadable { volume, error },
    }
}

/// The patterns of the catalog whose card is at `root` with original clips on it.
pub fn card_patterns(catalog: &Catalog, root: &Path, source_uri: &str) -> Result<Vec<String>, String> {
    let source = qnc_source_reader::SourceReader::local(source_uri, root).map_err(|error| format!("{error:?}"))?;
    let report = qnc_camera_detector::detect(catalog, &source, SourceScope::CardRelative, Limits::default())?;
    Ok(report.roots.iter().filter(|root| root.has_original_candidates()).map(|root| root.pattern_id.clone()).collect::<BTreeSet<_>>().into_iter().collect())
}

/// The folder of a disk (for the wizard's `--card`).
pub fn card_folder(volume: &Volume) -> PathBuf {
    volume.root.clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn volume(letter: &str, serial: &str) -> Volume {
        Volume { root: PathBuf::from(format!("{letter}:\\")), serial_number: serial.into(), volume_name: None }
    }

    /// A folder laid out as a Sony card is a card by the catalog of the QNC root beside this repo
    /// (skipped where there is none); an empty folder is not.
    #[test]
    fn a_sony_card_layout_is_a_card_and_an_empty_folder_is_not() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../QNC");
        let Ok(catalog) = catalog(&root) else {
            eprintln!("no QNC root beside QNC-AI: skipped");
            return;
        };
        let card = tempfile_dir("card");
        std::fs::create_dir_all(card.join("XDROOT/Clip")).unwrap();
        std::fs::write(card.join("XDROOT/MEDIAPRO.XML"), "<x/>").unwrap();
        std::fs::write(card.join("XDROOT/Clip/A001C001.MXF"), b"").unwrap();
        let found = card_patterns(&catalog, &card, "qnc://local/source/volume-12345678").unwrap();
        assert!(found.iter().any(|id| id.starts_with("sony")), "{found:?}");
        let empty = tempfile_dir("empty");
        assert!(card_patterns(&catalog, &empty, "qnc://local/source/volume-87654321").unwrap().is_empty());
    }

    fn tempfile_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("qnc-card-watch-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn only_a_disk_attached_after_the_first_look_is_new() {
        let mut watch = CardWatch::new();
        assert!(watch.new_disks(vec![volume("C", "11111111"), volume("D", "22222222")]).is_empty(), "the first look starts nothing");
        assert!(watch.new_disks(vec![volume("C", "11111111"), volume("D", "22222222")]).is_empty());
        assert_eq!(watch.new_disks(vec![volume("C", "11111111"), volume("D", "22222222"), volume("G", "33333333")]), vec![volume("G", "33333333")]);
        assert!(watch.new_disks(vec![volume("C", "11111111")]).is_empty(), "a disk taken out is no new disk");
        assert_eq!(watch.new_disks(vec![volume("C", "11111111"), volume("G", "44444444")]), vec![volume("G", "44444444")], "another card on the same letter is new");
        watch.forget(&volume("G", "44444444"));
        assert_eq!(watch.new_disks(vec![volume("C", "11111111"), volume("G", "44444444")]), vec![volume("G", "44444444")], "a forgotten disk is looked at again");
    }
}
