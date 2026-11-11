//! What is on a camera card, read before the story has a project (user 2026-11-11: "baza je u
//! QNC-AI", "može biti i privremeni json dok se ne kreira projekt"). The card is read only: the
//! camera's own records (its index and the sidecars of each clip) through QNC's camera blocks,
//! never a probe, never a write on the card. What is found is kept as a temporary JSON in
//! QNC-AI's own data (`data/cards/<serial>.json`); it is not a project and nothing of QNC's root
//! is used.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use qnc_camera_detector::SourceScope;
use qnc_source_groups::IndexDocument;
use qnc_source_reader::{SourceReader, MAX_TEXT_BYTES};
use qnc_source_volumes::Volume;
use serde::{Deserialize, Serialize};

/// The folder of the temporary card files in a QNC-AI root.
pub const CARDS_DIR: &str = "data/cards";

/// One clip of a card, as the camera recorded it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardClip {
    /// The name of the original (its file name without the extension).
    pub name: String,
    /// The original on the card (QNC source reference URI).
    pub original: String,
    /// The proxies the camera made beside it.
    pub proxies: Vec<String>,
    /// The thumbnail the camera made, when it links one.
    pub thumbnail: Option<String>,
    /// The duration the camera's records give (seconds), when they give it.
    pub duration_seconds: Option<f64>,
}

/// What a card holds.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CardContent {
    pub serial: String,
    pub disk: String,
    pub source_uri: String,
    pub read_at_unix_ms: u64,
    pub clips: Vec<CardClip>,
    /// What could not be read (a clip without its records, a file the index names and the card
    /// has not); the clips that could are kept.
    pub warnings: Vec<String>,
}

/// Reads the card `volume` by the camera catalog and the camera profiles of the QNC-AI root
/// `root`. Nothing is written.
pub fn read(root: &Path, volume: &Volume) -> Result<CardContent, String> {
    let catalog = qnc_card_watch::catalog(root)?;
    let registry = qnc_ingest_cameras::registry(root)?;
    let source_uri = volume.source_uri();
    let source = SourceReader::local(&source_uri, &volume.root).map_err(|error| format!("{error:?}"))?;
    let indexes = registry.indexes();
    let scan = qnc_scanner::scan_roles(&catalog, &source, SourceScope::CardRelative, &indexes, Default::default())?;
    let mut warnings: Vec<String> = scan.issues.iter().map(|issue| format!("{}: {}", issue.reference.uri(), issue.message)).collect();
    warnings.extend(scan.grouping.blocked.iter().map(|blocked| format!("{blocked:?}")));
    let mut clips = Vec::new();
    for group in &scan.grouping.groups {
        let proposal = &group.proposal;
        let name = name_of(proposal.original.relative_path());
        let Some(adapter) = registry.for_reader(&proposal.evidence.reader_id) else {
            warnings.push(format!("{name}: no reader of its records"));
            continue;
        };
        let mut documents = Vec::new();
        for reference in adapter.documents(proposal) {
            if let Ok(text) = source.read_text(&reference, MAX_TEXT_BYTES) {
                documents.push(IndexDocument { reference, text: text.text });
            }
        }
        let duration_seconds = match adapter.metadata(&format!("card-{name}"), proposal, &documents) {
            Ok(metadata) => metadata.original.duration_seconds.map(|fact| fact.value).filter(|r| r.denominator != 0).map(|r| r.numerator as f64 / r.denominator as f64),
            Err(error) => {
                warnings.push(format!("{name}: {error}"));
                None
            }
        };
        clips.push(CardClip {
            name,
            original: proposal.original.uri(),
            proxies: proposal.proxies.iter().map(|proxy| proxy.uri()).collect(),
            thumbnail: adapter.thumbnail(proposal).map(|thumbnail| thumbnail.uri()),
            duration_seconds,
        });
    }
    clips.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(CardContent {
        serial: volume.serial_number.clone(),
        disk: volume.root.display().to_string(),
        source_uri,
        read_at_unix_ms: SystemTime::now().duration_since(UNIX_EPOCH).map(|time| time.as_millis() as u64).unwrap_or(0),
        clips,
        warnings,
    })
}

/// The temporary file of the card `serial` in the QNC-AI root `root`.
pub fn file(root: &Path, serial: &str) -> PathBuf {
    root.join(CARDS_DIR).join(format!("{serial}.json"))
}

/// Keeps what a card holds in its temporary file (a whole file each time).
pub fn keep(root: &Path, content: &CardContent) -> Result<PathBuf, String> {
    let path = file(root, &content.serial);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("{}: {error}", parent.display()))?;
    }
    let json = serde_json::to_vec_pretty(content).map_err(|error| error.to_string())?;
    std::fs::write(&path, json).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(path)
}

/// What the temporary file of the card `serial` says, if it has one.
pub fn kept(root: &Path, serial: &str) -> Result<Option<CardContent>, String> {
    let path = file(root, serial);
    match std::fs::read(&path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map(Some).map_err(|error| format!("{}: {error}", path.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{}: {error}", path.display())),
    }
}

/// The cards with a temporary file.
pub fn all_kept(root: &Path) -> Vec<CardContent> {
    let Ok(entries) = std::fs::read_dir(root.join(CARDS_DIR)) else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|entry| std::fs::read(entry.path()).ok())
        .filter_map(|bytes| serde_json::from_slice(&bytes).ok())
        .collect()
}

fn name_of(relative_path: &str) -> String {
    let file = relative_path.rsplit('/').next().unwrap_or(relative_path);
    file.rsplit_once('.').map_or(file, |(stem, _)| stem).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clip_is_named_by_its_file_without_the_extension() {
        assert_eq!(name_of("XDROOT/Clip/Mironik 3608.MXF"), "Mironik 3608");
        assert_eq!(name_of("C0012.MP4"), "C0012");
    }

    #[test]
    fn a_card_kept_is_read_back_and_a_card_never_kept_has_no_file() {
        let root = std::env::temp_dir().join(format!("qnc-card-read-{}", std::process::id()));
        let content = CardContent {
            serial: "a8155186".into(),
            disk: "D:\\".into(),
            source_uri: "qnc://local/source/volume-a8155186".into(),
            read_at_unix_ms: 1,
            clips: vec![CardClip { name: "A".into(), original: "o".into(), proxies: vec![], thumbnail: None, duration_seconds: Some(2.0) }],
            warnings: vec![],
        };
        keep(&root, &content).unwrap();
        assert_eq!(kept(&root, "a8155186").unwrap(), Some(content));
        assert_eq!(kept(&root, "00000000").unwrap(), None);
        assert_eq!(all_kept(&root).len(), 1);
        let _ = std::fs::remove_dir_all(&root);
    }
}
