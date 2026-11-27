//! Saving a SubClip (audit 2026-10-25: the checks and the write were in the Story application,
//! and the write waited for the database on the form thread). The clip must be one of the
//! imported clips of the pool and IN/OUT confirmed on the player; the short is written in the
//! background through `qnc-virtual-shots`, and the outcome is taken once it is there.

use std::sync::mpsc::{self, Receiver, TryRecvError};

use qnc_clip_row::ClipRow;
use qnc_db_broker::ProjectDbTarget;

pub const MODULE_ID: &str = "qnc.module.short-save";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// A saved short: its id, its clip and its IN/OUT.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Saved {
    pub shot_id: String,
    pub clip_id: String,
    pub marks: (u64, u64),
    /// A part added to the statement (its own stills stay; user 2026-11-19).
    pub part: bool,
}

/// The save of one short at a time.
#[derive(Default)]
pub struct ShortSave {
    pending: Option<Receiver<Result<Saved, String>>>,
    /// The SubClip whose IN or OUT was taken on the source timeline: `(shot, clip)`.
    fine_tuning: Option<(String, String)>,
    /// The cut last written for a SubClip (its row is read again after the write).
    written: Option<(String, (u64, u64))>,
    /// The same statement is already there: Overwrite or Cancel is asked (user 2026-11-01).
    asked: Option<Asked>,
    /// The clip shown and its camera timecode: IN and OUT are written as timecode too (user
    /// 2026-11-20: "time kod je jedino mjerilo"; the frames stay for the player).
    timecode: Option<(String, qnc_source_timecode::SourceTimecode)>,
    /// A new statement waiting for its caption (user 2026-11-19).
    captioning: Option<Writing>,
    /// A part waiting for how it joins the part before it (user 2026-11-23).
    joining: Option<Joining>,
}

/// A part of a statement asked about: the statement, the clip and IN/OUT, the database.
#[derive(Debug, Clone)]
struct Joining {
    shot_id: String,
    clip_id: String,
    marks: (u64, u64),
    target: ProjectDbTarget,
}

/// How the part joins the one before it chosen (target: `cut`, `smooth`, `blic`), or given up.
pub const JOIN: &str = "statement_join";
pub const JOIN_CANCEL: &str = "statement_join_cancel";

/// The catalog actions of the question "the same statement is already there".
pub const OVERWRITE: &str = "statement_overwrite";
pub const CANCEL: &str = "statement_cancel";
/// The caption of a new statement written (target: the fields) or given up.
pub const CAPTION_SAVE: &str = "statement_caption_save";
pub const CAPTION_CANCEL: &str = "statement_caption_cancel";

/// A save asked about: the statement already there with the same clip and IN/OUT.
#[derive(Debug, Clone)]
struct Asked {
    shot_id: String,
    clip_id: String,
    name: String,
    marks: (u64, u64),
    target: ProjectDbTarget,
}

/// A SubClip open on the source timeline: its id, clip and saved IN/OUT.
pub type OpenShort<'a> = (&'a str, &'a str, (u64, u64));

/// The action of the Update button of an open statement.
pub const UPDATE: &str = "statement_update";
/// The shown clip's IN/OUT added as a part of the chosen statement (user 2026-11-19: "svaka
/// izjava može postati složena"; grey while no statement is chosen).
pub const ADD_PART: &str = "statement_add_part";

/// Whether a part can be added: a statement chosen and an IN/OUT shown that is not its own.
pub fn can_add_part(open: Option<OpenShort<'_>>, shown_clip: Option<&str>, marks: Option<(u64, u64)>) -> bool {
    open.is_some_and(|(_, clip, saved)| shown_clip.is_some() && marks.is_some_and(|cut| cut.1 > cut.0 && !(Some(clip) == shown_clip && cut == saved)))
}

/// Whether the marks shown differ from the saved IN/OUT of the statement open on its clip.
pub fn changed(open: Option<OpenShort<'_>>, shown_clip: Option<&str>, marks: Option<(u64, u64)>) -> bool {
    open.is_some_and(|(_, clip, saved)| Some(clip) == shown_clip && marks.is_some_and(|cut| cut != saved))
}

impl ShortSave {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fine tuning of an open SubClip (user 2026-10-29: it went back to its old cut): while
    /// its IN or OUT is taken on the source timeline of its clip the tuning runs; once the
    /// mark is let go (Enter, a click) a cut other than the saved one is written as its new
    /// IN/OUT, and the outcome is given like a save's (this is `poll` too). New marks set
    /// with I and O without taking one stay a new SubClip (Add SubClip).
    pub fn follow(&mut self, open: Option<OpenShort<'_>>, shown_clip: Option<&str>, marks: Option<(u64, u64)>, taken: bool, target: Option<&ProjectDbTarget>, timecode: Option<&qnc_source_timecode::SourceTimecode>) -> Option<Result<Saved, String>> {
        self.timecode = shown_clip.zip(timecode).map(|(clip, timecode)| (clip.to_string(), *timecode));
        self.tune(open, shown_clip, marks, taken, target);
        self.poll()
    }

    /// IN and OUT of `clip` as its camera timecode; empty while its timecode is not known.
    fn labels(&self, clip: &str, (in_frame, out_frame): (u64, u64)) -> (String, String) {
        match &self.timecode {
            Some((shown, timecode)) if shown == clip && timecode.start_frame.is_some() => (timecode.label(in_frame), timecode.label(out_frame)),
            _ => (String::new(), String::new()),
        }
    }

    fn tune(&mut self, open: Option<OpenShort<'_>>, shown_clip: Option<&str>, marks: Option<(u64, u64)>, taken: bool, target: Option<&ProjectDbTarget>) {
        let open = open.filter(|(_, clip, _)| Some(*clip) == shown_clip);
        if taken {
            self.fine_tuning = open.map(|(shot, clip, _)| (shot.to_string(), clip.to_string())).or(self.fine_tuning.take());
            return;
        }
        let Some((shot, _)) = self.fine_tuning.take() else { return };
        let (Some((open_shot, clip_id, saved)), Some(cut), Some(target)) = (open, marks, target) else { return };
        let already = self.written.as_ref().is_some_and(|(id, written)| *id == shot && *written == cut);
        if open_shot != shot || cut == saved || already || self.pending.is_some() {
            return;
        }
        log(&format!("short-save fine tuning {shot} {saved:?} -> {cut:?}"));
        self.written = Some((shot.clone(), cut));
        let (tc_in, tc_out) = self.labels(clip_id, cut);
        let (target, clip_id) = (target.clone(), clip_id.to_string());
        let (sender, receiver) = mpsc::channel();
        let started = std::thread::Builder::new().name("qnc-short-trim".into()).spawn(move || {
            let trimmed = qnc_virtual_shots::trim_short_now(&target, &shot, cut.0, cut.1, (&tc_in, &tc_out)).map(|_| Saved { shot_id: shot, clip_id, marks: cut, part: false });
            let _ = sender.send(trimmed);
        });
        if started.is_ok() {
            self.pending = Some(receiver);
        }
    }

    /// Update of the open statement (user 2026-11-09): the IN/OUT its source timeline shows now
    /// become its IN/OUT, however they got there (keys, a drag).
    /// Adds the shown clip's IN/OUT as a part of the chosen statement, in the background.
    pub fn add_part(&mut self, open: Option<OpenShort<'_>>, shown_clip: Option<&str>, marks: Option<(u64, u64)>, target: Option<&ProjectDbTarget>) -> bool {
        let (true, Some((shot, _, _)), Some(clip), Some(cut), Some(target), None) = (can_add_part(open, shown_clip, marks), open, shown_clip, marks, target, &self.pending) else { return true };
        // How it joins the part before it is asked first (user 2026-11-23: Cut, Smooth or Blic,
        // "odmah pri dodavanju"); nothing is written until it is answered.
        self.joining = Some(Joining { shot_id: shot.to_string(), clip_id: clip.to_string(), marks: cut, target: target.clone() });
        true
    }

    /// Writes the part asked about with the join chosen, in the background.
    fn write_part(&mut self, joining: Joining, join: qnc_virtual_shots::JoinKind) {
        let Joining { shot_id: shot, clip_id, marks: cut, target } = joining;
        log(&format!("short-save part {shot} {clip_id} {cut:?} join={}", join.code()));
        let (sender, receiver) = mpsc::channel();
        let started = std::thread::Builder::new().name("qnc-short-part".into()).spawn(move || {
            let added = qnc_virtual_shots::add_part_now(&target, &shot, &clip_id, cut.0, cut.1, join).map(|_| Saved { shot_id: shot, clip_id, marks: cut, part: true });
            let _ = sender.send(added);
        });
        if started.is_ok() {
            self.pending = Some(receiver);
        }
    }

    pub fn update(&mut self, open: Option<OpenShort<'_>>, shown_clip: Option<&str>, marks: Option<(u64, u64)>, target: Option<&ProjectDbTarget>) -> bool {
        let (Some((shot, clip_id, _)), Some(cut), Some(target), None) = (open.filter(|open| changed(Some(*open), shown_clip, marks)), marks, target, &self.pending) else { return true };
        log(&format!("short-save update {shot} -> {cut:?}"));
        self.written = Some((shot.to_string(), cut));
        let (tc_in, tc_out) = self.labels(clip_id, cut);
        let (target, shot, clip_id) = (target.clone(), shot.to_string(), clip_id.to_string());
        let (sender, receiver) = mpsc::channel();
        let started = std::thread::Builder::new().name("qnc-short-trim".into()).spawn(move || {
            let trimmed = qnc_virtual_shots::trim_short_now(&target, &shot, cut.0, cut.1, (&tc_in, &tc_out)).map(|_| Saved { shot_id: shot, clip_id, marks: cut, part: false });
            let _ = sender.send(trimmed);
        });
        if started.is_ok() {
            self.pending = Some(receiver);
        }
        true
    }

    /// Checks what is asked and starts the write: the chosen clip of `clips`, the marks the
    /// source timeline shows, the project database and project. The error is the message to
    /// show; nothing is written then.
    pub fn start(
        &mut self,
        chosen: Option<&str>,
        clips: &[ClipRow],
        marks: Option<(u64, u64)>,
        target: Option<&ProjectDbTarget>,
        project_id: Option<&str>,
        saved: &[qnc_virtual_short_cards::VirtualShortCard],
    ) -> Result<(), String> {
        // The same clip and IN/OUT saved already: asked, not saved twice (user 2026-11-01).
        let same = saved.iter().find(|card| !card.b_roll && Some(card.clip_id.as_str()) == chosen && Some((card.in_frame, card.out_frame)) == marks);
        if let (Some(card), Some(target)) = (same, target) {
            log(&format!("short-save asks: {} is the same statement", card.shot_id));
            let asked = Asked { shot_id: card.shot_id.clone(), clip_id: card.clip_id.clone(), name: card.name.clone(), marks: (card.in_frame, card.out_frame), target: target.clone() };
            self.asked = Some(asked);
            return Ok(());
        }
        // Why a SubClip was not saved goes to the log too (live 2026-10-28: none was saved and
        // the log did not say why).
        let checked = check(chosen, clips, marks, target, project_id);
        if let Err(error) = &checked {
            log(&format!("short-save refused clip={chosen:?} marks={marks:?}: {error}"));
        }
        // Who speaks and their role are asked before it is written (user 2026-11-19: the caption
        // the director puts in); the last ones written for this clip are offered.
        let mut writing = checked?;
        let last = saved.iter().rev().find(|card| !card.b_roll && card.clip_id == writing.clip_id && !card.caption_name.is_empty());
        writing.caption = last.map(|card| (card.caption_name.clone(), card.caption_role.clone())).unwrap_or_default();
        self.captioning = Some(writing);
        Ok(())
    }

    /// Writes the statement with its caption and its IN and OUT as timecode.
    fn write(&mut self, writing: Writing) -> Result<(), String> {
        let (tc_in, tc_out) = self.labels(&writing.clip_id, writing.marks);
        let Writing { clip_id, name, marks: (in_frame, out_frame), target, project_id, caption: (caption_name, caption_role) } = writing;
        let (sender, receiver) = mpsc::channel();
        std::thread::Builder::new()
            .name("qnc-short-save".into())
            .spawn(move || {
                let saved = qnc_virtual_shots::save_short_now(&target, &project_id, &clip_id, &name, in_frame, out_frame, (&tc_in, &tc_out)).and_then(|shot| {
                    if !caption_name.trim().is_empty() || !caption_role.trim().is_empty() {
                        qnc_virtual_shots::set_caption_now(&target, &shot.shot_id, &caption_name, &caption_role)?;
                    }
                    Ok(Saved { shot_id: shot.shot_id, clip_id, marks: (in_frame, out_frame), part: false })
                });
                let _ = sender.send(saved.map_err(|error| error.to_string()));
            })
            .map_err(|error| error.to_string())?;
        self.pending = Some(receiver);
        Ok(())
    }

    /// The question while the same statement is already there: an overlay with Overwrite and
    /// Cancel (user 2026-11-01); while a new one waits for its caption: who speaks and their role.
    pub fn question(&self) -> Option<qnc_notice::Choice> {
        let option = |action_id: &str, code: &'static str, strong| qnc_notice::ChoiceOption::new(action_id, text(code), strong);
        if self.joining.is_some() {
            let join = |kind: qnc_virtual_shots::JoinKind, code: &'static str| option(JOIN, code, kind == qnc_virtual_shots::JoinKind::Cut).with_target(kind.code());
            return Some(qnc_notice::Choice {
                title: text("statement.join_title"),
                message: text("statement.join_message"),
                items: Vec::new(),
                solution: String::new(),
                options: vec![
                    option(JOIN_CANCEL, "common.cancel", false),
                    join(qnc_virtual_shots::JoinKind::Blic, "statement.join_blic"),
                    join(qnc_virtual_shots::JoinKind::Smooth, "statement.join_smooth"),
                    join(qnc_virtual_shots::JoinKind::Cut, "statement.join_cut"),
                ],
                fields: Vec::new(),
            });
        }
        if let Some(writing) = &self.captioning {
            let field = |code: &'static str, value: &str| qnc_notice::ChoiceField { label: text(code), value: value.to_string() };
            return Some(qnc_notice::Choice {
                title: text("statement.caption_title"),
                message: text("statement.caption_message"),
                items: vec![writing.name.clone()],
                solution: String::new(),
                options: vec![option(CAPTION_CANCEL, "common.cancel", false), option(CAPTION_SAVE, "common.save", true).taking_fields()],
                fields: vec![field("statement.caption_name", &writing.caption.0), field("statement.caption_role", &writing.caption.1)],
            });
        }
        let asked = self.asked.as_ref()?;
        let option = |action_id: &str, code: &'static str, strong| qnc_notice::ChoiceOption::new(action_id, text(code), strong);
        Some(qnc_notice::Choice {
            title: text("statement.exists_title"),
            message: text("statement.exists"),
            items: vec![asked.name.clone()],
            solution: text("statement.exists_solution"),
            options: vec![option(OVERWRITE, "statement.overwrite", false), option(CANCEL, "common.cancel", true)],
            fields: Vec::new(),
        })
    }

    /// The answer to the question; `false`: `action_id` is not one of its answers. Overwrite
    /// writes the statement already there again (its IN/OUT and stills made anew), never a
    /// second one; Cancel leaves it.
    pub fn answer(&mut self, action_id: &str, target: Option<&str>) -> bool {
        // How a part joins the one before it: written with the kind chosen, or nothing.
        if action_id == JOIN || action_id == JOIN_CANCEL {
            let Some(joining) = self.joining.take() else { return true };
            if let (JOIN, Some(join)) = (action_id, target.and_then(qnc_virtual_shots::JoinKind::of)) {
                self.write_part(joining, join);
            }
            return true;
        }
        // The caption of a new statement: Save writes it with what was written, Cancel writes nothing.
        if action_id == CAPTION_SAVE || action_id == CAPTION_CANCEL {
            let Some(mut writing) = self.captioning.take() else { return true };
            if action_id == CAPTION_CANCEL {
                return true;
            }
            let fields = target.map(qnc_notice::fields_of).unwrap_or_default();
            writing.caption = (fields.first().cloned().unwrap_or_default(), fields.get(1).cloned().unwrap_or_default());
            log(&format!("short-save caption name={:?} role={:?}", writing.caption.0, writing.caption.1));
            if let Err(error) = self.write(writing) {
                log(&format!("short-save not started: {error}"));
            }
            return true;
        }
        if action_id != OVERWRITE && action_id != CANCEL {
            return false;
        }
        let Some(asked) = self.asked.take() else { return true };
        if action_id == CANCEL || self.pending.is_some() {
            return true;
        }
        log(&format!("short-save overwrites {}", asked.shot_id));
        let (tc_in, tc_out) = self.labels(&asked.clip_id, asked.marks);
        let (sender, receiver) = mpsc::channel();
        let started = std::thread::Builder::new().name("qnc-short-overwrite".into()).spawn(move || {
            let (in_frame, out_frame) = asked.marks;
            let written = qnc_virtual_shots::trim_short_now(&asked.target, &asked.shot_id, in_frame, out_frame, (&tc_in, &tc_out))
                .map(|_| Saved { shot_id: asked.shot_id, clip_id: asked.clip_id, marks: asked.marks, part: false });
            let _ = sender.send(written);
        });
        if started.is_ok() {
            self.pending = Some(receiver);
        }
        true
    }

    /// The outcome of the save, once.
    pub fn poll(&mut self) -> Option<Result<Saved, String>> {
        let outcome = match self.pending.as_ref()?.try_recv() {
            Ok(outcome) => outcome,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Err(text("story.msg.db_not_writable")),
        };
        self.pending = None;
        match &outcome {
            Ok(saved) => log(&format!("short-save saved {} clip={} marks={:?}", saved.shot_id, saved.clip_id, saved.marks)),
            Err(error) => log(&format!("short-save failed: {error}")),
        }
        Some(outcome)
    }
}

fn log(line: &str) {
    if qnc_dev_diagnostics::player_diagnostics_enabled() {
        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line.to_string());
    }
}

fn text(code: &'static str) -> String {
    qnc_ui_text::text(code).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(imported: bool) -> ClipRow {
        ClipRow { clip_id: "c".into(), name: "C".into(), imported, ..ClipRow::default() }
    }

    #[test]
    fn update_is_there_only_when_the_marks_of_the_open_statement_changed() {
        let open = Some(("s", "c", (10, 20)));
        assert!(!changed(open, Some("c"), Some((10, 20))));
        assert!(changed(open, Some("c"), Some((12, 20))));
        assert!(!changed(open, Some("other"), Some((12, 20))));
        assert!(!changed(None, Some("c"), Some((12, 20))));
    }

    #[test]
    fn a_part_asks_how_it_joins_first_cut_proposed_and_cancel_writes_nothing() {
        let target = ProjectDbTarget::from_owner_binding(std::path::Path::new("nowhere.db"), "qnc://local/db/project_db/p1").unwrap();
        let mut save = ShortSave::new();
        assert!(save.add_part(Some(("s", "c", (10, 20))), Some("c"), Some((30, 40)), Some(&target)));
        let asked = save.question().expect("the join is asked before anything is written");
        assert!(save.pending.is_none(), "nothing written before the answer");
        let options: Vec<(&str, Option<&str>, bool)> = asked.options.iter().map(|o| (o.action_id.as_str(), o.target.as_deref(), o.strong)).collect();
        assert_eq!(options, [(JOIN_CANCEL, None, false), (JOIN, Some("blic"), false), (JOIN, Some("smooth"), false), (JOIN, Some("cut"), true)]);
        assert!(save.answer(JOIN_CANCEL, None));
        assert!(save.question().is_none() && save.pending.is_none(), "Cancel writes no part");
    }

    #[test]
    fn nothing_is_written_without_a_clip_marks_and_a_database() {
        let mut save = ShortSave::new();
        assert_eq!(save.start(None, &[], Some((0, 5)), None, Some("p"), &[]), Err(text("story.msg.pick_clip")));
        // A row read before the import does not refuse: the database decides when it writes.
        assert_eq!(save.start(Some("c"), &[clip(false)], Some((0, 5)), None, Some("p"), &[]), Err(text("story.msg.db_not_writable")));
        assert_eq!(save.start(Some("c"), &[clip(true)], None, None, Some("p"), &[]), Err(text("story.msg.marks_not_confirmed")));
        assert_eq!(save.start(Some("c"), &[clip(true)], Some((0, 5)), None, Some("p"), &[]), Err(text("story.msg.db_not_writable")));
        assert!(save.poll().is_none(), "no write was started");
    }

    #[test]
    fn the_same_statement_is_asked_about_and_cancel_writes_nothing() {
        let card = |b_roll: bool| qnc_virtual_short_cards::VirtualShortCard {
            shot_id: if b_roll { "c_broll_001" } else { "c_shot_001" }.into(),
            clip_id: "c".into(),
            name: "C".into(),
            in_frame: 0,
            out_frame: 5,
            duration_label: String::new(),
            import_status: "imported".into(),
            imported_media_uri: String::new(),
            poster_uri: None,
            poster_image: None,
            b_roll,
            caption_name: String::new(),
            caption_role: String::new(),
            parts: Vec::new(),
        };
        let target = ProjectDbTarget::from_owner_binding(std::path::Path::new("nowhere.db"), "qnc://local/db/project_db/p1").unwrap();
        let mut save = ShortSave::new();
        assert!(save.start(Some("c"), &[clip(true)], Some((0, 5)), Some(&target), Some("p"), &[card(true), card(false)]).is_ok());
        let question = save.question().expect("the same statement (not the B-roll shot) is asked about");
        assert_eq!(save.asked.as_ref().map(|a| a.shot_id.as_str()), Some("c_shot_001"));
        assert_eq!(question.options.iter().map(|o| o.action_id.as_str()).collect::<Vec<_>>(), [OVERWRITE, CANCEL]);
        assert!(save.pending.is_none(), "nothing is written while asking");
        assert!(!save.answer("play_pause", None), "not an answer");
        assert!(save.answer(CANCEL, None));
        assert!(save.question().is_none() && save.pending.is_none(), "Cancel writes nothing");
        // A new statement asks who speaks first, offering the last caption of its clip.
        let captioned = qnc_virtual_short_cards::VirtualShortCard { in_frame: 10, out_frame: 20, caption_name: "Ana Anić".into(), caption_role: "gradonačelnica".into(), ..card(false) };
        assert!(save.start(Some("c"), &[clip(true)], Some((0, 5)), Some(&target), Some("p"), &[captioned]).is_ok());
        let asked = save.question().expect("the caption is asked");
        assert_eq!(asked.fields.iter().map(|f| f.value.as_str()).collect::<Vec<_>>(), ["Ana Anić", "gradonačelnica"]);
        assert!(save.pending.is_none(), "nothing is written before the caption");
        assert!(save.answer(CAPTION_CANCEL, None));
        assert!(save.question().is_none() && save.pending.is_none(), "Cancel writes no statement");
    }
}

/// A new statement checked and waiting for its caption.
#[derive(Debug, Clone)]
struct Writing {
    clip_id: String,
    name: String,
    marks: (u64, u64),
    target: ProjectDbTarget,
    project_id: String,
    /// Who speaks and their role.
    caption: (String, String),
}

/// A new statement of `chosen` between `marks`, checked before it is written.
fn check(chosen: Option<&str>, clips: &[ClipRow], marks: Option<(u64, u64)>, target: Option<&ProjectDbTarget>, project_id: Option<&str>) -> Result<Writing, String> {
    let clip_id = chosen.ok_or_else(|| text("story.msg.pick_clip"))?;
    let clip = clips.iter().find(|clip| clip.clip_id == clip_id).ok_or_else(|| text("ingest.msg.clip_not_found"))?;
    // Whether the clip is imported is the database's, read when the short is written
    // (`qnc-virtual-shots`), not the pool row a form read earlier (live 2026-10-29).
    let marks = marks.ok_or_else(|| text("story.msg.marks_not_confirmed"))?;
    let target = target.ok_or_else(|| text("story.msg.db_not_writable"))?.clone();
    let project_id = project_id.ok_or_else(|| text("story.msg.no_active_project"))?.to_string();
    Ok(Writing { clip_id: clip.clip_id.clone(), name: clip.name.clone(), marks, target, project_id, caption: Default::default() })
}
