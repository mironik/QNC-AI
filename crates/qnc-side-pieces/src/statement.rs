// A statement of several parts (user 2026-11-19: "i složena i obična su iste izjave, samo složena
// u bazi drži više in/out"; "dodaj izjavu zapisuje 5 do 12 za prvi dio, 24 do 36 za drugi"). Its
// parts are played one after another, never joined, and drawn on the timeline in place of the
// source timeline by the SubSegment block (`qnc-subsegment-edit`), as a compound statement of the
// script is: this piece hands it the statement's IN/OUT as its parts. Nothing is written
// here except what was kept: Enter or Save writes its parts back to the statement's record
// (audit 2026-11-25: they were thrown away), what that record holds (clip, IN, OUT, join).

// (a module of the right panel pieces: it hands the SubSegment block the statement's parts)
use std::collections::HashMap;

use qnc_clip_row::ClipRow;
use qnc_script_db::{Block, Piece};
use qnc_source_preview::SourcePreview;
use qnc_subsegment_edit::{SubsegmentEdit, SubsegmentView};
use qnc_virtual_shots::{JoinKind, ShotPart};

/// The block id the SubSegment block knows the statement by (it has one block).
const BLOCK: i64 = 1;

#[derive(Default)]
pub struct StatementPlay {
    edit: SubsegmentEdit,
    block: Option<Block>,
    names: HashMap<String, (String, u64, f64)>,
    /// The project the waves and filmstrips of its clips are read from.
    project: Option<String>,
    /// The clip shown before the first part and after the last (frames).
    margins: (u64, u64),
    /// The first play request starts at the first part's IN, not at the window's start.
    cue_in: bool,
    /// Its play goes from the first part's IN to the last part's OUT (user 2026-11-23).
    play: qnc_source_panel::RangePlay,
    /// The value of the action being handled (a word's first clip frame).
    value: Option<i64>,
    /// The statement open and its parts as written (their joins are kept by the parts).
    shot_id: String,
    written: Vec<ShotPart>,
    /// A write of its parts going on, and whether one was written since asked.
    writing: Option<std::sync::mpsc::Receiver<Result<(), String>>>,
    pending: Vec<ShotPart>,
    wrote: bool,
}

impl std::fmt::Debug for StatementPlay {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("StatementPlay").field("open", &self.block.is_some()).finish()
    }
}

impl StatementPlay {
    pub fn new() -> Self {
        Self::default()
    }

    /// Opens the statement of `parts` (clip, IN, OUT; its own first) in place of the source
    /// timeline; the player plays them one after another.
    pub fn open(&mut self, shot_id: &str, parts: &[ShotPart], clips: &[ClipRow]) {
        self.names = clips.iter().map(|clip| (clip.clip_id.clone(), (clip.name.clone(), clip.duration_frames, clip.duration_seconds))).collect();
        self.shot_id = shot_id.to_string();
        self.written = parts.to_vec();
        let mut pieces: Vec<Piece> = parts.iter().filter_map(|part| piece(&part.clip_id, part.in_frame, part.out_frame, &self.names)).collect();
        if pieces.len() < 2 {
            return;
        }
        // 1/13 of the window either side, as a statement shows (user 2026-11-23): the clip before
        // the first part and after the last, as far as the clip goes.
        let length: u64 = pieces.iter().map(|piece| piece.to_frame.saturating_sub(piece.from_frame)).sum();
        let margin = length / 11;
        let (first, last) = (pieces[0].clone(), pieces[pieces.len() - 1].clone());
        let last_frames = self.names.get(&last.clip_id).map_or(last.to_frame, |(_, frames, _)| *frames);
        let before = margin.min(first.from_frame);
        let after = margin.min(last_frames.saturating_sub(last.to_frame));
        if before > 0 {
            pieces.insert(0, piece(&first.clip_id, first.from_frame - before, first.from_frame, &self.names).unwrap_or(first));
        }
        if after > 0 {
            pieces.extend(piece(&last.clip_id, last.to_frame, last.to_frame + after, &self.names));
        }
        self.margins = (before, after);
        self.edit.set_margins(before > 0, after > 0);
        self.cue_in = true;
        let mut block = Block::of_pieces("sot", pieces);
        block.id = BLOCK;
        self.edit.sync(std::slice::from_ref(&block), &self.names, self.project.as_ref(), true);
        if self.edit.open(BLOCK) {
            self.block = Some(block);
        }
    }

    /// Follows what the player plays; the parts go to the player when asked; what was kept is
    /// written to the statement's record.
    pub fn poll(&mut self, preview: &mut SourcePreview) {
        let Some(block) = &self.block else { return };
        // The project its waves and filmstrips are read from (without it none is read).
        self.project = preview.project_id().map(str::to_string);
        self.edit.sync(std::slice::from_ref(block), &self.names, self.project.as_ref(), true);
        let drafted = self.edit.drafted_block();
        self.edit.poll(preview);
        if let Some((pieces, frame)) = self.edit.take_pieces_request() {
            let frame = if std::mem::take(&mut self.cue_in) { self.margins.0 } else { frame };
            preview.show_pieces(&pieces, frame);
        }
        let length: u64 = block.pieces.iter().map(|piece| piece.to_frame.saturating_sub(piece.from_frame)).sum();
        let frame = preview.in_program().then(|| preview.player_view().confirmed_source_frame()).flatten();
        let out = length.saturating_sub(self.margins.1);
        self.play.follow(preview, frame, Some(out), Some(self.margins.0));
        if !self.edit.take_writes().is_empty() {
            self.keep(drafted, preview);
        }
        if let Some(result) = self.writing.as_ref().and_then(|writing| writing.try_recv().ok()) {
            self.writing = None;
            match result {
                Ok(()) => {
                    self.written = std::mem::take(&mut self.pending);
                    self.wrote = true;
                }
                Err(error) => qnc_notice::raise(error),
            }
        }
        if !self.edit.is_open() {
            self.block = None;
        }
    }

    /// The keys and pointer actions of its timeline (play, step, cue, zoom): `None` while it is
    /// not open or for another action.
    pub fn act(&mut self, action_id: &str, target: Option<&str>, preview: &mut SourcePreview) -> Option<bool> {
        self.block.as_ref()?;
        // A word of the transcript (user 2026-11-24: it sent a clip frame the statement's player
        // refused): the playhead goes to where the statement says it; a word it does not say
        // leaves the playhead.
        if action_id == qnc_transcript::CUE_WORD {
            let frame = self.value.and_then(|frame| u64::try_from(frame).ok());
            let at = frame.and_then(|frame| self.block.as_ref().and_then(|block| axis_frame(&block.pieces, preview.view().clip_id.as_deref(), frame)));
            let Some(at) = at else { return Some(true) };
            return self.act(qnc_subsegment_edit::SUBSEGMENT_CUE, Some(&at.to_string()), preview);
        }
        let own = self.edit.own(action_id, target);
        let (action_id, target) = match &own {
            Some((action, own_target)) => (action.as_str(), own_target.as_deref()),
            None => (action_id, target),
        };
        // What is kept (Enter, Save) stays as drafted (user 2026-11-23: "visible at once"): the
        // statement's parts held here become the draft that was kept.
        let drafted = self.edit.drafted_block();
        let done = self.edit.key(action_id, target, preview).or_else(|| self.edit.act(action_id, target, preview));
        if let Some((pieces, frame)) = self.edit.take_pieces_request() {
            preview.show_pieces(&pieces, frame);
        }
        if !self.edit.take_writes().is_empty() {
            self.keep(drafted, preview);
        }
        done
    }

    /// What was kept (Enter, Save, a mark let go): the draft becomes the statement shown, its
    /// margins against its first IN and last OUT, and its parts are written to its record.
    fn keep(&mut self, drafted: Option<Block>, preview: &SourcePreview) {
        let Some(mut drafted) = drafted else { return };
        // The margins stay against the first IN and the last OUT.
        let count = drafted.pieces.len();
        if self.margins.0 > 0 && count > 2 {
            let first_in = drafted.pieces[1].from_frame;
            (drafted.pieces[0].from_frame, drafted.pieces[0].to_frame) = (first_in.saturating_sub(self.margins.0), first_in);
        }
        if self.margins.1 > 0 && count > 2 {
            let last_out = drafted.pieces[count - 2].to_frame;
            let frames = self.names.get(&drafted.pieces[count - 1].clip_id).map_or(u64::MAX, |(_, frames, _)| *frames);
            (drafted.pieces[count - 1].from_frame, drafted.pieces[count - 1].to_frame) = (last_out, (last_out + self.margins.1).min(frames).max(last_out + 1));
        }
        // The statement's own parts, the margins left out.
        let own = usize::from(self.margins.0 > 0)..count.saturating_sub(usize::from(self.margins.1 > 0));
        let parts: Vec<ShotPart> = drafted.pieces.get(own).unwrap_or_default().iter().map(|piece| ShotPart {
            clip_id: piece.clip_id.clone(),
            in_frame: piece.from_frame,
            out_frame: piece.to_frame,
            join: join_of(&self.written, piece),
        }).collect();
        self.block = Some(drafted);
        if parts.is_empty() || parts == self.written {
            return;
        }
        let Some(target) = preview.project_target().cloned() else {
            return qnc_notice::raise(qnc_ui_text::text("stills.project_subclips_initialised"));
        };
        self.pending = parts.clone();
        let (send, receive) = std::sync::mpsc::channel();
        let shot_id = self.shot_id.clone();
        std::thread::spawn(move || {
            let _ = send.send(qnc_virtual_shots::set_parts_now(&target, &shot_id, parts));
        });
        self.writing = Some(receive);
    }

    /// Whether its parts were written since last asked (the statements are then read again).
    pub fn take_written(&mut self) -> bool {
        std::mem::take(&mut self.wrote)
    }

    /// What its timeline shows, while it is open.
    pub fn view(&self) -> Option<SubsegmentView> {
        let mut view = self.block.as_ref().and(self.edit.view.subsegment.clone())?;
        view.margins = self.margins;
        Some(view)
    }

    pub fn set_value(&mut self, value: Option<i64>) {
        self.value = value;
        self.edit.set_value(value);
    }
}

/// The join a part had when it was written (the same clip, overlapping its frames); a part new
/// to the statement joins with a cut.
fn join_of(written: &[ShotPart], piece: &Piece) -> JoinKind {
    written
        .iter()
        .find(|part| part.clip_id == piece.clip_id && part.in_frame < piece.to_frame && piece.from_frame < part.out_frame)
        .map_or(JoinKind::Cut, |part| part.join)
}

/// Where frame `frame` of a clip is on the axis of `pieces` (one after another, each after its
/// pause): the part of the shown clip that has it first, else any part that has it.
fn axis_frame(pieces: &[Piece], shown: Option<&str>, frame: u64) -> Option<u64> {
    let mut start = 0;
    let mut found = None;
    for piece in pieces {
        start += piece.gap_before;
        if (piece.from_frame..piece.to_frame).contains(&frame) {
            let at = start + frame - piece.from_frame;
            if shown == Some(piece.clip_id.as_str()) {
                return Some(at);
            }
            found = found.or(Some(at));
        }
        start += piece.to_frame.saturating_sub(piece.from_frame);
    }
    found
}

/// `[from, to)` of `clip` as a part (only IN/OUT: the text is read from the transcript).
fn piece(clip: &str, from: u64, to: u64, names: &HashMap<String, (String, u64, f64)>) -> Option<Piece> {
    let (name, frames, seconds) = names.get(clip).cloned().unwrap_or_default();
    let rate = if frames > 0 && seconds > 0.0 { frames as f64 / seconds } else { 0.0 };
    let part = serde_json::json!({
        "clip_id": clip,
        "clip_name": name,
        "from_frame": from,
        "to_frame": to,
        "start_tc": "",
        "end_tc": "",
        "length": "",
        "seconds": if rate > 0.0 { to.saturating_sub(from) as f64 / rate } else { 0.0 },
    });
    serde_json::from_value(part).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_word_of_a_clip_is_where_the_statement_says_it_and_nowhere_when_it_does_not() {
        let names = HashMap::from([("a".to_string(), ("A".to_string(), 1000, 20.0)), ("b".to_string(), ("B".to_string(), 1000, 20.0))]);
        let pieces = vec![piece("a", 90, 100, &names).unwrap(), piece("a", 100, 200, &names).unwrap(), piece("b", 150, 250, &names).unwrap()];
        assert_eq!(axis_frame(&pieces, Some("a"), 120), Some(30));
        assert_eq!(axis_frame(&pieces, Some("b"), 160), Some(120), "the shown clip's part first");
        assert_eq!(axis_frame(&pieces, Some("a"), 160), Some(70), "a frame both clips have: the shown clip's");
        assert_eq!(axis_frame(&pieces, Some("a"), 500), None, "a word the statement does not say");
    }
}
