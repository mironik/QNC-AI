//! The keyboard preset the keys of every form follow (user 2026-11-24: "preset tipki je po
//! korisniku"): the one the user signed in chose (`keyboard-preset.json` in the user's
//! settings, `qnc-user-settings`), else the one the active project chose (user 2026-10-09:
//! Project, Advanced settings; projects made before the keys were the user's). A thread of its
//! own reads it once a second (the user's choice and the active project may change at any
//! time); a caller only takes the latest value, so no paint ever waits. Without either there is
//! no preset and the keyboard catalog keeps its own.

use qnc_active_project_read::ActiveProjectReader;
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

const READ_EVERY: Duration = Duration::from_secs(1);

/// The latest keyboard preset of the active project, kept up to date in the background.
pub struct KeyPresetWatcher {
    latest: Arc<Mutex<Option<String>>>,
}

impl KeyPresetWatcher {
    /// Starts reading the active project of the QNC `root`; the thread lives as long as
    /// the process.
    pub fn start(root: PathBuf) -> Self {
        let latest = Arc::new(Mutex::new(None));
        let shared = latest.clone();
        let _ = thread::Builder::new()
            .name("qnc-key-preset".into())
            .spawn(move || loop {
                let preset = read_preset(&root);
                *shared.lock().unwrap() = preset;
                thread::sleep(READ_EVERY);
            });
        Self { latest }
    }

    /// The preset the active project chose, as last read.
    pub fn preset(&self) -> Option<String> {
        self.latest.lock().unwrap().clone()
    }
}

/// The setting of the user's keyboard preset (user 2026-11-24: "preset tipki je po
/// korisniku"): `{"preset": "<id>"}` in the user's settings (`qnc-user-settings`).
pub const SETTING_FILE: &str = "keyboard-preset.json";

/// The preset of `root` the keys follow: the one the user signed in chose, else the one the
/// active project chose (projects made before the keys were the user's); `None` without either.
pub fn read_preset(root: &std::path::Path) -> Option<String> {
    user_preset(root).or_else(|| {
        let project = ActiveProjectReader::from_root(root).ok()?.read().ok()?;
        project.settings.keyboard_preset().map(str::to_owned)
    })
}

/// The preset the user signed in chose (or the computer, for every user); `None` without one.
pub fn user_preset(root: &std::path::Path) -> Option<String> {
    let text = std::fs::read_to_string(qnc_user_settings::read_from(root, SETTING_FILE)?).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).ok()?;
    value.get("preset").and_then(serde_json::Value::as_str).filter(|preset| !preset.is_empty()).map(str::to_owned)
}

/// Keeps `preset` as the keyboard preset of the user signed in.
pub fn choose_preset(root: &std::path::Path, preset: &str) -> Result<(), String> {
    qnc_user_settings::write(root, SETTING_FILE, &serde_json::json!({ "preset": preset }).to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_qnc_root_means_no_preset() {
        let dir = std::env::temp_dir().join("qnc-key-preset-none");
        assert_eq!(read_preset(&dir), None);
        let watcher = KeyPresetWatcher::start(dir);
        assert_eq!(watcher.preset(), None);
    }

    #[test]
    fn the_user_s_preset_is_kept_for_the_user_and_comes_first() {
        let root = std::env::temp_dir().join(format!("qnc-key-preset-user-{}", std::process::id()));
        std::fs::create_dir_all(root.join("data")).unwrap();
        assert_eq!(read_preset(&root), None);
        choose_preset(&root, "resolve").unwrap();
        assert_eq!(read_preset(&root).as_deref(), Some("resolve"));
        assert!(qnc_user_settings::user_file(&root, SETTING_FILE).unwrap().is_file());
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod live {
    /// Reads the preset of the active project of `QNC_KEY_PRESET_ROOT` (a QNC root, for a
    /// check on a protected copy); run with `--ignored --nocapture`.
    #[test]
    #[ignore = "reads the QNC root given in QNC_KEY_PRESET_ROOT"]
    fn preset_of_a_given_root() {
        let root = std::env::var("QNC_KEY_PRESET_ROOT").expect("QNC_KEY_PRESET_ROOT");
        println!("preset={:?}", super::read_preset(std::path::Path::new(&root)));
    }
}
