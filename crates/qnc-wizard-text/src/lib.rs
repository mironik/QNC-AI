//! The words of the QNC AI wizard. Every text has a code (as in QNC); the dictionaries are the
//! wizard's own (`contracts/text`), the language is the one this computer chose for QNC
//! (`ui-language.json` beside its settings), English without it. A language the wizard has no
//! dictionary for falls to English; a code a dictionary does not have is an error in the tests
//! and shows as its code, never a made-up text.

use std::collections::BTreeMap;
use std::sync::OnceLock;

use serde::Deserialize;
use serde_json::Value;

pub const BASE_LANGUAGE: &str = "en";

/// The dictionaries of the wizard.
pub const DICTIONARIES: &[(&str, &str)] =
    &[("en", include_str!("../../../contracts/text/en.json")), ("hr", include_str!("../../../contracts/text/hr.json"))];

#[derive(Deserialize)]
struct Dictionary {
    language: String,
    texts: BTreeMap<String, String>,
}

/// The texts of one language.
#[derive(Debug, Clone)]
pub struct Texts {
    pub language: String,
    texts: BTreeMap<String, String>,
}

impl Texts {
    pub fn for_language(language: &str) -> Result<Self, String> {
        let (_, json) = DICTIONARIES.iter().find(|(code, _)| *code == language).ok_or_else(|| format!("no wizard dictionary for '{language}'"))?;
        let file: Dictionary = serde_json::from_str(json).map_err(|error| format!("wizard dictionary '{language}': {error}"))?;
        if file.language != language {
            return Err(format!("wizard dictionary '{language}' says it is '{}'", file.language));
        }
        Ok(Self { language: language.into(), texts: file.texts })
    }

    pub fn get(&self, code: &str) -> Option<&str> {
        self.texts.get(code).map(String::as_str)
    }

    pub fn codes(&self) -> impl Iterator<Item = &str> {
        self.texts.keys().map(String::as_str)
    }
}

/// The texts of this computer's language (English when the wizard has none for it).
pub fn host() -> &'static Texts {
    static HOST: OnceLock<Texts> = OnceLock::new();
    HOST.get_or_init(|| {
        let language = if qnc_dev_diagnostics::unit_test() {
            BASE_LANGUAGE.to_string()
        } else {
            qnc_dev_diagnostics::resolve_qnc_root().map_or_else(|| BASE_LANGUAGE.to_string(), |root| qnc_ui_text::chosen_language(&root))
        };
        Texts::for_language(&language).or_else(|_| Texts::for_language(BASE_LANGUAGE)).expect("the base wizard dictionary")
    })
}

/// The text of a code.
pub fn text(code: &str) -> String {
    host().get(code).unwrap_or(code).to_string()
}

/// The text of a code with `{name}` places filled.
pub fn fill(code: &str, values: &[(&str, &str)]) -> String {
    values.iter().fold(text(code), |text, (name, value)| text.replace(&format!("{{{name}}}"), value))
}

/// A contract with every `@code` string replaced by its text.
pub fn resolve(value: &mut Value) {
    match value {
        Value::String(string) if string.starts_with('@') => *string = text(&string[1..]),
        Value::Array(items) => items.iter_mut().for_each(resolve),
        Value::Object(map) => map.values_mut().for_each(resolve),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_dictionary_has_every_code_of_english() {
        let base = Texts::for_language(BASE_LANGUAGE).unwrap();
        for (language, _) in DICTIONARIES {
            let texts = Texts::for_language(language).unwrap();
            for code in base.codes() {
                assert!(texts.get(code).is_some(), "'{language}' has no '{code}'");
            }
            assert_eq!(texts.codes().count(), base.codes().count(), "'{language}' has codes English has not");
        }
    }

    #[test]
    fn a_code_is_filled() {
        assert_eq!(fill("wizard.step_of", &[("n", "2"), ("all", "10")]), "Step 2 of 10");
    }
}
