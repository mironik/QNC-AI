//! How the tray works on this computer (user 2026-11-18): one tray for every computer, its work
//! chosen by a setting.
//! - **Net**: an ingest point on the network; it ingests and does all the AI work, and opens no
//!   application.
//! - **Local** (a laptop in the field), with what the journalist does next:
//!   - **Breaking news**: the application opens as soon as the copy is done; the waves first, the
//!     AI shots with the filmstrip, SigLIP, then the transcript and the captions in the background;
//!   - **Story**: the transcript, the waves and the filmstrip are waited for, then Ingest opens.
//!
//! The setting is the computer's (`data/tray-mode.json`), chosen in the tray's menu. Not chosen
//! yet, the tray only ingests (as it did before the setting): nothing is made up.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Where the tray works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "place", rename_all = "snake_case")]
pub enum Mode {
    /// A laptop in the field: after a card, the work the journalist does next.
    Local { work: Work },
    /// An ingest point on the network: ingest and AI only.
    Net,
}

/// What a journalist in the field does after a card.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Work {
    BreakingNews,
    Story,
}

/// Every mode, in the menu's order.
pub const ALL: [Mode; 3] = [Mode::Local { work: Work::BreakingNews }, Mode::Local { work: Work::Story }, Mode::Net];

impl Mode {
    /// The text code of its name in the menu.
    pub fn label(self) -> &'static str {
        match self {
            Mode::Local { work: Work::BreakingNews } => "tray.mode_local_breaking_news",
            Mode::Local { work: Work::Story } => "tray.mode_local_story",
            Mode::Net => "tray.mode_net",
        }
    }

    /// The Wizard's form this mode opens once a card is copied (its program's name and arguments):
    /// Breaking news opens AI Media Assist on All, there is no transcript yet (user 2026-11-18:
    /// "otvoriti u brejking news ide na AI Media Assist", "otvori ga na All kad je brejking news");
    /// the others open none (Story waits, then Ingest opens; AI Media Assist opens on its transcript).
    pub fn opens(self) -> Option<(&'static str, &'static [&'static str])> {
        match self {
            Mode::Local { work: Work::BreakingNews } => Some(("qnc-media-assist-audio-ai", &["--pool-tab", "editorial_tab_all"])),
            Mode::Local { work: Work::Story } | Mode::Net => None,
        }
    }
}

fn file(root: &Path) -> PathBuf {
    root.join("data").join("tray-mode.json")
}

/// The mode chosen on this computer; `None` when none is chosen yet. A file that cannot be read is
/// an error, never a mode made up.
pub fn read(root: &Path) -> Result<Option<Mode>, String> {
    let file = file(root);
    match std::fs::read_to_string(&file) {
        Ok(text) => serde_json::from_str(&text).map(Some).map_err(|error| format!("{}: {error}", file.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(format!("{}: {error}", file.display())),
    }
}

/// Keeps `mode` as this computer's.
pub fn write(root: &Path, mode: Mode) -> Result<(), String> {
    let file = file(root);
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    }
    let text = serde_json::to_string_pretty(&mode).map_err(|error| error.to_string())?;
    std::fs::write(&file, text).map_err(|error| format!("{}: {error}", file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mode_is_kept_and_read_back_and_none_is_chosen_at_first() {
        let root = std::env::temp_dir().join(format!("qnc-tray-mode-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(read(&root).unwrap(), None, "nothing made up");
        for mode in ALL {
            write(&root, mode).unwrap();
            assert_eq!(read(&root).unwrap(), Some(mode));
        }
        std::fs::write(root.join("data").join("tray-mode.json"), "{\"place\":\"moon\"}").unwrap();
        assert!(read(&root).is_err(), "a setting not read is an error");
        let _ = std::fs::remove_dir_all(&root);
    }
}
