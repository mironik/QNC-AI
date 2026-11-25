//! The history of a project (user 2026-11-25: the journal is "a quasi git that remembers every
//! action"; "the project is the user's category, nobody may delete but the user"). One thing:
//! the window of the project's moments, newest first, each with when, who, which piece and what,
//! and going back to the moment right after one of them (piece `qnc-project-restore`). Only the
//! QNC user who made the project may go back; a project made before its owner was kept may be
//! taken back by anyone. Nothing is deleted: a going back is itself a moment, and the moments
//! after the one gone back to stay in the list, a branch that can be gone back to again.
//! Everything is read and done in the background; no paint here (piece `qnc-project-history-view`).

mod read;

use std::sync::mpsc::{channel, Receiver};

use qnc_db_broker::ProjectDbTarget;
use serde::{Deserialize, Serialize};

pub use read::MODULE_ID;

pub type Result<T> = std::result::Result<T, String>;

/// The action that opens and closes the window (Ctrl+Shift+H) and the one that goes back.
pub const OPEN: &str = "project_history_open";
pub const RESTORE: &str = "project_history_restore";
/// The keys of the window while it is open: its own scope of the catalog, the same in every form
/// (user 2026-11-25: Esc closes it, Enter goes back, arrows light a row).
pub const KEY_SCOPE: &str = "project_history";

/// One moment of the project: a commit of its journal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Moment {
    pub id: i64,
    /// When, as this computer keeps local time.
    pub at: String,
    pub author: String,
    pub module: String,
    pub action: String,
    /// The moment an undo, a redo or a going back refers to.
    pub refers: Option<i64>,
}

/// What the window shows.
#[derive(Debug, Clone, PartialEq)]
pub struct HistoryView {
    /// The rows, newest first; the last is the start of the journal.
    pub rows: Vec<String>,
    pub lit: usize,
    /// Who owns the project, as a line to read.
    pub owner: String,
    /// The user here may go back.
    pub may: bool,
    pub message: String,
    pub working: bool,
}

type Read = Result<(ProjectDbTarget, Vec<Moment>, Option<String>)>;

#[derive(Default)]
pub struct ProjectHistory {
    open: bool,
    target: Option<ProjectDbTarget>,
    moments: Vec<Moment>,
    owner: Option<String>,
    lit: usize,
    message: String,
    reading: Option<Receiver<Read>>,
    restoring: Option<Receiver<Result<u64>>>,
    /// The project the history is of, given (none: the active project, found again on each open).
    fixed: Option<ProjectDbTarget>,
}

impl ProjectHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// The history of the project behind `target`, not the active one.
    pub fn of(target: ProjectDbTarget) -> Self {
        Self { fixed: Some(target), ..Self::default() }
    }

    /// An action of the form while the window is open, or the one that opens it; whether it was
    /// the window's (while open it takes every key: nothing reaches the form behind it).
    pub fn key(&mut self, action_id: &str, value: Option<i64>) -> bool {
        if action_id == OPEN {
            if self.open {
                self.open = false;
            } else {
                self.open = true;
                self.lit = 0;
                self.message.clear();
                self.target = self.fixed.clone();
                self.read();
            }
            return true;
        }
        if !self.open {
            return false;
        }
        let last = self.moments.len();
        match action_id {
            "step_prev_part" | "panel_up" | "step_prev_segment" => self.lit = self.lit.saturating_sub(1),
            "step_next_part" | "panel_down" | "step_next_segment" => self.lit = (self.lit + 1).min(last),
            "page_prev" => self.lit = self.lit.saturating_sub(10),
            "page_next" => self.lit = (self.lit + 10).min(last),
            "clear_focus" => self.open = false,
            "activate_focused_item" => self.restore(self.lit),
            RESTORE => {
                if let Some(row) = value.and_then(|row| usize::try_from(row).ok()) {
                    self.lit = row.min(last);
                }
                self.restore(self.lit);
            }
            "project_history_light" => {
                if let Some(row) = value.and_then(|row| usize::try_from(row).ok()) {
                    self.lit = row.min(last);
                }
            }
            _ => {}
        }
        true
    }

    /// What came back from the background; whether the view changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        if let Some(read) = self.reading.as_ref().and_then(|reading| reading.try_recv().ok()) {
            self.reading = None;
            changed = true;
            match read {
                Ok((target, moments, owner)) => {
                    self.target = Some(target);
                    self.moments = moments;
                    self.owner = owner;
                    self.lit = self.lit.min(self.moments.len());
                }
                Err(error) => self.message = error,
            }
        }
        if let Some(done) = self.restoring.as_ref().and_then(|restoring| restoring.try_recv().ok()) {
            self.restoring = None;
            changed = true;
            match done {
                Ok(taken) => {
                    self.message = qnc_ui_text::fill("history.restored", &[("count", &taken.to_string())]);
                    self.lit = 0;
                    self.read();
                }
                Err(error) => self.message = error,
            }
        }
        changed
    }

    /// Reading or going back.
    pub fn busy(&self) -> bool {
        self.reading.is_some() || self.restoring.is_some()
    }

    /// Whether the window is open (its keys are then the only keys of the form).
    pub fn is_open(&self) -> bool {
        self.open
    }

    /// The window, while it is open.
    pub fn view(&self) -> Option<HistoryView> {
        self.open.then(|| HistoryView {
            rows: self.moments.iter().map(row).chain([qnc_ui_text::text("history.start").to_string()]).collect(),
            lit: self.lit,
            owner: match &self.owner {
                Some(owner) => qnc_ui_text::fill("history.owner", &[("user", owner)]),
                None => qnc_ui_text::text("history.no_owner").to_string(),
            },
            may: self.may(),
            message: match self.restoring.is_some() {
                true => qnc_ui_text::text("history.restoring").to_string(),
                false => self.message.clone(),
            },
            working: self.busy(),
        })
    }

    /// The QNC user here owns the project, or it has no owner kept.
    fn may(&self) -> bool {
        let Some(owner) = &self.owner else { return true };
        qnc_user_settings::user_name().is_ok_and(|user| user.eq_ignore_ascii_case(owner))
    }

    /// The active project of this QNC (from its database, as every piece finds it), its moments and
    /// its owner.
    fn read(&mut self) {
        let known = self.target.clone();
        let (send, receive) = channel();
        std::thread::spawn(move || {
            let read = (|| -> Read {
                let target = match known {
                    Some(target) => target,
                    None => active_project()?,
                };
                let read::Data::Moments(moments) = read::ask(&target, &read::Operation::Moments { limit: read::MOMENTS })? else { return Err(qnc_ui_text::text("history.reply").into()) };
                let read::Data::Owner(owner) = read::ask(&target, &read::Operation::Owner)? else { return Err(qnc_ui_text::text("history.reply").into()) };
                Ok((target, moments, owner))
            })();
            let _ = send.send(read);
        });
        self.reading = Some(receive);
    }

    /// Goes back to the moment of the row `at` (the last row: the start of the journal).
    fn restore(&mut self, at: usize) {
        if self.busy() {
            return;
        }
        if !self.may() {
            self.message = qnc_ui_text::fill("history.not_owner", &[("user", self.owner.as_deref().unwrap_or(""))]);
            return;
        }
        let Some(target) = self.target.clone() else { return };
        if at == 0 && !self.moments.is_empty() {
            self.message = qnc_ui_text::text("history.already_here").to_string();
            return;
        }
        let to = self.moments.get(at).map_or(0, |moment| moment.id);
        let (send, receive) = channel();
        std::thread::spawn(move || {
            let done = (|| -> Result<u64> {
                let read::Data::Modules(modules) = read::ask(&target, &read::Operation::ModulesAfter { to })? else { return Err(qnc_ui_text::text("history.reply").into()) };
                qnc_project_restore::restore(&target, to, &modules)
            })();
            let _ = send.send(done);
        });
        self.restoring = Some(receive);
    }
}

/// The project database of the active project of this QNC (its root, its registry, its settings).
fn active_project() -> Result<ProjectDbTarget> {
    let root = qnc_dev_diagnostics::resolve_qnc_root().ok_or_else(|| qnc_ui_text::text("history.no_project").to_string())?;
    let reader = qnc_work_settings::SettingsReader::from_root(&root).map_err(|error| error.to_string())?;
    let settings = reader.read().map_err(|_| qnc_ui_text::text("history.no_project").to_string())?;
    ProjectDbTarget::for_project(&reader, &settings)
}

/// A row of the list: when, who, which piece and what it did.
fn row(moment: &Moment) -> String {
    let action = match (moment.action.as_str(), moment.refers) {
        ("restore", Some(to)) => qnc_ui_text::fill("history.went_back", &[("id", &to.to_string())]),
        ("undo", Some(id)) => qnc_ui_text::fill("history.undid", &[("id", &id.to_string())]),
        ("redo", Some(id)) => qnc_ui_text::fill("history.redid", &[("id", &id.to_string())]),
        (action, _) => action.replace('_', " "),
    };
    format!("#{}  {}  {}  {}  {}", moment.id, moment.at, moment.author, piece(&moment.module), action)
}

/// The name of a piece to read.
fn piece(module: &str) -> &str {
    match module {
        "qnc.module.program-db" => qnc_ui_text::text("journal.piece.story"),
        "qnc.module.script-db" => qnc_ui_text::text("journal.piece.script"),
        "qnc.module.virtual-shots" => qnc_ui_text::text("journal.piece.statements"),
        "qnc.module.clip-sound" => qnc_ui_text::text("journal.piece.clip_sound"),
        "qnc.module.skeleton-db" => qnc_ui_text::text("journal.piece.s_story"),
        "qnc.module.newsroom-sheet-db" => qnc_ui_text::text("journal.piece.sheet"),
        "qnc.module.clip-marks" => qnc_ui_text::text("journal.piece.clip_marks"),
        other => other,
    }
}

#[cfg(test)]
mod tests;

impl std::fmt::Debug for ProjectHistory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ProjectHistory").field("open", &self.open).field("moments", &self.moments.len()).finish_non_exhaustive()
    }
}
