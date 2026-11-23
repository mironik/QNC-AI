//! The shot labels of this computer edited (user 2026-11-23: AI settings "dodaj sve što imamo i što
//! je planirano: ... oznake kadrova"). The list is the file the AI pieces already read
//! (`qnc-shot-labels`, `catalogs/ai/shot-labels.json` beside the application); it is kept as JSON,
//! so a field this piece does not know stays as it is.
//!
//! What can change: each label's name, what the model is asked, whether it is a statement; new
//! labels; the two thresholds; the caption rules (which label the words of a caption give). A list
//! is written only if `qnc_shot_labels::Labels::read` reads it, so the AI pieces never get one they
//! refuse. When the list changes, the AI pieces work out its vectors again by themselves.

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

/// A label as edited.
#[derive(Debug, Clone, PartialEq)]
pub struct Label {
    pub code: String,
    /// The dictionary key of its name, or the name as typed (a key the dictionary lacks shows as
    /// it is written).
    pub text: String,
    pub prompt: String,
    pub statement: bool,
}

/// A caption rule as edited: its label and its words, one string, separated by commas.
#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub label: String,
    pub words: String,
}

/// The list read, as JSON, and what of it is edited.
#[derive(Debug, Clone, PartialEq)]
pub struct List {
    pub path: PathBuf,
    value: Value,
    pub least: f32,
    pub statement_least: f32,
    pub labels: Vec<Label>,
    pub rules: Vec<Rule>,
}

impl List {
    /// This computer's list.
    pub fn installed() -> Result<Self, String> {
        Self::read(&qnc_shot_labels::installed_path()?)
    }

    /// The list at `path`.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let value: Value = serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        let number = |key: &str, default: f32| value.get(key).and_then(Value::as_f64).map_or(default, |n| n as f32);
        let string = |item: &Value, key: &str| item.get(key).and_then(Value::as_str).unwrap_or_default().to_string();
        let labels = value
            .get("labels")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|label| Label { code: string(label, "code"), text: string(label, "text"), prompt: string(label, "prompt"), statement: label.get("statement").and_then(Value::as_bool).unwrap_or(false) })
            .collect();
        let rules = value
            .get("caption_rules")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .map(|rule| Rule {
                label: string(rule, "label"),
                words: rule.get("words").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect::<Vec<_>>().join(", "),
            })
            .collect();
        Ok(Self { path: path.to_path_buf(), least: number("least", 0.15), statement_least: number("statement_least", 0.05), labels, rules, value })
    }

    /// A new label: its code made of the name (lower case letters, digits and `_`), never one the
    /// list has; an empty name or prompt is refused.
    pub fn add(&mut self, name: &str, prompt: &str) -> Result<(), String> {
        let (name, prompt) = (name.trim(), prompt.trim());
        if name.is_empty() || prompt.is_empty() {
            return Err("a label needs a name and what the model is asked".into());
        }
        let base: String = name.to_lowercase().chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect::<String>().trim_matches('_').to_string();
        let base = if base.is_empty() { "label".to_string() } else { base };
        let mut code = base.clone();
        let mut n = 2;
        while self.labels.iter().any(|label| label.code == code) {
            code = format!("{base}_{n}");
            n += 1;
        }
        self.labels.push(Label { code, text: name.into(), prompt: prompt.into(), statement: false });
        Ok(())
    }

    /// The list as it would be written.
    fn as_value(&self) -> Value {
        let mut value = self.value.as_object().cloned().unwrap_or_else(Map::new);
        value.insert("least".into(), Value::from(f64::from(self.least)));
        value.insert("statement_least".into(), Value::from(f64::from(self.statement_least)));
        let labels = self
            .labels
            .iter()
            .map(|label| {
                let mut item = Map::new();
                item.insert("code".into(), Value::from(label.code.clone()));
                item.insert("text".into(), Value::from(label.text.trim()));
                item.insert("prompt".into(), Value::from(label.prompt.trim()));
                if label.statement {
                    item.insert("statement".into(), Value::from(true));
                }
                Value::Object(item)
            })
            .collect();
        value.insert("labels".into(), Value::Array(labels));
        let rules = self
            .rules
            .iter()
            .map(|rule| {
                let words: Vec<Value> = rule.words.split(',').map(str::trim).filter(|word| !word.is_empty()).map(Value::from).collect();
                serde_json::json!({ "label": rule.label, "words": words })
            })
            .collect();
        value.insert("caption_rules".into(), Value::Array(rules));
        Value::Object(value)
    }

    /// Writes the list if the AI pieces read it; else nothing is written and why is said.
    pub fn save(&self) -> Result<(), String> {
        let folder = self.path.parent().ok_or("the shot labels have no folder")?;
        let text = format!("{}\n", serde_json::to_string_pretty(&self.as_value()).map_err(|error| error.to_string())?);
        let draft = folder.join(format!(".shot-labels-draft-{}.json", std::process::id()));
        std::fs::write(&draft, &text).map_err(|error| format!("{}: {error}", draft.display()))?;
        let read = qnc_shot_labels::Labels::read(&draft);
        let _ = std::fs::remove_file(&draft);
        read.map_err(|_| "the list is not usable: every label needs a name and a prompt, the shares are below 1, every caption rule names a label of the list and has words".to_string())?;
        std::fs::write(&self.path, text).map_err(|error| format!("{}: {error}", self.path.display()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_list_is_written_only_when_the_ai_pieces_read_it_and_unknown_fields_stay() {
        let folder = std::env::temp_dir().join(format!("qnc-shot-labels-edit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        let path = folder.join("shot-labels.json");
        let start = serde_json::json!({
            "version": 1, "about": "kept", "least": 0.15, "statement_least": 0.011,
            "labels": [{ "code": "person", "text": "shot.label.person", "prompt": "a photo of a person" }],
            "caption_rules": [{ "label": "person", "words": ["man", "woman"] }]
        });
        std::fs::write(&path, serde_json::to_string(&start).unwrap()).unwrap();
        let mut list = List::read(&path).unwrap();
        assert_eq!(list.rules[0].words, "man, woman");
        list.add("Brod na moru", "a photo of a ship at sea").unwrap();
        list.add("Brod na moru", "a photo of a boat").unwrap();
        assert_eq!((list.labels[1].code.as_str(), list.labels[2].code.as_str()), ("brod_na_moru", "brod_na_moru_2"));
        list.rules.push(Rule { label: "brod_na_moru".into(), words: "ship, boat".into() });
        list.save().unwrap();
        let read = List::read(&path).unwrap();
        assert_eq!((read.labels.len(), read.rules.len()), (3, 2));
        assert!(std::fs::read_to_string(&path).unwrap().contains("kept"));
        // A rule naming a label the list does not have: not written.
        let mut bad = read.clone();
        bad.rules.push(Rule { label: "none".into(), words: "x".into() });
        assert!(bad.save().is_err());
        assert_eq!(List::read(&path).unwrap().rules.len(), 2);
        let _ = std::fs::remove_dir_all(&folder);
    }
}
