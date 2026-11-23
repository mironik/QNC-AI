//! The export templates of this computer (user 2026-11-23: "za export file trebamo moći imati više
//! templatea, obrazac za kreiranje korisničkih postavki koristeći naše predefinirane postavke, kako bi
//! korisnik mogao prilagoditi svojim potrebama"; "na računalu").
//!
//! A template is made from a prepared export preset (`contracts/export_profiles.json`, through
//! `qnc-export-preset`), which is never written over: the user's own is a file of its own in
//! `data/export-templates`. Its values are `export.*` keys of the project settings; the fields it
//! has are the contract `contracts/export-template-fields.json`. Nothing is made up: a value not
//! set is not in the template.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// The fields of a template, as the contract names them.
const FIELDS_JSON: &str = include_str!("../../../contracts/export-template-fields.json");

/// A group of fields (picture, sound, delivery).
#[derive(Debug, Clone, Deserialize)]
pub struct FieldGroup {
    pub label: String,
    pub fields: Vec<Field>,
}

/// A field: its `export.*` key, its label's text code and how it is set.
#[derive(Debug, Clone, Deserialize)]
pub struct Field {
    pub key: String,
    pub label: String,
    pub kind: FieldKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldKind {
    /// One of the values the prepared presets have.
    Choice,
    /// A number typed.
    Number,
    /// A text typed.
    Text,
    /// On or off.
    Switch,
}

/// The groups of fields of a template; a contract that cannot be read is an error.
pub fn field_groups() -> Result<Vec<FieldGroup>, String> {
    #[derive(Deserialize)]
    struct Contract {
        groups: Vec<FieldGroup>,
    }
    serde_json::from_str::<Contract>(FIELDS_JSON).map(|contract| contract.groups).map_err(|error| format!("export-template-fields.json: {error}"))
}

/// A prepared export preset: its id, name and values.
#[derive(Debug, Clone, PartialEq)]
pub struct Prepared {
    pub id: String,
    pub name: String,
    pub values: Map<String, Value>,
}

/// The prepared export presets QNC brings.
pub fn prepared() -> Vec<Prepared> {
    qnc_export_preset::presets(&Value::Null)
        .into_iter()
        .map(|preset| Prepared { id: preset.id, name: preset.name, values: preset.values.as_object().cloned().unwrap_or_default() })
        .collect()
}

/// The values a choice field offers: every value the prepared presets have for its key, once.
pub fn choices(key: &str) -> Vec<Value> {
    let mut values: Vec<Value> = Vec::new();
    for preset in prepared() {
        if let Some(value) = preset.values.get(key).filter(|value| !values.contains(value)) {
            values.push(value.clone());
        }
    }
    values
}

/// A user's export template of this computer.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Template {
    pub id: String,
    pub name: String,
    /// The prepared preset it was made from.
    pub from: String,
    /// `export.*` keys and their values.
    pub values: Map<String, Value>,
}

fn folder(root: &Path) -> PathBuf {
    root.join("data").join("export-templates")
}

/// The templates of this computer, by name; a file that cannot be read is an error, not skipped.
pub fn templates(root: &Path) -> Result<Vec<Template>, String> {
    let folder = folder(root);
    let entries = match std::fs::read_dir(&folder) {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("{}: {error}", folder.display())),
    };
    let mut templates = Vec::new();
    for entry in entries {
        let path = entry.map_err(|error| error.to_string())?.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let text = std::fs::read_to_string(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        templates.push(serde_json::from_str::<Template>(&text).map_err(|error| format!("{}: {error}", path.display()))?);
    }
    templates.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(templates)
}

/// The id of a template named `name`: its name in lower case, letters and digits joined by `_`;
/// never the id of a prepared preset (those are never written over).
pub fn id_of(name: &str) -> String {
    let mut id = String::new();
    for word in name.to_lowercase().split(|c: char| !c.is_alphanumeric()).filter(|word| !word.is_empty()) {
        if !id.is_empty() {
            id.push('_');
        }
        id.push_str(word);
    }
    if prepared().iter().any(|preset| preset.id == id) {
        id.push_str("_own");
    }
    id
}

/// Keeps `template` (an empty name is refused); a template of the same id is written over.
pub fn save(root: &Path, template: &Template) -> Result<(), String> {
    if template.name.trim().is_empty() || template.id.is_empty() {
        return Err("an export template needs a name".into());
    }
    if prepared().iter().any(|preset| preset.id == template.id) {
        return Err(format!("{} is a prepared preset, it is never written over", template.id));
    }
    let folder = folder(root);
    std::fs::create_dir_all(&folder).map_err(|error| format!("{}: {error}", folder.display()))?;
    let file = folder.join(format!("{}.json", template.id));
    let text = serde_json::to_string_pretty(template).map_err(|error| error.to_string())?;
    std::fs::write(&file, text).map_err(|error| format!("{}: {error}", file.display()))
}

/// Deletes the template `id` of this computer (a prepared preset is no file here).
pub fn delete(root: &Path, id: &str) -> Result<(), String> {
    let file = folder(root).join(format!("{id}.json"));
    std::fs::remove_file(&file).map_err(|error| format!("{}: {error}", file.display()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_template_is_made_from_a_prepared_preset_kept_on_the_computer_and_the_preset_is_never_written_over() {
        let root = std::env::temp_dir().join(format!("qnc-export-templates-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        assert!(templates(&root).unwrap().is_empty());
        let groups = field_groups().unwrap();
        assert!(groups.iter().flat_map(|group| &group.fields).any(|field| field.key == "video_codec" && field.kind == FieldKind::Choice));
        assert!(choices("container").len() >= 2, "the containers the prepared presets have");
        let preset = prepared().into_iter().find(|preset| preset.id == "h264_1080p50").unwrap();
        let mut values = preset.values.clone();
        values.insert("video_bitrate_mbps".into(), 25.into());
        let template = Template { id: id_of("HRT vijesti 25"), name: "HRT vijesti 25".into(), from: preset.id.clone(), values };
        assert_eq!(template.id, "hrt_vijesti_25");
        save(&root, &template).unwrap();
        assert_eq!(templates(&root).unwrap(), vec![template.clone()]);
        assert_eq!(id_of("H264 1080p50"), "h264_1080p50_own", "a prepared preset's id is never taken");
        assert!(save(&root, &Template { id: "h264_1080p50".into(), ..template.clone() }).is_err());
        delete(&root, "hrt_vijesti_25").unwrap();
        assert!(templates(&root).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
