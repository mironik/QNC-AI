//! The IN and OUT of each clip as last saved on the source timeline, and their history (user
//! 2026-11-25: "the zoom gets a history, a drop-down in its header bar; the user can go back to any
//! position of it"; "it is kept per clip, not together"; the journal of the project is "a quasi
//! git that remembers every action"). A Save (Enter) that moved IN or OUT writes the clip's row;
//! the row is an edit, so the project's journal keeps every write of it (who, when, before and
//! after). The history of a clip is read from that journal, not kept here: there is no second
//! history. Going back to an entry is a write like any other, so nothing is lost.
//!
//! A table module of the project database intermediary (`qnc-db-broker`): it owns only its table,
//! is written only through that intermediary and knows no form. The source timeline does not take
//! its marks from here; the row is what was saved, the journal what happened to it.

mod store;

use std::sync::Arc;

use qnc_db_broker::{Access, ProjectDbTarget, ProjectDbWriter, TableModuleFactory};
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const MODULE_ID: &str = "qnc.module.clip-marks";

pub type Result<T> = std::result::Result<T, String>;

/// One position of a clip's history: a write of its IN and OUT, from the journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Unique in the clip's history (`0` until written).
    pub id: i64,
    pub clip_id: String,
    pub in_frame: u64,
    pub out_frame: u64,
    /// A going back to an earlier position.
    pub back: bool,
    /// Who wrote it (the user and the computer).
    pub author: String,
    /// When it was written (UTC, as the database keeps it).
    pub made_at_utc: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case", deny_unknown_fields)]
pub enum Operation {
    /// A Save that moved IN or OUT: `from` where they were (kept as the first position of a clip
    /// with none), `to` where they are.
    Keep { clip_id: String, from: (u64, u64), to: (u64, u64) },
    /// Going back to an earlier position of the history.
    Back { clip_id: String, to: (u64, u64) },
    /// The history of a clip, the oldest first.
    History { clip_id: String },
}

impl Operation {
    pub fn is_write(&self) -> bool {
        !matches!(self, Self::History { .. })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
pub enum Data {
    Entries(Vec<Entry>),
    Changed,
}

/// Joins the marks table to a project database.
pub struct ClipMarksModule;

impl ClipMarksModule {
    pub fn factory() -> Arc<dyn TableModuleFactory> {
        qnc_db_broker::owned_tables::<store::Store>()
    }
}

fn encode(operation: &Operation) -> Result<Value> {
    serde_json::to_value(operation).map_err(|e| e.to_string())
}

/// The history of `clip_id` in the project behind `target`, the oldest first.
pub fn read(target: &ProjectDbTarget, clip_id: &str) -> Result<Vec<Entry>> {
    let mut client = target.open(Access::ReadOnly, vec![ClipMarksModule::factory()])?;
    let answer = client.execute(MODULE_ID, encode(&Operation::History { clip_id: clip_id.into() })?)?;
    match serde_json::from_value(answer).map_err(|e| e.to_string())? {
        Data::Entries(entries) => Ok(entries),
        Data::Changed => Err(qnc_ui_text::text("db.clip_marks_reply").into()),
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
        Self { follow: qnc_clip_follow::ClipFollow::new("clip-marks", qnc_clip_follow::EVERY), writer: None, pending: Vec::new() }
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
                log(format!("clip-marks not kept: {error}"));
                false
            }
        });
        let (target, clip_id) = (target?, clip_id?);
        self.follow.get(target.uri(), clip_id, self.pending.is_empty(), || {
            let (owned, clip) = (target.clone(), clip_id.to_string());
            move || read(&owned, &clip)
        })
    }

    /// A Save that moved IN or OUT from `from` to `to` (written through the serial writer; it is
    /// in the history at once).
    pub fn keep(&mut self, target: &ProjectDbTarget, clip_id: &str, from: (u64, u64), to: (u64, u64)) {
        let first = self.follow.known(target.uri(), clip_id).is_none_or(Vec::is_empty);
        self.write(target, Operation::Keep { clip_id: clip_id.into(), from, to });
        let mut entries = self.follow.known(target.uri(), clip_id).cloned().unwrap_or_default();
        if first {
            entries.push(unwritten(clip_id, from, false));
        }
        entries.push(unwritten(clip_id, to, false));
        self.follow.set(target.uri(), clip_id, entries);
    }

    /// Going back to the position `to`.
    pub fn back(&mut self, target: &ProjectDbTarget, clip_id: &str, to: (u64, u64)) {
        self.write(target, Operation::Back { clip_id: clip_id.into(), to });
        let mut entries = self.follow.known(target.uri(), clip_id).cloned().unwrap_or_default();
        entries.push(unwritten(clip_id, to, true));
        self.follow.set(target.uri(), clip_id, entries);
    }

    fn write(&mut self, target: &ProjectDbTarget, operation: Operation) {
        if !self.writer.as_ref().is_some_and(|(uri, _)| uri == target.uri()) {
            match ProjectDbWriter::spawn(target.clone(), vec![ClipMarksModule::factory()]) {
                Ok(writer) => self.writer = Some((target.uri().to_string(), writer)),
                Err(error) => {
                    log(format!("clip-marks not kept: {error}"));
                    return;
                }
            }
        }
        let Some((_, writer)) = &self.writer else { return };
        match encode(&operation).and_then(|payload| writer.submit(MODULE_ID, payload)) {
            Ok(pending) => self.pending.push(pending),
            Err(error) => log(format!("clip-marks not kept: {error}")),
        }
    }
}

fn unwritten(clip_id: &str, (in_frame, out_frame): (u64, u64), back: bool) -> Entry {
    Entry { id: 0, clip_id: clip_id.into(), in_frame, out_frame, back, author: String::new(), made_at_utc: String::new() }
}

fn log(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
}

#[cfg(test)]
mod tests;
