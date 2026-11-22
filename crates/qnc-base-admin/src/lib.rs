//! The administration of the Ingest bases (user 2026-11-16: "Administracija bazama će biti
//! administratorska forma, kako inače i jeste u pravim ingestima"; it left the tray). Its list:
//! every base with what it holds and whether this user may delete it; the delete of a base off the
//! form's thread, as the tray did it: the base says it closes (its window and worker let it go),
//! is closed as the active project when it is one, then forgotten with its folder. Only who made a
//! base, or the administrator, deletes it (`qnc-ingest-base` `may_delete`).

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::time::Duration;

pub use qnc_ingest_base::{IngestBase, Refusal};
pub use qnc_work_progress::Progress;

/// One base of the list.
#[derive(Debug, Clone)]
pub struct Row {
    pub base: IngestBase,
    /// What it holds and what is done; `None` when its database cannot be read.
    pub progress: Option<Progress>,
    /// Whether this user may delete it, and why not.
    pub may_delete: Result<(), Refusal>,
}

/// Every Ingest base, read now.
pub fn read(root: &Path) -> Result<Vec<Row>, String> {
    Ok(qnc_ingest_base::all(root)?
        .into_iter()
        .map(|base| {
            let progress = qnc_work_progress::progress_of(root, &base.id).ok();
            let may_delete = qnc_ingest_base::may_delete(root, &base.id);
            Row { base, progress, may_delete }
        })
        .collect())
}

/// The list read on a thread of its own, every few seconds while it lives.
pub struct Reader {
    read: Receiver<Result<Vec<Row>, String>>,
    last: Option<Result<Vec<Row>, String>>,
}

impl Reader {
    pub fn start(root: &Path) -> Self {
        let (send, read) = channel();
        let root = root.to_path_buf();
        let _ = std::thread::Builder::new().name("qnc-base-admin".into()).spawn(move || loop {
            if send.send(self::read(&root)).is_err() {
                return;
            }
            std::thread::sleep(Duration::from_secs(5));
        });
        Self { read, last: None }
    }

    /// What was read last; true when something new came.
    pub fn poll(&mut self) -> bool {
        let mut new = false;
        while let Ok(read) = self.read.try_recv() {
            self.last = Some(read);
            new = true;
        }
        new
    }

    pub fn last(&self) -> Option<&Result<Vec<Row>, String>> {
        self.last.as_ref()
    }
}

/// A base being deleted, off the form's thread.
pub struct Delete {
    pub label: String,
    done: Receiver<Result<(), String>>,
}

impl Delete {
    /// Starts the delete of `base` (asked and allowed before).
    pub fn start(root: &Path, base: &IngestBase) -> Self {
        let (send, done) = channel();
        let (root, base): (PathBuf, IngestBase) = (root.to_path_buf(), base.clone());
        let label = base.label().to_string();
        let _ = std::thread::Builder::new().name("qnc-base-delete".into()).spawn(move || {
            let _ = send.send(delete(&root, &base));
        });
        Self { label, done }
    }

    /// The result once it is done.
    pub fn poll(&self) -> Option<Result<(), String>> {
        self.done.try_recv().ok()
    }
}

fn delete(root: &Path, base: &IngestBase) -> Result<(), String> {
    // A base deleted meanwhile (its ingest window deleted it the same moment, 2026-11-22: the
    // administration then said "NoOwner") is deleted: nothing to refuse.
    if qnc_ingest_base::by_id(root, &base.id).is_err_and(|error| error.starts_with("no Ingest base")) {
        log(format!("base {} was deleted already", base.label()));
        return Ok(());
    }
    qnc_ingest_base::may_delete(root, &base.id).map_err(|refusal| format!("{refusal:?}"))?;
    // Its window ends and its worker stops once the base says it closes.
    if let Err(error) = qnc_ingest_base::mark_closing(root, &base.id) {
        log(format!("base not marked closing {}: {error}", base.label()));
    }
    // Closed as the active project first: its worker lets the database go within a second.
    if base.active {
        if let Err(error) = qnc_project_close::CloseProjectComponent::from_root(root).close_active_project() {
            log(format!("base not closed {}: {error}", base.label()));
        }
    }
    // A worker in the middle of an AI step ends it first (seconds).
    let mut result = Err(String::new());
    for _ in 0..60 {
        result = qnc_ingest_base::forget(root, &base.id);
        if result.is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_secs(1));
    }
    log(match &result {
        Ok(()) => format!("base deleted {}", base.label()),
        Err(error) => format!("base not deleted {}: {error}", base.label()),
    });
    result
}

fn log(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("base-admin {line}"));
}
