//! The AI catalog of this computer edited (user 2026-11-23: AI settings "dodaj sve što imamo i što
//! je planirano: ... AI motori"). The catalog is the file the AI pieces already read
//! (`qnc-ai-catalog`, `catalogs/ai/catalog.json` beside the application); it is kept as JSON, so a
//! field this piece does not know stays as it is.
//!
//! What can change: which transcription engine is the best, the fast and the live one; the threads
//! and the clips at once of each engine; the model of each other AI job; the backend of the shot
//! caption; the pictures per shot of the shot content. A change is written only if no engine that
//! worked before stops working after it (the same checks the AI pieces make when they start); what
//! already did not work (a model not on this computer) does not stop a save.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// The transcription slots: the catalog's key and the text code of its name.
pub const TRANSCRIBE_SLOTS: [(&str, &str); 3] = [("selected", "ai_engines.best"), ("fast", "ai_engines.fast"), ("live", "ai_engines.live")];

/// The other AI jobs with a model: the catalog's key and the text code of its name.
pub const JOBS: [(&str, &str); 5] = [
    ("letters", "ai_engines.letters"),
    ("shot_content", "ai_engines.shot_content"),
    ("shot_caption", "ai_engines.shot_caption"),
    ("translate", "ai_engines.translate"),
    ("smooth", "ai_engines.smooth"),
];

/// A transcription engine of the catalog.
#[derive(Debug, Clone, PartialEq)]
pub struct Engine {
    pub id: String,
    pub program: String,
    pub backend: String,
    pub model: String,
    pub threads: u32,
    /// Clips heard at once; absent: one.
    pub parallel: Option<u32>,
}

/// The catalog read, as JSON.
#[derive(Debug, Clone, PartialEq)]
pub struct Catalog {
    pub path: PathBuf,
    pub value: Value,
}

impl Catalog {
    /// This computer's catalog.
    pub fn installed() -> Result<Self, String> {
        Self::read(&qnc_ai_catalog::installed_path()?)
    }

    /// The catalog at `path`.
    pub fn read(path: &Path) -> Result<Self, String> {
        let text = std::fs::read_to_string(path).map_err(|error| format!("{}: {error}", path.display()))?;
        let value = serde_json::from_str(&text).map_err(|error| format!("{}: {error}", path.display()))?;
        Ok(Self { path: path.to_path_buf(), value })
    }

    /// The transcription engines, in the catalog's order.
    pub fn engines(&self) -> Vec<Engine> {
        let list = self.value.pointer("/transcribe/engines").and_then(Value::as_array).cloned().unwrap_or_default();
        list.iter()
            .map(|engine| Engine {
                id: string(engine, "id"),
                program: string(engine, "program"),
                backend: string(engine, "backend"),
                model: string(engine, "model"),
                threads: engine.get("threads").and_then(Value::as_u64).unwrap_or(1) as u32,
                parallel: engine.get("parallel").and_then(Value::as_u64).map(|n| n as u32),
            })
            .collect()
    }

    /// The engine a transcription slot names; none when the catalog has no such slot.
    pub fn slot(&self, slot: &str) -> Option<String> {
        self.value.get("transcribe")?.get(slot)?.as_str().map(str::to_string)
    }

    /// Names engine `id` for the slot; `None` takes the slot away (not the best one: it must be there).
    pub fn set_slot(&mut self, slot: &str, id: Option<&str>) -> Result<(), String> {
        if let Some(id) = id {
            if !self.engines().iter().any(|engine| engine.id == id) {
                return Err(format!("the AI catalog has no transcription engine {id}"));
            }
        }
        let transcribe = self.value.get_mut("transcribe").and_then(Value::as_object_mut).ok_or("the AI catalog has no transcription")?;
        match id {
            Some(id) => {
                transcribe.insert(slot.into(), Value::from(id));
            }
            None if slot == "selected" => return Err("the best transcription engine cannot be taken away".into()),
            None => {
                transcribe.remove(slot);
            }
        }
        Ok(())
    }

    /// Sets the threads and the clips at once of engine `id` (at least 1 each).
    pub fn set_engine(&mut self, id: &str, threads: u32, parallel: Option<u32>) -> Result<(), String> {
        let engines = self.value.pointer_mut("/transcribe/engines").and_then(Value::as_array_mut).ok_or("the AI catalog has no transcription engines")?;
        let engine = engines
            .iter_mut()
            .find(|engine| engine.get("id").and_then(Value::as_str) == Some(id))
            .and_then(Value::as_object_mut)
            .ok_or_else(|| format!("the AI catalog has no transcription engine {id}"))?;
        engine.insert("threads".into(), Value::from(threads.max(1)));
        match parallel {
            Some(parallel) => engine.insert("parallel".into(), Value::from(parallel.max(1))),
            None => engine.remove("parallel"),
        };
        Ok(())
    }

    /// Whether the catalog has job `job`.
    pub fn has_job(&self, job: &str) -> bool {
        self.value.get(job).is_some_and(Value::is_object)
    }

    /// A field of a job (`model`, `backend`, `program`, `pictures`): as text, numbers too.
    pub fn job(&self, job: &str, field: &str) -> Option<String> {
        match self.value.get(job)?.get(field)? {
            Value::String(text) => Some(text.clone()),
            Value::Number(number) => Some(number.to_string()),
            _ => None,
        }
    }

    /// Sets a field of a job that is in the catalog; `pictures` is a whole number (1 to 3).
    pub fn set_job(&mut self, job: &str, field: &str, value: &str) -> Result<(), String> {
        let entry = self.value.get_mut(job).and_then(Value::as_object_mut).ok_or_else(|| format!("the AI catalog has no {job}"))?;
        let value = match field {
            "pictures" => match value.trim().parse::<u32>() {
                Ok(n @ 1..=3) => Value::from(n),
                _ => return Err("pictures per shot are 1 to 3".into()),
            },
            _ => Value::from(value.trim()),
        };
        entry.insert(field.into(), value);
        Ok(())
    }

    /// Writes the catalog, if every engine that worked in the catalog on disk still works in this
    /// one; else nothing is written and the engines it would break are said.
    pub fn save(&self) -> Result<(), String> {
        let folder = self.path.parent().ok_or("the AI catalog has no folder")?;
        let before = checks(&self.path);
        // The draft is checked beside the catalog, so its relative models resolve the same way.
        let draft = folder.join(format!(".catalog-draft-{}.json", std::process::id()));
        let text = serde_json::to_string_pretty(&self.value).map_err(|error| error.to_string())?;
        std::fs::write(&draft, format!("{text}\n")).map_err(|error| format!("{}: {error}", draft.display()))?;
        let after = checks(&draft);
        let _ = std::fs::remove_file(&draft);
        let broken: Vec<String> = before
            .iter()
            .zip(&after)
            .filter(|((_, was), (_, is))| was.is_ok() && is.is_err())
            .map(|((name, _), (_, is))| format!("{name}: {}", is.as_ref().err().cloned().unwrap_or_default()))
            .collect();
        if !broken.is_empty() {
            return Err(broken.join("; "));
        }
        std::fs::write(&self.path, format!("{text}\n")).map_err(|error| format!("{}: {error}", self.path.display()))
    }
}

/// The models in the shared AI folder, as the catalog names them (`share:models/<name>`); none when
/// the folder is not set or not read.
pub fn shared_models(root: &Path) -> Vec<String> {
    let Some(folder) = qnc_ai_catalog::chosen_share(root) else { return Vec::new() };
    let mut names: Vec<String> = std::fs::read_dir(folder.join("models"))
        .into_iter()
        .flatten()
        .flatten()
        .map(|entry| format!("share:models/{}", entry.file_name().to_string_lossy()))
        .collect();
    names.sort();
    names
}

fn string(value: &Value, key: &str) -> String {
    value.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

/// What the AI pieces check when they start, for the catalog at `path`.
fn checks(path: &Path) -> Vec<(&'static str, Result<(), String>)> {
    use qnc_ai_catalog::Quality;
    vec![
        ("transcription (best)", qnc_ai_catalog::transcriber_for(path, Quality::Best).map(drop)),
        ("transcription (fast)", qnc_ai_catalog::fast_transcribers(path).map(drop)),
        ("transcription (live)", qnc_ai_catalog::transcriber_for(path, Quality::Live).map(drop)),
        ("letters", qnc_ai_catalog::letter_aligner(path).map(drop)),
        ("shot content", qnc_ai_catalog::shot_content(path).map(drop)),
        ("shot caption", qnc_ai_catalog::shot_captioner(path).map(drop)),
        ("translation", qnc_ai_catalog::translator(path).map(drop)),
        ("smooth", qnc_ai_catalog::frame_interpolator(path).map(drop)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn catalog_in(folder: &Path) -> PathBuf {
        std::fs::create_dir_all(folder.join("models").join("a")).unwrap();
        let path = folder.join("catalog.json");
        let text = serde_json::json!({
            "version": "1",
            "transcribe": {
                "selected": "one",
                "fast": "one",
                "engines": [
                    { "id": "one", "program": "p", "backend": "cpu", "model": "models/a", "threads": 2 },
                    { "id": "two", "program": "p", "backend": "cpu", "model": "models/gone", "threads": 2, "parallel": 3 }
                ]
            }
        });
        std::fs::write(&path, serde_json::to_string_pretty(&text).unwrap()).unwrap();
        path
    }

    #[test]
    fn a_change_is_written_unless_it_breaks_an_engine_that_worked() {
        let folder = std::env::temp_dir().join(format!("qnc-ai-catalog-edit-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        let path = catalog_in(&folder);
        let mut catalog = Catalog::read(&path).unwrap();
        assert_eq!(catalog.engines().len(), 2);
        assert_eq!(catalog.slot("fast").as_deref(), Some("one"));
        catalog.set_engine("one", 4, Some(2)).unwrap();
        catalog.save().unwrap();
        let read = Catalog::read(&path).unwrap();
        assert_eq!((read.engines()[0].threads, read.engines()[0].parallel), (4, Some(2)));
        // The fast engine moved to one whose model is not here: refused, nothing written.
        let mut broken = read.clone();
        broken.set_slot("fast", Some("two")).unwrap();
        assert!(broken.save().unwrap_err().contains("fast"));
        assert_eq!(Catalog::read(&path).unwrap().slot("fast").as_deref(), Some("one"));
        assert!(broken.set_slot("selected", None).is_err() && broken.set_slot("fast", Some("none")).is_err());
        let _ = std::fs::remove_dir_all(&folder);
    }
}
