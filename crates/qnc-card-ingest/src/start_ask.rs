//! When the tray asks whether QNC is started (user 2026-11-14: "na tray ingest prozoru ne treba biti
//! dugme Pokreni QNC, nego info izbornik koji kaže šta je uvezeno, šta je još u tijeku, s pitanjem
//! želite li da pokrenem QNC ili čekamo da kompletan Ingest završi"). One question for every base
//! being ingested, not a button per window: once the first base is ready for editing, and, when the
//! user chose to wait, again when all of them are done. Only decides; reads and shows nothing.

use std::collections::BTreeSet;

use crate::Progress;

/// What is asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    /// A base is ready for editing, another is still being ingested: start QNC now or wait?
    Ready,
    /// Every base is done: start QNC?
    AllDone,
}

/// The questions of one round: the bases being ingested together. A base that joins starts a new
/// round.
#[derive(Debug, Default)]
pub struct StartAsk {
    round: BTreeSet<String>,
    asked_ready: bool,
    asked_done: bool,
}

impl StartAsk {
    pub fn new() -> Self {
        Self::default()
    }

    /// The question to ask now about the bases being ingested (`id`, what is done), if one.
    pub fn next(&mut self, bases: &[(String, Progress)]) -> Option<Ask> {
        let ids: BTreeSet<String> = bases.iter().map(|(id, _)| id.clone()).collect();
        if ids.is_empty() {
            return None;
        }
        if !ids.is_subset(&self.round) {
            *self = Self { round: ids, ..Self::default() };
        }
        let all_done = bases.iter().all(|(_, progress)| progress.complete());
        let any_ready = bases.iter().any(|(_, progress)| progress.ready_for_qnc());
        if all_done && !self.asked_done {
            self.asked_done = true;
            self.asked_ready = true;
            return Some(Ask::AllDone);
        }
        if any_ready && !all_done && !self.asked_ready {
            self.asked_ready = true;
            return Some(Ask::Ready);
        }
        None
    }

    /// The answer: QNC started ends the round's questions; "wait" leaves the one at the end.
    pub fn answered(&mut self, start: bool) {
        if start {
            self.asked_ready = true;
            self.asked_done = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn working() -> Progress {
        Progress { clips: 4, imported: 2, transcripts: Some(1), ..Default::default() }
    }

    fn ready() -> Progress {
        Progress { clips: 4, imported: 4, posters: 4, transcripts: Some(4), filmstrips: Some(1), ..Default::default() }
    }

    fn done() -> Progress {
        Progress { clips: 4, imported: 4, posters: 4, transcripts: Some(4), ..Default::default() }
    }

    #[test]
    fn it_asks_once_a_base_is_ready_and_again_when_all_are_done_if_the_user_waited() {
        let mut ask = StartAsk::new();
        assert_eq!(ask.next(&[("a".into(), working()), ("b".into(), working())]), None, "nothing ready yet");
        assert_eq!(ask.next(&[("a".into(), ready()), ("b".into(), working())]), Some(Ask::Ready));
        assert_eq!(ask.next(&[("a".into(), ready()), ("b".into(), working())]), None, "asked once");
        ask.answered(false);
        assert_eq!(ask.next(&[("a".into(), done()), ("b".into(), done())]), Some(Ask::AllDone), "the user waited");
        assert_eq!(ask.next(&[("a".into(), done()), ("b".into(), done())]), None);
    }

    #[test]
    fn started_qnc_is_not_asked_again_but_a_new_base_is() {
        let mut ask = StartAsk::new();
        assert_eq!(ask.next(&[("a".into(), ready()), ("b".into(), working())]), Some(Ask::Ready));
        ask.answered(true);
        assert_eq!(ask.next(&[("a".into(), done()), ("b".into(), done())]), None, "QNC runs already");
        assert_eq!(ask.next(&[("a".into(), done()), ("b".into(), done()), ("c".into(), ready())]), Some(Ask::Ready), "a card put in later: a new round");
    }

    #[test]
    fn one_base_done_at_once_is_asked_only_at_the_end() {
        let mut ask = StartAsk::new();
        assert_eq!(ask.next(&[("a".into(), done())]), Some(Ask::AllDone));
    }
}
