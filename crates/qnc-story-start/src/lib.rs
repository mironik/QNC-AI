//! Uvezi of an Ingest base starts the story (user 2026-11-11: "nakon Selektiraj sve kada klikne
//! UVEZI mora se otvoriti overlay obrazac za kreiranje projekta; kad se kreira projekt naziv
//! projekta se dodaje u ingest bazu"). When the base Ingest works in has no project yet, Uvezi
//! opens the new project's sheet (the story sheet of QNC's Project, `qnc-newsroom-sheet-model`);
//! Create makes the story's project, which takes the base (`qnc-ingest-base`), and then the
//! import goes on; Cancel closes the sheet and imports nothing. A base that has its project
//! imports at once. Everything that reads or writes runs off the form's thread. It draws
//! nothing: the form shows `sheet_mut()` and sends the answer back.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

use qnc_newsroom_sheet_model::{Answer, SheetOverlay};

/// What the owner does after a poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// The base has its story now: the import goes on.
    Import,
    /// Shown to the user (a failure, the sheet's text).
    Say(String),
}

/// The base Ingest works in (its id and the project it has) and the project templates.
type Checked = (Option<(String, Option<String>)>, (Vec<(String, String)>, usize));

enum Job {
    /// Which base Ingest works in, the project it has, and the templates a project is made with.
    Checked(Result<Checked, String>),
    /// The story's project made (its name).
    Created(Result<String, String>),
}

/// The start of a story from Uvezi.
#[derive(Default)]
pub struct StoryStart {
    sheet: Option<SheetOverlay>,
    base_id: Option<String>,
    job: Option<Receiver<Job>>,
}

impl std::fmt::Debug for StoryStart {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("StoryStart").field("sheet", &self.sheet.is_some()).field("base_id", &self.base_id).field("busy", &self.job.is_some()).finish()
    }
}

impl StoryStart {
    fn run(&mut self, work: impl FnOnce() -> Job + Send + 'static) {
        let (send, receive) = channel();
        std::thread::spawn(move || {
            let _ = send.send(work());
        });
        self.job = Some(receive);
    }

    /// Uvezi: looks whether the base of `root` has its story (the answer comes in `poll`).
    pub fn import_asked(&mut self, root: &Path) {
        if self.busy() || self.sheet.is_some() {
            return;
        }
        let root = root.to_path_buf();
        self.run(move || {
            Job::Checked(qnc_ingest_base::active(&root).and_then(|base| {
                let base = base.map(|base| (base.id, base.project_name));
                // The templates only matter when the sheet opens.
                let templates = if matches!(base, Some((_, None))) { qnc_ingest_base::templates(&root)? } else { (Vec::new(), 0) };
                Ok((base, templates))
            }))
        });
    }

    /// The sheet's answer.
    pub fn answer(&mut self, root: &Path, answer: Answer) {
        match answer {
            Answer::Cancel => (self.sheet, self.base_id) = (None, None),
            Answer::Confirm => {
                let (Some(sheet), Some(base_id)) = (self.sheet.as_ref(), self.base_id.clone()) else { return };
                let name = sheet.name.trim().to_string();
                let template = sheet.template_id().unwrap_or_default().to_string();
                let root: PathBuf = root.to_path_buf();
                self.run(move || Job::Created(qnc_ingest_base::take_for_story(&root, &base_id, &name, &template).map(|_| name)));
            }
        }
    }

    /// A check or a creation finished.
    pub fn poll(&mut self, root: &Path, confirm_label: &str) -> Option<Next> {
        let job = self.job.as_ref()?.try_recv().ok()?;
        self.job = None;
        match job {
            Job::Checked(Ok((Some((_, Some(_))), _))) => Some(Next::Import),
            Job::Checked(Ok((Some((base_id, None)), (templates, chosen)))) => {
                self.base_id = Some(base_id);
                let sheet = SheetOverlay::new(root, String::new(), true, Default::default(), None, confirm_label.to_string());
                self.sheet = Some(sheet.with_templates(templates, chosen));
                None
            }
            // The active project is no Ingest base (a story's): there is no story to start.
            Job::Checked(Ok((None, _))) => Some(Next::Import),
            Job::Checked(Err(error)) => Some(Next::Say(error)),
            Job::Created(Ok(_name)) => {
                (self.sheet, self.base_id) = (None, None);
                Some(Next::Import)
            }
            Job::Created(Err(error)) => {
                if let Some(sheet) = self.sheet.as_mut() {
                    sheet.message = error.clone();
                }
                Some(Next::Say(error))
            }
        }
    }

    /// The sheet shown now, for the form to draw.
    pub fn sheet_mut(&mut self) -> Option<&mut SheetOverlay> {
        self.sheet.as_mut()
    }

    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
}
