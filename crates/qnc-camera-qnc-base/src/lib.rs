//! A QNC card base as a card (user 2026-11-15: "tray-ingest stvara kvazi karticu koju mi nazivamo
//! baza; ta kvazi kartica je u biti standardizirana kvazi QNC-kartica"; "možeš kreirati u katalogu
//! kamera kocku za import baze"). The tray's ingest copies a card into a base; beside its database
//! the base carries an index, `QNCCARD.JSON`, as a Sony card carries `MEDIAPRO.XML`: the clips of
//! the base, each with its original, its proxy, its poster and its finished record (a clip
//! metadata file under `QNCCARD/`). This adapter reads that index for Select, which then takes the
//! base as any card: the clips are written into the project's own database and their media stays in
//! the base (Link). A base is never written by a project.
//!
//! The record the tray finished (the camera's metadata and the one probe of the import) is declared:
//! a clip whose record has the facts of a probe is not probed again. Stateless, no I/O: the caller
//! reads the documents and passes their text in.

use qnc_camera_adapter::{has_probe_facts, CameraAdapter, MetadataSufficiency};
use qnc_media_metadata::ClipMetadata;
use qnc_source_contract::SourceReference;
use qnc_source_groups::{GroupEvidence, GroupProposal, IndexDocument, IndexReader, RelatedReference};
use serde::{Deserialize, Serialize};

pub const ADAPTER_ID: &str = "camera.qnc.base-card";
pub const PATTERN_ID: &str = "qnc-base-card";
pub const READER_ID: &str = "camera.qnc.base-card.read";
/// The namespace of the index in the camera catalog.
pub const NAMESPACE: &str = "urn:qnc:base-card:1";
/// The index file at the root of a base.
pub const INDEX_FILE: &str = "QNCCARD.JSON";
/// The folder of the clips' records beside it.
pub const RECORD_FOLDER: &str = "QNCCARD";
/// The format the index says it is.
pub const FORMAT: &str = "qnc-base-card";
pub const VERSION: u32 = 1;

const RECORD: &str = "QNC_RECORD";
const POSTER: &str = "QNC_POSTER";

/// `QNCCARD.JSON`: the clips of a base.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseCardIndex {
    pub format: String,
    pub version: u32,
    pub clips: Vec<BaseCardClip>,
}

/// One clip of a base: paths relative to the base's folder.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BaseCardClip {
    /// The clip's id in the base: its recording identity.
    pub id: String,
    pub name: String,
    pub original: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub proxy: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub poster: Option<String>,
    /// The clip's finished record (`ClipMetadata` as JSON).
    pub record: String,
}

impl BaseCardIndex {
    pub fn new(clips: Vec<BaseCardClip>) -> Self {
        Self { format: FORMAT.into(), version: VERSION, clips }
    }

    pub fn parse(text: &str) -> Result<Self, String> {
        let index: Self = serde_json::from_str(text).map_err(|error| format!("{INDEX_FILE}: {error}"))?;
        if index.format != FORMAT || index.version != VERSION {
            return Err(format!("{INDEX_FILE}: format {} {} is not {FORMAT} {VERSION}", index.format, index.version));
        }
        Ok(index)
    }

    pub fn text(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }
}

/// The relative path of a clip's record in a base.
pub fn record_path(clip_id: &str) -> String {
    format!("{RECORD_FOLDER}/{clip_id}.json")
}

#[derive(Debug, Default, Clone, Copy)]
pub struct QncBaseCard;

impl QncBaseCard {
    pub fn new() -> Self {
        Self
    }
}

impl IndexReader for QncBaseCard {
    fn reader_id(&self) -> &str {
        READER_ID
    }

    fn namespace(&self) -> &str {
        NAMESPACE
    }

    fn read(&self, root: &SourceReference, document: &IndexDocument) -> Result<Vec<GroupProposal>, String> {
        if !document.reference.is_within(root) || &document.reference == root {
            return Err("index must be inside recording root".into());
        }
        let index = BaseCardIndex::parse(&document.text)?;
        let at = |path: &str| root.descendant(path).map_err(|error| error.to_string());
        index
            .clips
            .iter()
            .map(|clip| {
                let mut related = vec![RelatedReference { reference: at(&clip.record)?, kind: RECORD.into() }];
                if let Some(poster) = &clip.poster {
                    related.push(RelatedReference { reference: at(poster)?, kind: POSTER.into() });
                }
                let proposal = GroupProposal {
                    root: root.clone(),
                    recording_identity: clip.id.clone(),
                    evidence: GroupEvidence { reader_id: READER_ID.into(), document: document.reference.clone(), locator: format!("clips/{}", clip.id) },
                    original: at(&clip.original)?,
                    proxies: clip.proxy.as_deref().map(at).transpose()?.into_iter().collect(),
                    related,
                    stamp: None,
                };
                proposal.validate(root.source_uri())?;
                Ok(proposal)
            })
            .collect()
    }
}

fn related(group: &GroupProposal, kind: &str) -> Option<SourceReference> {
    group.related.iter().find(|related| related.kind == kind).map(|related| related.reference.clone())
}

impl CameraAdapter for QncBaseCard {
    fn adapter_id(&self) -> &str {
        ADAPTER_ID
    }

    fn pattern_ids(&self) -> Vec<&str> {
        vec![PATTERN_ID]
    }

    fn index(&self) -> &dyn IndexReader {
        self
    }

    fn documents(&self, group: &GroupProposal) -> Vec<SourceReference> {
        related(group, RECORD).into_iter().collect()
    }

    fn thumbnail(&self, group: &GroupProposal) -> Option<SourceReference> {
        related(group, POSTER)
    }

    /// The record the base keeps for the clip, at the media of this source (the base as it is
    /// reached here: a disk, a share).
    fn metadata(&self, clip_id: &str, group: &GroupProposal, documents: &[IndexDocument]) -> Result<ClipMetadata, String> {
        if group.evidence.reader_id != READER_ID {
            return Err("not a group of a QNC card base".into());
        }
        let reference = related(group, RECORD).ok_or("the clip has no record in the base")?;
        let document = documents.iter().find(|document| document.reference == reference).ok_or("the clip's record in the base was not read")?;
        let mut metadata: ClipMetadata = serde_json::from_str(&document.text).map_err(|error| format!("{}: {error}", reference.relative_path()))?;
        metadata.clip_id = clip_id.into();
        metadata.original.media_uri = group.original.uri();
        match (metadata.proxy.as_mut(), group.proxies.first()) {
            (Some(proxy), Some(file)) => proxy.media_uri = file.uri(),
            (Some(_), None) => metadata.proxy = None,
            (None, _) => {}
        }
        Ok(metadata)
    }

    fn sufficiency(&self, metadata: &ClipMetadata) -> MetadataSufficiency {
        if has_probe_facts(metadata) {
            MetadataSufficiency::Declared
        } else {
            MetadataSufficiency::NeedsProbe
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use qnc_media_metadata::MediaRepresentation;

    const SOURCE: &str = "qnc://local/source/volume-1234abcd";

    fn record() -> ClipMetadata {
        let media = |uri: &str| MediaRepresentation { media_uri: uri.into(), container: None, duration_seconds: None, streams_complete: None, streams: vec![], tags: Default::default() };
        ClipMetadata {
            contract_id: qnc_media_metadata::CONTRACT_ID.into(),
            contract_version: qnc_media_metadata::CONTRACT_VERSION.into(),
            clip_id: "clip-in-base".into(),
            evidence: vec![],
            original: media("qnc://local/source/elsewhere/file/a.mxf"),
            proxy: None,
        }
    }

    #[test]
    fn a_base_card_gives_its_clips_with_the_record_it_keeps_at_this_source() {
        let root = SourceReference::new(SOURCE, "bases/b1").unwrap();
        let index = BaseCardIndex::new(vec![BaseCardClip {
            id: "clip-1".into(),
            name: "Mironik 3744".into(),
            original: "original/clip-1_Mironik_3744.MXF".into(),
            proxy: None,
            poster: Some("products/thumbnails/clip-1/poster.jpg".into()),
            record: record_path("clip-1"),
        }]);
        let document = IndexDocument { reference: root.descendant(INDEX_FILE).unwrap(), text: index.text() };
        let card = QncBaseCard::new();
        let groups = card.read(&root, &document).unwrap();
        assert_eq!(groups.len(), 1);
        let group = &groups[0];
        assert_eq!(group.recording_identity, "clip-1");
        assert_eq!(group.original.relative_path(), "bases/b1/original/clip-1_Mironik_3744.MXF");
        assert_eq!(card.thumbnail(group).unwrap().relative_path(), "bases/b1/products/thumbnails/clip-1/poster.jpg");
        let documents = card.documents(group);
        let read = IndexDocument { reference: documents[0].clone(), text: serde_json::to_string(&record()).unwrap() };
        let metadata = card.metadata("project-clip", group, &[read]).unwrap();
        assert_eq!(metadata.clip_id, "project-clip");
        assert_eq!(metadata.original.media_uri, group.original.uri(), "the media is the base's, where this source reaches it");
        assert_eq!(card.sufficiency(&metadata), MetadataSufficiency::NeedsProbe, "a record without the facts of a probe is probed once");
        assert!(BaseCardIndex::parse(r#"{"format":"x","version":1,"clips":[]}"#).is_err(), "only a QNC card base index");
    }
}
