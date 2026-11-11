//! The story of an Ingest base started (user 2026-11-11: "nakon Selektiraj sve kada klikne UVEZI
//! mora se otvoriti obrazac za kreiranje projekta; kad se kreira projekt naziv projekta se dodaje
//! u ingest bazu"; 2026-11-12: "ako projekt postoji onda korisnik može nastaviti dalje ili
//! kreirati novi projekt"). When the base Ingest worked in has no project yet: if a story was
//! worked on before, the user first chooses to go on with it (the base joins it) or to make a new
//! one; a new one is QNC's new project sheet (`qnc-newsroom-sheet-model`, with the template);
//! Create makes the story's project, which takes the base (`qnc-ingest-base`). A base that has
//! its project goes on at once. Everything that reads or writes runs off the form's thread. It
//! draws nothing: the form shows `offer()` or `sheet_mut()` and sends the answer back.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};

use qnc_newsroom_sheet_model::{Answer, SheetOverlay};

/// What the owner does after a poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Next {
    /// The base has its story: what comes next (AI Media Assist) opens.
    Import,
    /// Shown to the user (a failure, the sheet's text).
    Say(String),
}

/// What the check found.
struct Found {
    /// The base Ingest works in: its id and the project it has.
    base: Option<(String, Option<String>)>,
    /// The story worked on last (its id and name), when the base has none.
    last: Option<(String, String)>,
    /// The project templates and the one chosen.
    templates: (Vec<(String, String)>, usize),
}

enum Job {
    Checked(Result<Found, String>),
    /// The base has its story (made or gone on with).
    Taken(Result<String, String>),
}

/// The start of a story.
#[derive(Default)]
pub struct StoryStart {
    sheet: Option<SheetOverlay>,
    base_id: Option<String>,
    /// The story the user may go on with (its id and name), asked before the sheet.
    offer: Option<(String, String)>,
    templates: (Vec<(String, String)>, usize),
    confirm_label: String,
    job: Option<Receiver<Job>>,
}

impl std::fmt::Debug for StoryStart {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_struct("StoryStart").field("sheet", &self.sheet.is_some()).field("base_id", &self.base_id).field("offer", &self.offer).field("busy", &self.job.is_some()).finish()
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

    /// Looks whether the base of `root` has its story (the answer comes in `poll`).
    pub fn import_asked(&mut self, root: &Path) {
        if self.busy() || self.sheet.is_some() || self.offer.is_some() {
            return;
        }
        let root = root.to_path_buf();
        self.run(move || {
            Job::Checked(qnc_ingest_base::active(&root).and_then(|base| {
                let base = base.map(|base| (base.id, base.project_name));
                let Some((id, None)) = &base else { return Ok(Found { base, last: None, templates: (Vec::new(), 0) }) };
                let last = qnc_ingest_base::last_story(&root, id)?;
                Ok(Found { last, templates: qnc_ingest_base::templates(&root)?, base })
            }))
        });
    }

    /// The story the user may go on with, shown before the sheet (its name).
    pub fn offer(&self) -> Option<&str> {
        self.offer.as_ref().map(|(_, name)| name.as_str())
    }

    /// The user goes on with the story offered: the base joins it.
    pub fn go_on(&mut self, root: &Path) {
        let (Some((story_id, _)), Some(base_id)) = (self.offer.take(), self.base_id.clone()) else { return };
        let root = root.to_path_buf();
        self.run(move || Job::Taken(qnc_ingest_base::continue_story(&root, &base_id, &story_id)));
    }

    /// The user makes a new story: the sheet opens.
    pub fn new_story(&mut self, root: &Path) {
        self.offer = None;
        let (templates, chosen) = self.templates.clone();
        let sheet = SheetOverlay::new(root, String::new(), true, Default::default(), None, self.confirm_label.clone());
        self.sheet = Some(sheet.with_templates(templates, chosen));
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
                self.run(move || Job::Taken(qnc_ingest_base::take_for_story(&root, &base_id, &name, &template).map(|_| name)));
            }
        }
    }

    /// A check or a taking finished.
    pub fn poll(&mut self, root: &Path, confirm_label: &str) -> Option<Next> {
        let job = self.job.as_ref()?.try_recv().ok()?;
        self.job = None;
        match job {
            // A base with its story, or no base (a story's project is active): nothing to start.
            Job::Checked(Ok(Found { base: Some((_, Some(_))) | None, .. })) => Some(Next::Import),
            Job::Checked(Ok(Found { base: Some((base_id, None)), last, templates })) => {
                (self.base_id, self.templates, self.confirm_label) = (Some(base_id), templates, confirm_label.to_string());
                match last {
                    Some(last) => self.offer = Some(last),
                    None => self.new_story(root),
                }
                None
            }
            Job::Checked(Err(error)) => Some(Next::Say(error)),
            Job::Taken(Ok(_name)) => {
                (self.sheet, self.base_id) = (None, None);
                Some(Next::Import)
            }
            Job::Taken(Err(error)) => {
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
