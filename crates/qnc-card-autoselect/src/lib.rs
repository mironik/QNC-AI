//! Ingest goes to the cards of the story by itself (user 2026-11-11: "nastavi s automatskim odabirom
//! kartice u Ingestu"). It does what the user would do in Ingest's browser, through Ingest's own
//! actions: the computer's disks, the card's disk (by its volume serial), Select; then the next card
//! of the story, when Ingest has finished with the one before. It reads only what Ingest shows (its
//! view) and gives back the next action; Ingest does the work. A step Ingest refuses is tried again
//! a little later; a card that is not among the disks is skipped with a reason.

use std::time::{Duration, Instant};

use qnc_ingest_application::{action_ids, IngestIntent, IngestPayload, IngestViewModel};

/// How long a step waits before it is tried again (Ingest still loading its project).
const AGAIN: Duration = Duration::from_millis(500);
/// How long a card is looked for among the disks before it is skipped.
const FIND: Duration = Duration::from_secs(10);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    /// Ingest lists the computer's disks.
    Disks,
    /// The card's disk is opened.
    Open { since: Instant },
    /// Select of the card's disk.
    Select,
    /// Select runs; the next card waits for its end.
    Selecting,
}

/// The cards still to go to, by volume serial, and the step of the first.
#[derive(Debug)]
pub struct AutoSelect {
    cards: Vec<String>,
    step: Step,
    sent: Option<Instant>,
    /// A card skipped and why, for the caller to show.
    skipped: Vec<(String, String)>,
}

impl AutoSelect {
    /// Goes to the cards of these volume serials, in this order.
    pub fn new(cards: Vec<String>) -> Self {
        Self { cards, step: Step::Disks, sent: None, skipped: Vec::new() }
    }

    /// Whether it has cards still to go to.
    pub fn busy(&self) -> bool {
        !self.cards.is_empty()
    }

    /// The cards skipped since the last ask, and why.
    pub fn take_skipped(&mut self) -> Vec<(String, String)> {
        std::mem::take(&mut self.skipped)
    }

    /// The next action for Ingest from what it shows now, when there is one.
    pub fn next(&mut self, view: &IngestViewModel) -> Option<IngestIntent> {
        let serial = self.cards.first()?.clone();
        if self.sent.is_some_and(|at| at.elapsed() < AGAIN) || view.browser.busy {
            return None;
        }
        let browser = &view.browser;
        match self.step.clone() {
            Step::Disks => {
                self.sent = Some(Instant::now());
                if browser.roots && !browser.entries.is_empty() {
                    self.step = Step::Open { since: Instant::now() };
                    return self.next(view);
                }
                Some(IngestIntent::empty(action_ids::INGEST_SOURCE_KIND_LOCAL))
            }
            Step::Open { since } => {
                if !browser.roots && browser.current_uri.as_deref().is_some_and(|uri| uri.contains(&serial)) {
                    self.step = Step::Select;
                    return self.next(view);
                }
                let entry = browser.entries.iter().find(|entry| entry.serial_number.eq_ignore_ascii_case(&serial) || entry.qnc_uri.contains(&serial));
                match entry {
                    Some(entry) => {
                        self.sent = Some(Instant::now());
                        Some(IngestIntent::new(action_ids::INGEST_DIR_OPEN, IngestPayload::LocationUri(entry.qnc_uri.clone())))
                    }
                    None if since.elapsed() >= FIND => {
                        self.skipped.push((serial, "the card is not among the disks Ingest lists".into()));
                        self.done_with_card();
                        None
                    }
                    None => {
                        self.sent = Some(Instant::now());
                        Some(IngestIntent::empty(action_ids::INGEST_DIR_ROOTS))
                    }
                }
            }
            Step::Select => {
                let uri = browser.current_uri.clone()?;
                // Ingest took it: Select runs (or ran already, a fast card).
                if view.command_busy || view.selected_source_uri.as_deref() == Some(uri.as_str()) {
                    self.step = Step::Selecting;
                    return None;
                }
                self.sent = Some(Instant::now());
                Some(IngestIntent::new(action_ids::INGEST_DIR_CONFIRM, IngestPayload::LocationUri(uri)))
            }
            Step::Selecting => {
                if !view.command_busy {
                    self.done_with_card();
                }
                None
            }
        }
    }

    fn done_with_card(&mut self) {
        self.cards.remove(0);
        self.step = Step::Disks;
        self.sent = None;
    }
}
