//! The header of a story (user 2026-11-24: "kad korisnik ide na skript formu, najprije se otvori
//! overlay obrazac za naziv, novinar, snimatelj... nakon toga se otvara obrazac za pisanje"):
//! the fields of the project's story sheet (show, date, slug, length, journalists, editors,
//! camera operators, source, notes for the gallery) filled in an overlay before the story is
//! written. A project without a sheet gets the prepared TV package sheet. Nobody's name is
//! guessed: a journalist's laptop does not know who writes on it, so names are typed, and a
//! field of names takes several (several journalists may make one story).
//!
//! The sheet is read from and kept in the project database (`qnc-newsroom-sheet-db`) off the
//! caller's thread; what is typed is a draft here until it is kept or let go. It draws nothing:
//! a panel shows `HeaderView` and sends the actions below.

use std::collections::BTreeMap;
use std::sync::mpsc::{self, Receiver};

use qnc_db_broker::ProjectDbTarget;
use qnc_field_spec::FieldKind;
use qnc_newsroom_sheet_db::StorySheet;
use qnc_newsroom_sheet_model::{SheetSpec, Sheets};
use serde_json::Value;

pub const MODULE_ID: &str = "qnc.module.story-header";

/// Opens the header again (a click on the story's title).
pub const OPEN: &str = "story_header_open";
/// The text of a field as typed; target `path \u{1f} text`.
pub const TEXT: &str = "story_header_text";
/// One name of a field of names; target `path \u{1f} index \u{1f} name` (an index past the last
/// adds a name).
pub const NAME: &str = "story_header_name";
/// Takes a name out of a field of names; target `path \u{1f} index`.
pub const REMOVE_NAME: &str = "story_header_remove_name";
/// Keeps the header in the project and closes it.
pub const CONFIRM: &str = "story_header_confirm";
/// Closes it, nothing kept.
pub const CANCEL: &str = "story_header_cancel";

/// The sheet a project without one gets (user 2026-11-24: the TV package of the newsroom).
pub const PREPARED: &str = "tv_prilog";

/// How the names of a field of names are kept in its text.
const NAMES_JOIN: &str = ", ";

/// One field as the overlay shows it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HeaderField {
    pub path: String,
    pub label: String,
    pub kind: Option<FieldKind>,
    pub hint: String,
    /// Its text (a length as `m:ss`, a choice by its code).
    pub value: String,
    /// The names of a field of names, one per row.
    pub names: Vec<String>,
    /// The choices of a field of choices: `(code, name)`.
    pub choices: Vec<(String, String)>,
}

/// What a panel shows of the header.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HeaderView {
    /// The overlay is open (before writing, or opened again).
    pub open: bool,
    /// The sheet's name.
    pub sheet_name: String,
    pub fields: Vec<HeaderField>,
    /// The header as people read it (name of the field, value), for the header shown in
    /// several columns over the story and in the documents; the fields with a value only.
    pub shown: Vec<(String, String)>,
    pub message: String,
    pub saving: bool,
}

#[derive(Default)]
pub struct StoryHeader {
    project: Option<String>,
    target: Option<ProjectDbTarget>,
    sheet: Option<SheetSpec>,
    /// What is kept in the project.
    kept: BTreeMap<String, Value>,
    /// What is typed (the overlay's values).
    draft: BTreeMap<String, Value>,
    open: bool,
    reading: Option<Receiver<Result<(SheetSpec, BTreeMap<String, Value>), String>>>,
    saving: Option<Receiver<Result<(), String>>>,
    message: String,
    view: HeaderView,
}

impl std::fmt::Debug for StoryHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StoryHeader").field("project", &self.project).field("open", &self.open).finish()
    }
}

impl StoryHeader {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn view(&self) -> &HeaderView {
        &self.view
    }

    /// Follows the project of `target`: a project opened reads its header, then the overlay
    /// opens before the story is written; the answers of reads and writes are taken. Returns
    /// whether the view changed.
    pub fn poll(&mut self, target: Option<&ProjectDbTarget>) -> bool {
        let before = self.view.clone();
        let key = target.map(|target| target.uri().to_string());
        if key != self.project {
            *self = Self { project: key, target: target.cloned(), ..Self::default() };
            if let Some(target) = target.cloned() {
                let (send, receive) = mpsc::channel();
                if std::thread::Builder::new().name("qnc-story-header-read".into()).spawn(move || _ = send.send(read(&target))).is_ok() {
                    self.reading = Some(receive);
                }
            }
        }
        if let Some(Ok(read)) = self.reading.as_ref().map(Receiver::try_recv) {
            self.reading = None;
            match read {
                Ok((sheet, values)) => {
                    self.sheet = Some(sheet);
                    self.kept = values;
                    self.begin();
                }
                Err(error) => self.message = error,
            }
        }
        if let Some(Ok(saved)) = self.saving.as_ref().map(Receiver::try_recv) {
            self.saving = None;
            match saved {
                Ok(()) => self.message.clear(),
                Err(error) => {
                    // Not kept: the overlay opens again with what was typed.
                    self.message = error;
                    self.open = true;
                }
            }
        }
        self.refresh();
        self.view != before
    }

    /// The header's own actions; `None` for any other.
    pub fn act(&mut self, action_id: &str, target: Option<&str>) -> Option<bool> {
        let parts: Vec<&str> = target.map(|target| target.split('\u{1f}').collect()).unwrap_or_default();
        match action_id {
            OPEN => self.begin(),
            TEXT => {
                let [path, text] = parts[..] else { return Some(false) };
                let value = match self.kind_of(path) {
                    Some(FieldKind::Duration) => seconds_of(text).map_or_else(|| Value::String(text.into()), Value::from),
                    _ => Value::String(text.into()),
                };
                self.draft.insert(path.into(), value);
            }
            NAME => {
                let [path, index, name] = parts[..] else { return Some(false) };
                let Ok(index) = index.parse::<usize>() else { return Some(false) };
                let mut names = names_of(self.draft.get(path));
                if index < names.len() {
                    names[index] = name.replace(',', " ");
                } else {
                    names.push(name.replace(',', " "));
                }
                self.draft.insert(path.into(), Value::String(names.join(NAMES_JOIN)));
            }
            REMOVE_NAME => {
                let [path, index] = parts[..] else { return Some(false) };
                let Ok(index) = index.parse::<usize>() else { return Some(false) };
                let mut names = names_of(self.draft.get(path));
                if index < names.len() {
                    names.remove(index);
                }
                self.draft.insert(path.into(), Value::String(names.join(NAMES_JOIN)));
            }
            CONFIRM => self.keep(),
            CANCEL => {
                self.open = false;
                self.draft = self.kept.clone();
            }
            _ => return None,
        }
        self.refresh();
        Some(true)
    }

    /// Opens the overlay with what is kept; a date left empty is today.
    fn begin(&mut self) {
        let Some(sheet) = &self.sheet else { return };
        self.draft = self.kept.clone();
        for field in sheet.fields.iter().filter(|field| field.kind == FieldKind::Date) {
            if self.draft.get(&field.path).and_then(Value::as_str).is_none_or(|date| date.trim().is_empty()) {
                self.draft.insert(field.path.clone(), Value::String(qnc_newsroom_sheet_model::date_today()));
            }
        }
        self.open = true;
    }

    /// Keeps the draft in the project off the caller's thread; empty names are left out.
    fn keep(&mut self) {
        let (Some(sheet), Some(target)) = (self.sheet.clone(), self.target.clone()) else { return };
        let mut values = BTreeMap::new();
        for field in &sheet.fields {
            let Some(value) = self.draft.get(&field.path) else { continue };
            let value = match field.kind {
                FieldKind::Names => Value::String(names_of(Some(value)).join(NAMES_JOIN)),
                _ => value.clone(),
            };
            values.insert(field.path.clone(), value);
        }
        self.kept = values.clone();
        self.open = false;
        let story = StorySheet { sheet: serde_json::to_value(&sheet).unwrap_or(Value::Null), values };
        let (send, receive) = mpsc::channel();
        if std::thread::Builder::new().name("qnc-story-header-save".into()).spawn(move || _ = send.send(qnc_newsroom_sheet_db::save(&target, story))).is_ok() {
            self.saving = Some(receive);
        }
    }

    fn kind_of(&self, path: &str) -> Option<FieldKind> {
        self.sheet.as_ref()?.fields.iter().find(|field| field.path == path).map(|field| field.kind)
    }

    fn refresh(&mut self) {
        let Some(sheet) = &self.sheet else {
            self.view = HeaderView { message: self.message.clone(), ..HeaderView::default() };
            return;
        };
        let values = if self.open { &self.draft } else { &self.kept };
        let fields: Vec<HeaderField> = sheet
            .fields
            .iter()
            .map(|field| {
                let stored = values.get(&field.path);
                HeaderField {
                    path: field.path.clone(),
                    label: field.label.clone(),
                    kind: Some(field.kind),
                    hint: field.hint.clone(),
                    value: text_of(stored, field.kind),
                    names: if field.kind == FieldKind::Names { names_of(stored) } else { Vec::new() },
                    choices: field.options.as_ref().and_then(|set| sheet.options.get(set)).cloned().unwrap_or_default(),
                }
            })
            .collect();
        let kept_fields: Vec<(String, String)> = sheet
            .fields
            .iter()
            .filter_map(|field| {
                let text = text_of(self.kept.get(&field.path), field.kind);
                let text = match field.options.as_ref().and_then(|set| sheet.options.get(set)) {
                    Some(choices) => choices.iter().find(|(code, _)| *code == text).map_or(text, |(_, name)| name.clone()),
                    None => text,
                };
                (!text.trim().is_empty()).then(|| (field.label.clone(), text))
            })
            .collect();
        self.view = HeaderView { open: self.open, sheet_name: sheet.name.clone(), fields, shown: kept_fields, message: self.message.clone(), saving: self.saving.is_some() };
    }
}

/// The sheet of the project and its values; a project without a sheet gets the prepared TV
/// package sheet (else the one chosen on this computer).
fn read(target: &ProjectDbTarget) -> Result<(SheetSpec, BTreeMap<String, Value>), String> {
    let story = qnc_newsroom_sheet_db::read(target)?;
    if let Ok(sheet) = serde_json::from_value::<SheetSpec>(story.sheet.clone()) {
        return Ok((sheet, story.values));
    }
    let root = qnc_dev_diagnostics::resolve_qnc_root().ok_or_else(|| qnc_ui_text::text("header.no_sheet").to_string())?;
    let (sheets, errors) = Sheets::load(&root);
    for error in errors {
        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("story-header sheet refused {error}"));
    }
    let sheet = sheets
        .list
        .iter()
        .map(|(sheet, _)| sheet)
        .find(|sheet| sheet.sheet_id == PREPARED)
        .or_else(|| sheets.current())
        .cloned()
        .ok_or_else(|| qnc_ui_text::text("header.no_sheet").to_string())?;
    Ok((sheet, story.values))
}

/// The text of a value: a length as `m:ss`, a number as it is, text as it is.
fn text_of(value: Option<&Value>, kind: FieldKind) -> String {
    match (value, kind) {
        (Some(Value::Number(seconds)), FieldKind::Duration) => seconds.as_f64().map(|seconds| minutes_seconds(seconds.round() as i64)).unwrap_or_default(),
        (Some(Value::Number(number)), _) => number.to_string(),
        (Some(Value::String(text)), _) => text.clone(),
        _ => String::new(),
    }
}

/// The names of a field of names (kept as text joined by ", ").
fn names_of(value: Option<&Value>) -> Vec<String> {
    value.and_then(Value::as_str).map(|text| text.split(',').map(str::trim).filter(|name| !name.is_empty()).map(str::to_string).collect()).unwrap_or_default()
}

fn minutes_seconds(seconds: i64) -> String {
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

/// A length typed as `m:ss` or whole seconds.
fn seconds_of(text: &str) -> Option<i64> {
    let text = text.trim();
    match text.split_once(':') {
        Some((minutes, seconds)) => Some(minutes.trim().parse::<i64>().ok()? * 60 + seconds.trim().parse::<i64>().ok().filter(|s| *s < 60)?),
        None => text.parse().ok(),
    }
}

#[cfg(test)]
mod tests;
