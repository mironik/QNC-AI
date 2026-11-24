//! The settings of the user of this computer (user 2026-11-24: "racunalo moze imati vise
//! korisnika s razlicitim preferencijama"; "preset tipki je po korisniku, jezik po korisniku").
//! Each user the operating system signs in has a folder of their own beside the computer's
//! settings (`data/users/<user>/`); the user is who the system says it is (`USERNAME` on
//! Windows, `USER` elsewhere), never one made up. A setting is read from the user's folder,
//! and where the user has none from the computer's (`data/<file>`), so what the computer
//! chose before keeps working; a setting is always written into the user's folder.

use std::path::{Path, PathBuf};

pub const MODULE_ID: &str = "qnc.module.user-settings";

/// The folder of every user's settings, beside the computer's.
pub const USERS_FOLDER: &str = "users";

/// The user the operating system signed in, as a folder name: letters, digits, `-`, `_`, `.`
/// and spaces kept, anything else `_`. An error when the system names none.
pub fn user_name() -> Result<String, String> {
    let name = ["USERNAME", "USER"].iter().find_map(|key| std::env::var(key).ok().filter(|value| !value.trim().is_empty()));
    let Some(name) = name else {
        return Err("the system names no user (USERNAME / USER)".to_string());
    };
    folder_name(&name)
}

fn folder_name(name: &str) -> Result<String, String> {
    let kept: String = name.trim().chars().map(|c| if c.is_alphanumeric() || "-_. ".contains(c) { c } else { '_' }).collect();
    let kept = kept.trim_matches(|c| c == '.' || c == ' ').to_string();
    if kept.is_empty() {
        return Err(format!("the user name '{name}' gives no folder"));
    }
    Ok(kept)
}

/// The computer's setting `file` (`data/<file>` of the QNC `root`).
pub fn computer_file(root: &Path, file: &str) -> PathBuf {
    qnc_dev_diagnostics::config_path(root).with_file_name(file)
}

/// The settings folder of the user signed in (`data/users/<user>`); it may not exist yet.
pub fn user_dir(root: &Path) -> Result<PathBuf, String> {
    Ok(computer_file(root, USERS_FOLDER).join(user_name()?))
}

/// The user's setting `file`; it may not exist yet.
pub fn user_file(root: &Path, file: &str) -> Result<PathBuf, String> {
    Ok(user_dir(root)?.join(file))
}

/// Where `file` is read from: the user's when the user has it, else the computer's when it
/// has it; `None` when neither chose.
pub fn read_from(root: &Path, file: &str) -> Option<PathBuf> {
    user_file(root, file).ok().filter(|path| path.is_file()).or_else(|| Some(computer_file(root, file)).filter(|path| path.is_file()))
}

/// Writes `text` as the user's setting `file`, the user's folder made when it is not there.
pub fn write(root: &Path, file: &str, text: &str) -> Result<(), String> {
    let path = user_file(root, file)?;
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|error| format!("{}: {error}", dir.display()))?;
    }
    std::fs::write(&path, text).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_user_name_becomes_a_folder_and_none_is_an_error() {
        assert_eq!(folder_name("Miro Nikoletić").unwrap(), "Miro Nikoletić");
        assert_eq!(folder_name("dom\\miro").unwrap(), "dom_miro");
        assert!(folder_name("..").is_err());
        assert!(folder_name("  ").is_err());
    }

    #[test]
    fn the_user_s_setting_comes_before_the_computer_s_and_is_written_for_the_user() {
        let root = std::env::temp_dir().join(format!("qnc-user-settings-{}", std::process::id()));
        std::fs::create_dir_all(root.join("data")).unwrap();
        assert_eq!(read_from(&root, "x.json"), None);
        std::fs::write(computer_file(&root, "x.json"), "computer").unwrap();
        assert_eq!(read_from(&root, "x.json"), Some(computer_file(&root, "x.json")));
        write(&root, "x.json", "user").unwrap();
        let path = read_from(&root, "x.json").unwrap();
        assert_eq!(path, user_file(&root, "x.json").unwrap());
        assert_eq!(std::fs::read_to_string(path).unwrap(), "user");
        let _ = std::fs::remove_dir_all(&root);
    }
}
