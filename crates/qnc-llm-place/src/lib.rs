//! The LLM model of this computer (user 2026-11-22: "u Postavkama putanja ... LLM modela"; "samo
//! zapisati postavku za sada"): a path to a model file or the address of a server that serves one,
//! as it is typed. A setting of this computer, never of a project: `llm.qncllm` (JSON) beside its other
//! settings. Nothing in QNC-AI uses it yet.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// The setting of this computer, beside its other settings.
pub const FILE: &str = "llm.qncllm";

#[derive(Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct File {
    model: String,
}

fn path(root: &Path) -> PathBuf {
    qnc_dev_diagnostics::config_path(root).with_file_name(FILE)
}

/// The LLM model of this computer; none when it is not set. A setting that cannot be read is an
/// error.
pub fn model_of(root: &Path) -> Result<Option<String>, String> {
    let path = path(root);
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("{}: {error}", path.display())),
    };
    let file: File = serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
    Ok(Some(file.model).filter(|model| !model.trim().is_empty()))
}

/// Writes the LLM model of this computer; an empty one removes the setting.
pub fn set_model_of(root: &Path, model: &str) -> Result<(), String> {
    let path = path(root);
    let model = model.trim();
    if model.is_empty() {
        return match std::fs::remove_file(&path) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(format!("{}: {error}", path.display())),
            _ => Ok(()),
        };
    }
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    }
    let text = serde_json::to_string_pretty(&File { model: model.into() }).map_err(|error| error.to_string())?;
    std::fs::write(&path, text).map_err(|error| format!("{}: {error}", path.display()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_model_written_is_read_back_and_empty_removes_it() {
        let root = tempfile::tempdir().unwrap();
        assert_eq!(super::model_of(root.path()).unwrap(), None);
        super::set_model_of(root.path(), " C:/models/llm.gguf ").unwrap();
        assert_eq!(super::model_of(root.path()).unwrap().as_deref(), Some("C:/models/llm.gguf"));
        super::set_model_of(root.path(), "").unwrap();
        assert_eq!(super::model_of(root.path()).unwrap(), None);
    }
}
