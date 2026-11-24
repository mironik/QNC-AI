//! What the tray's menu shows on this computer (user 2026-11-23: "sadržaj traya nije hardkodiran, i
//! njega ćemo kreirati u Postavkama; na taj način svaki korisnik može svojim potrebama podesiti
//! Tray"; "u postavkama trebaju biti checkboxovi za postavljanje onoga što će se vidjeti u trayu").
//!
//! The items are what the tray can do; the setting (`data/tray-menu.qnctray`) names the items hidden.
//! Nothing hidden (no setting yet): every item shows. Quit is always there, else the tray could not
//! be left.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use qnc_tray_mode::Mode;
use serde::{Deserialize, Serialize};

/// An item of the tray's menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Item {
    /// What runs (a line that only tells).
    Status,
    /// Stop all the work.
    StopAll,
    /// One of the tray's modes, ticked when it is this computer's.
    Mode(Mode),
    /// Import from a folder.
    IngestFolder,
    /// The Settings form (user 2026-11-24: "u tray dodaj Postavke link").
    Settings,
    /// The administrator's form.
    Admin,
    /// Opens the form of the mode again.
    OpenForm,
    /// Leaves the tray; never hidden.
    Quit,
}

/// Every item, in the menu's order.
pub const ALL: [Item; 10] = [
    Item::Status,
    Item::StopAll,
    Item::Mode(qnc_tray_mode::ALL[0]),
    Item::Mode(qnc_tray_mode::ALL[1]),
    Item::Mode(qnc_tray_mode::ALL[2]),
    Item::IngestFolder,
    Item::Settings,
    Item::Admin,
    Item::OpenForm,
    Item::Quit,
];

impl Item {
    /// Its name in the setting.
    pub fn code(self) -> &'static str {
        match self {
            Item::Status => "status",
            Item::StopAll => "stop_all",
            Item::Mode(mode) => match mode.key() {
                "breaking_news" => "mode_breaking_news",
                "story" => "mode_story",
                _ => "mode_net",
            },
            Item::IngestFolder => "ingest_folder",
            Item::Settings => "settings",
            Item::Admin => "admin",
            Item::OpenForm => "open_form",
            Item::Quit => "quit",
        }
    }

    /// The text code of its name (the tray's own).
    pub fn label(self) -> &'static str {
        match self {
            Item::Status => "tray.status_item",
            Item::StopAll => "tray.stop_all",
            Item::Mode(mode) => mode.label(),
            Item::IngestFolder => "tray.ingest_folder",
            Item::Settings => "tray.settings",
            Item::Admin => "tray.admin",
            Item::OpenForm => "tray.open_form",
            Item::Quit => "tray.quit",
        }
    }

    /// Whether the setting may hide it (Quit may not).
    pub fn may_hide(self) -> bool {
        self != Item::Quit
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct Setting {
    #[serde(default)]
    hidden: BTreeSet<String>,
}

fn file(root: &Path) -> PathBuf {
    root.join("data").join("tray-menu.qnctray")
}

/// The items hidden on this computer; no setting: none. A setting not read is an error.
pub fn hidden(root: &Path) -> Result<BTreeSet<String>, String> {
    let file = file(root);
    match std::fs::read_to_string(&file) {
        Ok(text) => serde_json::from_str::<Setting>(&text).map(|setting| setting.hidden).map_err(|error| format!("{}: {error}", file.display())),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(BTreeSet::new()),
        Err(error) => Err(format!("{}: {error}", file.display())),
    }
}

/// Keeps the items hidden as this computer's (Quit is never kept hidden).
pub fn set_hidden(root: &Path, hidden: &BTreeSet<String>) -> Result<(), String> {
    let file = file(root);
    if let Some(folder) = file.parent() {
        std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    }
    let hidden = hidden.iter().filter(|code| code.as_str() != Item::Quit.code()).cloned().collect();
    let text = serde_json::to_string_pretty(&Setting { hidden }).map_err(|error| error.to_string())?;
    std::fs::write(&file, text).map_err(|error| format!("{}: {error}", file.display()))
}

/// The items the menu shows, in its order: every item but the hidden ones; Quit always.
pub fn shown(hidden: &BTreeSet<String>) -> Vec<Item> {
    ALL.into_iter().filter(|item| !item.may_hide() || !hidden.contains(item.code())).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_item_shows_until_hidden_and_quit_is_never_hidden() {
        let root = std::env::temp_dir().join(format!("qnc-tray-menu-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(shown(&hidden(&root).unwrap()), ALL.to_vec(), "no setting: every item");
        let hide: BTreeSet<String> = ["admin", "quit", "mode_net"].into_iter().map(String::from).collect();
        set_hidden(&root, &hide).unwrap();
        let items = shown(&hidden(&root).unwrap());
        assert!(!items.contains(&Item::Admin) && !items.contains(&Item::Mode(qnc_tray_mode::Mode::Net)));
        assert_eq!(items.last(), Some(&Item::Quit), "Quit stays");
        let codes: BTreeSet<&str> = ALL.iter().map(|item| item.code()).collect();
        assert_eq!(codes.len(), ALL.len(), "every item has its own name");
        let _ = std::fs::remove_dir_all(&root);
    }
}
