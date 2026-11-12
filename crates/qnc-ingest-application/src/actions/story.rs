//! QNC-AI (user 2026-11-12: "ingest formi je uvijek izvor globalna baza"; "obrazac Novi projekt
//! tretiraj kao overlay prozor Ingest forme"; "projekt koristi dvije baze: globalnu ingest bazu i
//! lokalnu projekt bazu"). The clips are already in the global Ingest base (the background ingest
//! the tray starts). Uvezi of the form gives the chosen clips to a story (piece `qnc-story-start`):
//! a story worked on before may go on, or a new one is made over the form; then the host opens
//! AI Media Assist and closes Ingest. A project that is no base imports as it always did.

use crate::*;
use qnc_story_start::Next;

impl IngestApplication {
    /// Uvezi asked.
    pub(crate) fn import_asked(&mut self) -> IngestDispatchResult {
        let Some(root) = self.root.clone().filter(|root| qnc_ingest_base::active_is_base(root)) else { return self.start_import() };
        let clips: Vec<String> = self.view.clips.iter().filter(|clip| clip.selected).map(|clip| clip.clip_id.clone()).collect();
        if clips.is_empty() {
            return IngestDispatchResult::rejected(qnc_ui_text::text("ingest.msg.no_clips_selected"));
        }
        self.story.import_asked(&root, clips);
        IngestDispatchResult::accepted(None, true)
    }

    /// The background ingest of the cards: the selected clips are imported into the global base by
    /// the project's settings (link), with no story.
    pub fn import_into_base(&mut self) -> IngestDispatchResult {
        self.start_import()
    }

    /// The user's answer over the form: go on, new, cancel, or the sheet's.
    pub fn story_answer(&mut self, answer: StoryAnswer) {
        let Some(root) = self.root.clone() else { return };
        match answer {
            StoryAnswer::GoOn => self.story.go_on(&root),
            StoryAnswer::New => self.story.new_story(&root),
            StoryAnswer::Cancel => self.story.cancel(),
            StoryAnswer::Sheet(answer) => self.story.answer(&root, answer),
        }
    }

    /// A check or a taking finished: the story has the clips, the host goes on.
    pub(crate) fn poll_story(&mut self) -> bool {
        let Some(root) = self.root.clone() else { return false };
        match self.story.poll(&root, qnc_ui_text::text("project.create_project")) {
            Some(Next::Import) => {
                self.request_navigation_after_import();
                true
            }
            Some(Next::Say(message)) => {
                self.view.message = message;
                true
            }
            None => self.story.busy(),
        }
    }

    /// The story offered to go on with (its name), shown over the form.
    pub fn story_offer(&self) -> Option<&str> {
        self.story.offer()
    }

    /// The new project's sheet shown now, for the form to draw.
    pub fn story_sheet_mut(&mut self) -> Option<&mut qnc_newsroom_sheet_model::SheetOverlay> {
        self.story.sheet_mut()
    }
}

/// What the user answered over the form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StoryAnswer {
    GoOn,
    New,
    Cancel,
    Sheet(qnc_newsroom_sheet_model::Answer),
}
