use super::*;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceKind {
    #[default]
    Local,
    Lan,
    Internet,
    /// QNC-AI: the Ingest bases of the application (data/ingest), not a source of media.
    Bases,
}

impl SourceKind {
    pub fn label(self) -> &'static str {
        match self {
            Self::Local => qnc_ui_text::text("source.computer"),
            Self::Lan => "LAN",
            Self::Internet => "Internet",
            Self::Bases => qnc_ui_text::text("source.bases"),
        }
    }

    pub fn action_id(self) -> &'static str {
        match self {
            Self::Local => action_ids::INGEST_SOURCE_KIND_LOCAL,
            Self::Lan => action_ids::INGEST_SOURCE_KIND_LAN,
            Self::Internet => action_ids::INGEST_SOURCE_KIND_INTERNET,
            Self::Bases => action_ids::INGEST_SOURCE_KIND_BASES,
        }
    }
}

pub(crate) fn source_kind_id(kind: SourceKind) -> &'static str {
    match kind {
        SourceKind::Local => "local",
        SourceKind::Lan => "lan",
        SourceKind::Internet => "internet",
        SourceKind::Bases => "bases",
    }
}
