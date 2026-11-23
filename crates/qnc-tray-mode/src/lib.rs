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

    /// The key of this mode in the computer's setting of forms.
    pub fn key(self) -> &'static str {
        match self {
            Mode::Local { work: Work::BreakingNews } => "breaking_news",
            Mode::Local { work: Work::Story } => "story",
            Mode::Net => "net",
        }
    }

    /// The form this mode opens after the ingest Wizard and on the tray's Open application, as the
    /// computer's setting says (`data/tray-forms.json`, chosen in Settings; user 2026-11-23: "to
    /// možemo imati kao postavku u Postavkama, na taj način ništa nije hardkodirano"). Nothing
    /// set: none.
    pub fn opens_on(self, root: &Path) -> Result<Option<Opens>, String> {
        Ok(forms(root)?.of(self).cloned().filter(|opens| !opens.program.trim().is_empty()))
    }

    /// [`Mode::opens_on`] on this QNC's root, as a program's name and arguments.
    pub fn opens(self) -> Option<(&'static str, &'static [&'static str])> {
        let root = qnc_dev_diagnostics::resolve_qnc_root()?;
        self.opens_on(&root).ok().flatten().map(kept)
    }
}

/// A form a mode opens: its program (the application catalog's standalone executable) and its
/// arguments.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Opens {
    pub program: String,
    #[serde(default)]
    pub args: Vec<String>,
}

/// The forms of the modes on this computer; a mode without one opens none.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct Forms {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub breaking_news: Option<Opens>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub story: Option<Opens>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub net: Option<Opens>,
}

impl Forms {
    pub fn of(&self, mode: Mode) -> Option<&Opens> {
        match mode {
            Mode::Local { work: Work::BreakingNews } => self.breaking_news.as_ref(),
            Mode::Local { work: Work::Story } => self.story.as_ref(),
            Mode::Net => self.net.as_ref(),
        }
    }

    pub fn set(&mut self, mode: Mode, opens: Option<Opens>) {
        let slot = match mode {
            Mode::Local { work: Work::BreakingNews } => &mut self.breaking_news,
            Mode::Local { work: Work::Story } => &mut self.story,
            Mode::Net => &mut self.net,
        };
        *slot = opens.filter(|opens| !opens.program.trim().is_empty());
    }
}

fn forms_file(root: &Path) -> PathBuf {
    root.join("data").join("tray-forms.json")
}

/// The forms set on this computer; none set yet: no form. A file not read is an error.
pub fn forms(root: &Path) -> Result<Forms, String> {
    let file = forms_file(root);
    match std::fs::read_to_string(&file) {
        Ok(text) => serde_json::from_str(&text).map_err(|error| format!("{}: {error}", file.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Forms::default()),
        Err(error) => Err(format!("{}: {error}", file.display())),
    }
}

/// Keeps the forms as this computer's.
pub fn set_forms(root: &Path, forms: &Forms) -> Result<(), String> {
    let file = forms_file(root);
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    }
    let text = serde_json::to_string_pretty(forms).map_err(|error| error.to_string())?;
    std::fs::write(&file, text).map_err(|error| format!("{}: {error}", file.display()))
}

/// A form as the callers take it (a program's name and arguments that live on): each different form
/// is kept once for the process.
fn kept(opens: Opens) -> (&'static str, &'static [&'static str]) {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static KEPT: OnceLock<Mutex<HashMap<Opens, (&'static str, &'static [&'static str])>>> = OnceLock::new();
    let mut kept = KEPT.get_or_init(Default::default).lock().unwrap_or_else(|e| e.into_inner());
    *kept.entry(opens.clone()).or_insert_with(|| {
        let program: &'static str = Box::leak(opens.program.into_boxed_str());
        let args: Vec<&'static str> = opens.args.into_iter().map(|arg| &*Box::leak(arg.into_boxed_str())).collect();
        (program, Box::leak(args.into_boxed_slice()))
    })
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

    #[test]
    fn a_mode_opens_the_form_the_setting_names_and_none_without_it() {
        let root = std::env::temp_dir().join(format!("qnc-tray-forms-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let story = Mode::Local { work: Work::Story };
        assert_eq!(story.opens_on(&root).unwrap(), None, "nothing set: no form, none made up");
        let mut forms = Forms::default();
        forms.set(story, Some(Opens { program: "qnc-ingest".into(), args: Vec::new() }));
        set_forms(&root, &forms).unwrap();
        assert_eq!(story.opens_on(&root).unwrap().map(|opens| opens.program), Some("qnc-ingest".into()));
        assert_eq!(Mode::Net.opens_on(&root).unwrap(), None);
        assert_eq!(kept(Opens { program: "a".into(), args: vec!["b".into()] }), ("a", &["b"][..]));
        let _ = std::fs::remove_dir_all(&root);
    }
}
