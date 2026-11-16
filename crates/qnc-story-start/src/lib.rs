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
    /// The story's project is made and active: Ingest Selects the base's card (this source) into
    /// it as any card, then [`StoryStart::mark`] ticks the chosen clips for Uvezi (Link).
    SelectBase(String),
    /// Shown to the user (a failure, the sheet's text).
    Say(String),
}

/// What the check found.
struct Found {
    /// The base Ingest works in: its id and the project it has.
    base: Option<(String, Option<String>)>,
    /// The story offered (its id and name): the base's own, else the one worked on last.
    last: Option<(String, String)>,
    /// The project templates and the one chosen.
    templates: (Vec<(String, String)>, usize),
}

enum Job {
    Checked(Result<Found, String>),
    /// The base has its story (made or gone on with): the base's card and the chosen clips' files.
    Taken(Result<(String, Vec<String>), String>),
    /// A breaking news project is made for this card (its uri), or not.
    Breaking(String, Result<(String, String), String>),
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
    /// The clips the user chose in the Ingest form: the story takes them.
    clips: Vec<String>,
    /// The choice kept until a project is made (user 2026-11-15: "ono što korisnik označi treba
    /// zapisati u privremenu json; taj zapis se koristi kao podatak koji će se zapisati u bazu kad
    /// se kreira projekt; ako se ne kreira projekt, taj privremeni zapis se briše").
    pending: Option<PathBuf>,
    /// The files of the chosen clips on the base's card, ticked once its Select is done.
    marks: Option<Vec<String>>,
    job: Option<Receiver<Job>>,
}

impl Drop for StoryStart {
    fn drop(&mut self) {
        self.forget_choice();
    }
}

/// The temporary file of this form's choice: in the system's temporary folder, one per process.
fn choice_file() -> PathBuf {
    std::env::temp_dir().join(format!("qnc-ai-ingest-choice-{}.json", std::process::id()))
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

    /// The choice written to its temporary file (the base, once known, and the clips).
    fn keep_choice(&mut self) {
        let file = choice_file();
        let json = serde_json::json!({ "base_id": self.base_id, "clips": self.clips });
        match std::fs::write(&file, json.to_string()) {
            Ok(()) => self.pending = Some(file),
            Err(_) => self.pending = None,
        }
    }

    /// No project was made, or it was and took the clips: the temporary choice is gone.
    fn forget_choice(&mut self) {
        if let Some(file) = self.pending.take() {
            let _ = std::fs::remove_file(file);
        }
    }

    /// Uvezi with `clips` chosen: looks whether the base of `root` has its story (the answer comes
    /// in `poll`); a base that has one gives it the clips at once.
    pub fn import_asked(&mut self, root: &Path, clips: Vec<String>) {
        if self.busy() || self.sheet.is_some() || self.offer.is_some() {
            return;
        }
        self.clips = clips.clone();
        self.keep_choice();
        let root = root.to_path_buf();
        self.run(move || {
            Job::Checked(qnc_ingest_base::active(&root).and_then(|base| {
                let Some(base) = base else { return Ok(Found { base: None, last: None, templates: (Vec::new(), 0) }) };
                // A base whose story exists is not given to it silently (user 2026-11-15: "nije mi
                // ponudio upis podataka; od iste baze se mora moći kreirati više projekata"): its
                // story is offered to go on with, or a new one is made; else the last story.
                let last = match &base.project_name {
                    Some(name) => qnc_ingest_base::story_id(&root, name)?.map(|id| (id, name.clone())),
                    None => qnc_ingest_base::last_story(&root, &base.id)?,
                };
                Ok(Found { last, templates: qnc_ingest_base::templates(&root)?, base: Some((base.id, base.project_name)) })
            }))
        });
    }

    /// A card (not a base) chosen in the Ingest form is breaking news (user 2026-11-16: the
    /// application links the source card; "automatski kreiraš brejking news projekt, bez da
    /// novinar bilo što piše"): its project is made and made active off the form's thread, then
    /// `poll` gives [`Next::SelectBase`] with the card, which Ingest Selects into it.
    pub fn breaking_news(&mut self, root: &Path, card: &str) {
        if self.busy() {
            return;
        }
        let (root, card) = (root.to_path_buf(), card.to_string());
        self.run(move || Job::Breaking(card, qnc_breaking_news::make(&root)));
    }

    /// The story the user may go on with, shown before the sheet (its name).
    pub fn offer(&self) -> Option<&str> {
        self.offer.as_ref().map(|(_, name)| name.as_str())
    }

    /// The user goes on with the story offered: the base joins it.
    pub fn go_on(&mut self, root: &Path) {
        let (Some((story_id, _)), Some(base_id)) = (self.offer.take(), self.base_id.clone()) else { return };
        let (root, clips) = (root.to_path_buf(), self.clips.clone());
        self.run(move || {
            Job::Taken(
                qnc_ingest_base::continue_story(&root, &base_id, &story_id)
                    .and_then(|_| qnc_ingest_base::take_clips(&root, &story_id, &base_id, &clips))
                    .and_then(|_| qnc_ingest_base::card_of(&root, &base_id, &clips)),
            )
        });
    }

    /// The user cancels the choice: nothing is taken.
    pub fn cancel(&mut self) {
        (self.offer, self.sheet, self.base_id) = (None, None, None);
        self.forget_choice();
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
            Answer::Cancel => {
                (self.sheet, self.base_id) = (None, None);
                self.forget_choice();
            }
            Answer::Confirm => {
                let (Some(sheet), Some(base_id)) = (self.sheet.as_ref(), self.base_id.clone()) else { return };
                let name = sheet.name.trim().to_string();
                let template = sheet.template_id().unwrap_or_default().to_string();
                let (root, clips): (PathBuf, _) = (root.to_path_buf(), self.clips.clone());
                self.run(move || {
                    Job::Taken(
                        qnc_ingest_base::take_for_story(&root, &base_id, &name, &template)
                            .and_then(|story| qnc_ingest_base::take_clips(&root, &story, &base_id, &clips))
                            .and_then(|_| qnc_ingest_base::card_of(&root, &base_id, &clips)),
                    )
                });
            }
        }
    }

    /// A check or a taking finished.
    pub fn poll(&mut self, root: &Path, confirm_label: &str) -> Option<Next> {
        let job = self.job.as_ref()?.try_recv().ok()?;
        self.job = None;
        match job {
            // No base (a story's project is active): nothing to start. A base, with its story or
            // none: its story (else the last one) is offered, or the sheet of a new one opens.
            Job::Checked(Ok(Found { base: None, .. })) => Some(Next::Import),
            Job::Checked(Ok(Found { base: Some((base_id, _)), last, templates })) => {
                (self.base_id, self.templates, self.confirm_label) = (Some(base_id), templates, confirm_label.to_string());
                self.keep_choice();
                // Ingest opened by "Novi projekt" of the Projects form offers no story to go on with.
                match last.filter(|_| std::env::var_os(qnc_ai_open::NEW_STORY).is_none()) {
                    Some(last) => self.offer = Some(last),
                    None => self.new_story(root),
                }
                None
            }
            Job::Checked(Err(error)) => Some(Next::Say(error)),
            Job::Taken(Ok((card, files))) => {
                (self.sheet, self.base_id, self.marks) = (None, None, Some(files));
                Some(Next::SelectBase(card))
            }
            // Its clips are not ticked: the user chooses them in Ingest.
            Job::Breaking(card, Ok((_, name))) => {
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("breaking news project {name} made for {card}"));
                Some(Next::SelectBase(card))
            }
            Job::Breaking(card, Err(error)) => {
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("breaking news project for {card} not made: {error}"));
                Some(Next::Say(error))
            }
            Job::Taken(Err(error)) => {
                if let Some(sheet) = self.sheet.as_mut() {
                    sheet.message = error.clone();
                }
                Some(Next::Say(error))
            }
        }
    }

    /// The base's card is Selected into the project: its chosen clips are ticked for Uvezi, the
    /// others not, and the temporary choice is gone. False when nothing waits to be ticked.
    pub fn mark(&mut self, clips: &mut [qnc_clip_row::ClipRow]) -> bool {
        let Some(files) = self.marks.take() else { return false };
        for clip in clips.iter_mut() {
            clip.selected = files.contains(&clip.name);
        }
        self.forget_choice();
        true
    }

    /// The sheet shown now, for the form to draw.
    pub fn sheet_mut(&mut self) -> Option<&mut SheetOverlay> {
        self.sheet.as_mut()
    }

    pub fn busy(&self) -> bool {
        self.job.is_some()
    }
}

