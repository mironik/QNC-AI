// A statement of several parts (user 2026-11-19: "i složena i obična su iste izjave, samo složena
// u bazi drži više in/out"; "dodaj izjavu zapisuje 5 do 12 za prvi dio, 24 do 36 za drugi"). Its
// parts are played one after another, never joined, and drawn on the timeline in place of the
// source timeline by the SubSegment block (`qnc-subsegment-edit`), as a compound statement of the
// script is: this piece only hands it the statement's IN/OUT as its parts. Nothing is written
// here: the parts are the statement's record.

// (a module of the right panel pieces: it hands the SubSegment block the statement's parts)
use std::collections::HashMap;

use qnc_clip_row::ClipRow;
use qnc_script_db::{Block, Piece};
use qnc_source_preview::SourcePreview;
use qnc_subsegment_edit::{SubsegmentEdit, SubsegmentView};

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
    pub fn open(&mut self, parts: &[(String, u64, u64)], clips: &[ClipRow]) {
        self.names = clips.iter().map(|clip| (clip.clip_id.clone(), (clip.name.clone(), clip.duration_frames, clip.duration_seconds))).collect();
        let mut pieces: Vec<Piece> = parts.iter().filter_map(|(clip, from, to)| piece(clip, *from, *to, &self.names)).collect();
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
        self.cue_in = true;
        let mut block = Block::of_pieces("sot", pieces);
        block.id = BLOCK;
        self.edit.sync(std::slice::from_ref(&block), &self.names, self.project.as_ref(), true);
        if self.edit.open(BLOCK) {
            self.block = Some(block);
        }
    }

    /// Follows what the player plays; the parts go to the player when asked. Its edits are not
    /// written (the statement's parts are its record).
    pub fn poll(&mut self, preview: &mut SourcePreview) {
        let Some(block) = &self.block else { return };
        // The project its waves and filmstrips are read from (without it none is read).
        self.project = preview.project_id().map(str::to_string);
        self.edit.sync(std::slice::from_ref(block), &self.names, self.project.as_ref(), true);
        self.edit.poll(preview);
        if let Some((pieces, frame)) = self.edit.take_pieces_request() {
            let frame = if std::mem::take(&mut self.cue_in) { self.margins.0 } else { frame };
            preview.show_pieces(&pieces, frame);
        }
        let _ = self.edit.take_writes();
        if !self.edit.is_open() {
            self.block = None;
        }
    }

    /// The keys and pointer actions of its timeline (play, step, cue, zoom): `None` while it is
    /// not open or for another action.
    pub fn act(&mut self, action_id: &str, target: Option<&str>, preview: &mut SourcePreview) -> Option<bool> {
        self.block.as_ref()?;
        let own = self.edit.own(action_id, target);
        let (action_id, target) = match &own {
            Some((action, own_target)) => (action.as_str(), own_target.as_deref()),
            None => (action_id, target),
        };
        let done = self.edit.key(action_id, target, preview).or_else(|| self.edit.act(action_id, target, preview));
        if let Some((pieces, frame)) = self.edit.take_pieces_request() {
            preview.show_pieces(&pieces, frame);
        }
        let _ = self.edit.take_writes();
        done
    }

    /// What its timeline shows, while it is open.
    pub fn view(&self) -> Option<SubsegmentView> {
        let mut view = self.block.as_ref().and(self.edit.view.subsegment.clone())?;
        view.margins = self.margins;
        Some(view)
    }

    pub fn set_value(&mut self, value: Option<i64>) {
        self.edit.set_value(value);
    }
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
