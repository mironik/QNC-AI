//! QNC-AI: Uvezi starts the story (piece `qnc-story-start`): a base without its project shows the
//! new project's sheet first; Create makes the project, which takes the base, then the import
//! goes on.

use crate::*;
use qnc_story_start::Next;

impl IngestApplication {
    /// Uvezi asked: the story is looked at first.
    pub(crate) fn import_asked(&mut self) -> IngestDispatchResult {
        // A project that is no Ingest base (a story's or QNC's) imports as it always did.
        let Some(root) = self.root.clone().filter(|root| qnc_ingest_base::active_is_base(root)) else { return self.start_import() };
        self.story.import_asked(&root);
        IngestDispatchResult::accepted(None, true)
    }

    pub(crate) fn story_sheet_answer(&mut self, confirm: bool) -> IngestDispatchResult {
        let Some(root) = self.root.clone() else { return IngestDispatchResult::accepted(None, false) };
        let answer = if confirm { qnc_newsroom_sheet_model::Answer::Confirm } else { qnc_newsroom_sheet_model::Answer::Cancel };
        self.story.answer(&root, answer);
        IngestDispatchResult::accepted(None, true)
    }

    /// A check or a creation finished: the import goes on, or the user is told.
    pub(crate) fn poll_story(&mut self) -> bool {
        let Some(root) = self.root.clone() else { return false };
        match self.story.poll(&root, qnc_ui_text::text("project.create_project")) {
            Some(Next::Import) => {
                self.start_import();
                true
            }
            Some(Next::Say(message)) => {
                self.view.message = message;
                true
            }
            None => self.story.busy(),
        }
    }

    /// The new project's sheet shown now, for the form to draw.
    pub fn story_sheet_mut(&mut self) -> Option<&mut qnc_newsroom_sheet_model::SheetOverlay> {
        self.story.sheet_mut()
    }
}
