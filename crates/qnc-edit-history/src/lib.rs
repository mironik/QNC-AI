//! The history of the edits of each clip (user 2026-11-25: "the zoom gets a history, a drop-down in
//! its header bar; the user can go back to any position of it"; "it is kept per clip, not
//! together"). Every Save (Enter) of an edit of a clip writes one entry: when, what it says (a
//! label to read) and the state after it, as its owner wrote it. The first entry of a clip is the
//! state before its first edit, so its start can always be gone back to. Going back to an entry is
//! the owner's write and itself a new entry, so nothing is lost.
//!
//! A table module of the project database intermediary (`qnc-db-broker`): it owns only its table,
//! is written only through that intermediary and knows no form, owner or kind of edit.

mod store;

use std::sync::Arc;

use qnc_db_broker::{Access, ProjectDbTarget, ProjectDbWriter, TableModuleFactory};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MODULE_ID: &str = "qnc.module.edit-history";

pub type Result<T> = std::result::Result<T, String>;

/// One entry of a clip's history.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Its number in the project (`0` until written).
    pub id: i64,
    pub clip_id: String,
    /// Whose state it is (the owner that wrote it, e.g. `source_marks`).
    pub kind: String,
    /// What it says, to read in the list.
    pub label: String,
    /// When it was written (UTC, as the database keeps it).
    pub made_at_utc: String,
    /// The state after the edit, as its owner wrote it.
    pub state: Value,
}

/// An entry to write.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NewEntry {
    pub clip_id: String,
    pub kind: String,
    pub label: String,
    pub state: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    /// One entry more in a clip's history.
    Add(NewEntry),
    /// The history of a clip, the oldest first.
    Read { clip_id: String },
}

impl Operation {
    pub fn is_write(&self) -> bool {
        matches!(self, Self::Add(_))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Data {
    Entries(Vec<Entry>),
    Changed,
}

/// Joins the history table to a project database.
pub struct EditHistoryModule;

impl EditHistoryModule {
    pub fn factory() -> Arc<dyn TableModuleFactory> {
        qnc_db_broker::owned_tables::<store::Store>()
    }
}

fn encode(operation: &Operation) -> Result<Value> {
    serde_json::to_value(operation).map_err(|e| e.to_string())
}

/// The history of `clip_id` in the project behind `target`, the oldest first.
pub fn read(target: &ProjectDbTarget, clip_id: &str) -> Result<Vec<Entry>> {
    let mut client = target.open(Access::ReadOnly, vec![EditHistoryModule::factory()])?;
    let answer = client.execute(MODULE_ID, encode(&Operation::Read { clip_id: clip_id.into() })?)?;
    match serde_json::from_value(answer).map_err(|e| e.to_string())? {
        Data::Entries(entries) => Ok(entries),
        Data::Changed => Err(qnc_ui_text::text("db.edit_history_reply").into()),
    }
}

/// The history of the clip one form shows: read in the background and read again every second
/// while it is shown (piece `qnc-clip-follow`), so an edit of another form or computer is there;
/// written through the project's serial writer without waiting.
pub struct ClipHistory {
    follow: qnc_clip_follow::ClipFollow<Vec<Entry>>,
    writer: Option<(String, ProjectDbWriter)>,
    pending: Vec<qnc_db_broker::Pending>,
}

impl Default for ClipHistory {
    fn default() -> Self {
        Self { follow: qnc_clip_follow::ClipFollow::new("edit-history", qnc_clip_follow::EVERY), writer: None, pending: Vec::new() }
    }
}

impl std::fmt::Debug for ClipHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClipHistory").field("follow", &self.follow).finish_non_exhaustive()
    }
}

impl ClipHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// The history of `clip_id`, the oldest first; `None` until it is read.
    pub fn for_clip(&mut self, target: Option<&ProjectDbTarget>, clip_id: Option<&str>) -> Option<Vec<Entry>> {
        self.pending.retain(|pending| match pending.try_take() {
            None => true,
            Some(Ok(_)) => false,
            Some(Err(error)) => {
                log(format!("edit-history not kept: {error}"));
                false
            }
        });
        let (target, clip_id) = (target?, clip_id?);
        self.follow.get(target.uri(), clip_id, self.pending.is_empty(), || {
            let (owned, clip) = (target.clone(), clip_id.to_string());
            move || read(&owned, &clip)
        })
    }

    /// Writes `entry` (through the serial writer); it is in the history at once.
    pub fn add(&mut self, target: &ProjectDbTarget, entry: NewEntry) {
        if !self.writer.as_ref().is_some_and(|(uri, _)| uri == target.uri()) {
            match ProjectDbWriter::spawn(target.clone(), vec![EditHistoryModule::factory()]) {
                Ok(writer) => self.writer = Some((target.uri().to_string(), writer)),
                Err(error) => {
                    log(format!("edit-history not kept: {error}"));
                    return;
                }
            }
        }
        let Some((_, writer)) = &self.writer else { return };
        match encode(&Operation::Add(entry.clone())).and_then(|payload| writer.submit(MODULE_ID, payload)) {
            Ok(pending) => self.pending.push(pending),
            Err(error) => log(format!("edit-history not kept: {error}")),
        }
        let mut entries = self.follow.known(target.uri(), &entry.clip_id).cloned().unwrap_or_default();
        entries.push(Entry { id: 0, clip_id: entry.clip_id.clone(), kind: entry.kind, label: entry.label, made_at_utc: String::new(), state: entry.state });
        self.follow.set(target.uri(), &entry.clip_id, entries);
    }
}

fn log(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
}

#[cfg(test)]
mod tests;
