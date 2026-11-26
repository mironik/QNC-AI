//! The zoom of the source timeline (user 2026-11-25: "the zoom is an enlarged part of the timeline,
//! not of a form"): the taken IN or OUT of the shown clip, or of a statement, enlarged, with its
//! pictures, the player in the middle and the sound on the clip's own axis. It belongs to the one
//! source timeline, so every form that embeds it has it; a form may leave it unused ([`ZoomUse`]).
//! After Enter it stays open on the side last taken until Esc, Close or another clip.

use qnc_program_view::{ClipAxis, CutPictures, EditStill, SegmentsView, StillAction, StillPicture, ZoomFollow, ZoomLane};
use qnc_source_preview::SourcePreview;
use qnc_timeline_model::TimelineProjection;

/// Whether the form uses the source timeline's zoom this frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoomUse {
    /// The form does not use it (Ingest for now: it can, it does not).
    Off,
    /// Shown while a mark is taken, and after Enter until it is closed.
    On,
    /// Another zoom holds the cut on this timeline (a segment edge taken on it): not shown.
    Yield,
}

/// What the source timeline knows of its clip and its lanes for the zoom.
pub(crate) struct Clip<'a> {
    pub clip_id: Option<&'a str>,
    pub lanes: [qnc_lane_choice::Lane; 2],
    pub lane_choices: [Option<ZoomLane>; 2],
    pub curves: [Vec<(u64, i16)>; 2],
    /// Where IN and OUT were when the taken mark was taken.
    pub origin: (Option<u64>, Option<u64>),
    /// A statement is enlarged on the timeline.
    pub statement: bool,
    pub message: &'a str,
    /// The clip's timecode, for the labels of its history.
    pub timecode: Option<qnc_source_timecode::SourceTimecode>,
}

#[derive(Default)]
pub(crate) struct SourceZoom {
    /// The clip whose zoom stays open with no mark taken, and the side last taken (IN `true`;
    /// `None`: opened by a right click in the slot, the player in the middle active).
    open: Option<(String, Option<bool>)>,
    /// Opened on the slot (user 2026-11-26): its IN, the player and its OUT while it stays open.
    slot: bool,
    follow: ZoomFollow,
    stills: qnc_trim_stills::TrimStills,
    /// The history of the clip in the zoom (user 2026-11-25: kept per clip, any position can be gone
    /// back to), its entries as last read, the list open with its lit row, the side taken last frame
    /// and IN/OUT when it was taken (a Save that moved them writes an entry).
    history: qnc_clip_marks::ClipHistory,
    entries: Vec<qnc_clip_marks::Entry>,
    list: Option<usize>,
    last_taken: Option<bool>,
    held_from: Option<(u64, u64)>,
}

impl SourceZoom {
    /// Esc or the zoom's Close: a zoom left open with no mark taken closes.
    pub fn close(&mut self) -> bool {
        self.list = None;
        self.slot = false;
        self.open.take().is_some()
    }

    /// A right click inside the slot (or Ctrl+E): the zoom of `clip_id` opens with nothing taken,
    /// its IN, the player (active) and its OUT; the playhead stays where it is (user 2026-11-26).
    pub fn open_slot(&mut self, clip_id: &str) {
        self.open = Some((clip_id.to_string(), None));
        self.slot = true;
    }

    /// Decoding its pictures or reading its sound.
    pub fn loading(&self) -> bool {
        self.stills.loading()
    }

    /// The side the zoom shows (IN `true`): the taken mark's, else the side last taken while the
    /// zoom stays open after Enter; `Some(None)`: the player in the middle active (opened on the
    /// slot); `None`: closed. Another clip closes it.
    fn side(&mut self, taken: Option<bool>, clip_id: Option<&str>) -> Option<Option<bool>> {
        if let Some(side) = taken {
            self.open = clip_id.map(|id| (id.to_string(), Some(side)));
        } else if self.open.as_ref().map(|(id, _)| id.as_str()) != clip_id {
            self.open = None;
            self.slot = false;
        }
        taken.map(Some).or(self.open.as_ref().map(|(_, side)| *side))
    }

    /// The zoom this frame, its pictures from the monitor or decoded, its sound read.
    pub fn view(&mut self, used: ZoomUse, clip: Clip<'_>, timeline: &TimelineProjection, preview: &SourcePreview, given: &[(u64, std::sync::Arc<qnc_image_assets::RgbaImage>)]) -> Option<Box<SegmentsView>> {
        let taken = timeline.taken_source_mark();
        let side = self.side(taken, clip.clip_id);
        if used != ZoomUse::On {
            return None;
        }
        let clip_id = clip.clip_id?;
        let marks = timeline.visible_source_marks()?;
        self.keep_history(preview, clip_id, taken, marks);
        let side = side?;
        let timebase = preview.player_view().source_timebase()?;
        let playing = preview.playing_at().filter(|(program, _, _)| !program);
        // The player shows the playhead: where it plays, else where it stands.
        let playhead = playing.map(|(_, frame, _)| frame).or_else(|| timeline.confirmed_frame());
        let stills = stills(clip_id, marks, side, taken.is_some(), self.slot, &clip, timeline.duration_frames, playhead);
        let cut = stills.iter().find(|still| still.cut).or(stills.first()).map(|still| still.marker_frame)?;
        let heard = clip.lanes.map(|lane| ZoomLane { channel: lane.channel, off: lane.off, level_db: lane.level_db, ..Default::default() });
        let axis = ClipAxis {
            clip_id,
            cut,
            timebase: (timebase.fps_num.max(0) as u64, timebase.fps_den.max(1) as u64),
            duration: timeline.duration_frames,
            position: playing.map(|(_, frame, _)| (frame, true)),
            follow: &self.follow,
            heard,
            lane_choices: clip.lane_choices,
            curves: clip.curves,
            marks: Some(marks),
        };
        let rows = self.entries.iter().rev().map(|entry| (entry.id, history_row(entry, clip.timecode))).collect::<Vec<_>>();
        let zoom_history = qnc_program_view::ZoomHistory { open: self.list.is_some(), at: self.list.unwrap_or(0).min(rows.len().saturating_sub(1)), rows };
        let mut view = SegmentsView { source_edit_stills: stills, zoom_sound: Some(axis.sound()), message: clip.message.to_string(), zoom_history: Some(zoom_history), ..Default::default() };
        self.stills.give(Some(clip_id), given);
        self.stills.fill(&mut view, preview.still_locator(), None, preview.shown_picture());
        Some(Box::new(view))
    }
}

/// The pictures of the zoom (user 2026-10-25: as an M marker's, the picture before the cut and the
/// one after it). A statement (user 2026-11-20) has three: its IN picture, the player and its OUT
/// picture; a click on IN or OUT takes that mark, a click on the player lets the taken mark go and
/// puts the playhead on IN; the taken side is the player and the middle holds where it was.
/// A zoom opened on the slot (user 2026-11-26) has the same three; with nothing taken (`side`
/// `None`) the player is the active place, framed.
#[allow(clippy::too_many_arguments)]
fn stills(clip_id: &str, (in_frame, out_frame): (u64, u64), side: Option<bool>, held: bool, slot: bool, clip: &Clip<'_>, duration: u64, playhead: Option<u64>) -> Vec<EditStill> {
    let picture = |frame: u64| Some(StillPicture { clip_id: clip_id.to_string(), source_frame: frame });
    let taken = side.unwrap_or(true);
    if !clip.statement && !slot {
        let cut = if taken {
            CutPictures { cut: in_frame, before: (in_frame > 0).then(|| (picture(in_frame - 1), qnc_ui_text::text("story.still.before_in"))), after: Some((picture(in_frame), qnc_ui_text::text("story.still.in_first"))), moving_after: true }
        } else {
            CutPictures { cut: out_frame, before: Some((picture(out_frame.saturating_sub(1)), qnc_ui_text::text("story.still.out_last"))), after: (out_frame < duration).then(|| (picture(out_frame), qnc_ui_text::text("story.still.after_out"))), moving_after: false }
        };
        return cut.stills();
    }
    let action = |action: &str| Some(StillAction::new(action, None, None));
    let last = out_frame.saturating_sub(1).max(in_frame);
    let playhead = playhead.unwrap_or(if taken { in_frame } else { last });
    let mut in_still = EditStill::new(in_frame, true, picture(in_frame)).labelled(&qnc_ui_text::text("story.still.in_first"));
    in_still.click = action("select_mark_in");
    in_still.cut = held && taken;
    let mut player = EditStill::new(playhead, true, picture(playhead)).labelled(&qnc_ui_text::text("zoom.player"));
    player.live = true; // the player itself, the monitor's picture as it plays
    player.middle = true;
    player.cut = side.is_none(); // nothing taken: the player is the active place
    player.click = Some(StillAction::new("activate_focused_item", None, Some(in_frame)));
    let mut out_still = EditStill::new(out_frame, false, picture(last)).labelled(&qnc_ui_text::text("story.still.out_last"));
    out_still.click = action("select_mark_out");
    out_still.cut = held && !taken;
    let mut stills = vec![in_still, player, out_still];
    let origin = if taken { clip.origin.0 } else { clip.origin.1.map(|out| out.saturating_sub(1)) };
    qnc_program_view::hand_player(&mut stills, origin.filter(|_| held).and_then(picture));
    stills
}

impl std::fmt::Debug for SourceZoom {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SourceZoom").field("open", &self.open).finish_non_exhaustive()
    }
}

impl SourceZoom {
    /// The history of the clip read (the journal of the project), and a Save that moved IN or OUT
    /// written as the clip's marks (its first Save keeps where they were before it).
    fn keep_history(&mut self, preview: &SourcePreview, clip_id: &str, taken: Option<bool>, marks: (u64, u64)) {
        let target = preview.project_target();
        if let Some(entries) = self.history.for_clip(target, Some(clip_id)) {
            self.entries = entries;
        }
        match (self.last_taken, taken) {
            (None, Some(_)) => self.held_from = Some(marks),
            (Some(_), None) => {
                if let (Some(target), Some(from)) = (target, self.held_from.take()) {
                    if from != marks {
                        self.history.keep(target, clip_id, from, marks);
                    }
                }
            }
            _ => {}
        }
        self.last_taken = taken;
    }

    /// Ctrl+H, a row of the list and the keys of the open list (Up/Down the lit row, Enter
    /// restores it, Esc closes it); `None`: not the history's. A restore puts IN and OUT back,
    /// takes the side the zoom shows (Enter writes it as any edit) and is itself an entry.
    pub fn history_key(&mut self, action_id: &str, value: Option<i64>, preview: &SourcePreview, timeline: &mut TimelineProjection) -> Option<bool> {
        let shown = self.open.is_some();
        let count = self.entries.len();
        let restore = match (action_id, self.list) {
            (qnc_edit_verbs::HISTORY_OPEN, _) if shown => {
                self.list = if self.list.is_some() { None } else { Some(0) };
                return Some(true);
            }
            (qnc_edit_verbs::HISTORY_RESTORE, _) => value,
            ("step_prev_part", Some(at)) => {
                self.list = Some(at.saturating_sub(1));
                return Some(true);
            }
            ("step_next_part", Some(at)) => {
                self.list = Some((at + 1).min(count.saturating_sub(1)));
                return Some(true);
            }
            ("activate_focused_item", Some(at)) => self.entries.iter().rev().nth(at).map(|entry| entry.id),
            ("clear_focus", Some(_)) => {
                self.list = None;
                return Some(true);
            }
            _ => return None,
        };
        self.list = None;
        let found = self.entries.iter().find(|entry| Some(entry.id) == restore)?.clone();
        let (in_frame, out_frame) = (found.in_frame, found.out_frame);
        let side = self.open.as_ref().and_then(|(_, side)| *side).unwrap_or(true);
        timeline.source_in_frame = Some(in_frame);
        timeline.source_out_frame = Some(out_frame);
        qnc_source_mark_focus::take(timeline, side);
        (self.last_taken, self.held_from) = (Some(side), Some((in_frame, out_frame)));
        if let Some(target) = preview.project_target() {
            self.history.back(target, &found.clip_id, (in_frame, out_frame));
        }
        Some(true)
    }
}

/// What an entry of IN and OUT says: their timecodes (frames without one).
fn label((in_frame, out_frame): (u64, u64), timecode: Option<qnc_source_timecode::SourceTimecode>) -> String {
    let tc = |frame: u64| timecode.map_or(frame.to_string(), |timecode| timecode.label(frame));
    qnc_ui_text::fill("zoom.history_marks", &[("in", &tc(in_frame)), ("out", &tc(out_frame))])
}

/// A row of the list: the time it was written (hours and minutes) and its IN and OUT (a going
/// back says so).
fn history_row(entry: &qnc_clip_marks::Entry, timecode: Option<qnc_source_timecode::SourceTimecode>) -> String {
    let marks = label((entry.in_frame, entry.out_frame), timecode);
    let said = if entry.back { qnc_ui_text::fill("zoom.history_back", &[("label", &marks)]) } else { marks };
    let time = entry.made_at_utc.get(11..16).unwrap_or("");
    if time.is_empty() { said } else { format!("{time}  {said}") }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip(statement: bool) -> Clip<'static> {
        Clip { clip_id: Some("clip-x"), lanes: Default::default(), lane_choices: [None, None], curves: Default::default(), origin: (None, None), statement, message: "", timecode: None }
    }

    /// User 2026-11-20: a statement's zoom is three windows, its IN picture, the player (the
    /// picture at the playhead) and its OUT picture; the taken edge is the cut.
    #[test]
    fn a_statements_zoom_is_its_in_the_player_and_its_out_and_the_taken_edge_is_the_cut() {
        let frames = |stills: Vec<EditStill>| stills.iter().map(|still| (still.picture.as_ref().map(|p| p.source_frame), still.cut)).collect::<Vec<_>>();
        assert_eq!(frames(stills("clip-x", (10, 60), Some(true), true, false, &clip(true), 60, None)), [(Some(10), true), (Some(10), false), (Some(59), false)], "no player frame yet: the player shows the taken IN");
        assert_eq!(frames(stills("clip-x", (10, 40), Some(false), true, false, &clip(true), 60, None)), [(Some(10), false), (Some(39), false), (Some(39), true)]);
    }

    /// User 2026-11-22: a click on the IN or OUT picture takes that mark (as Ctrl+I / Ctrl+O), a
    /// click on the player lets go of the taken mark and puts the playhead on IN.
    #[test]
    fn a_click_on_the_in_or_out_picture_takes_the_mark_the_player_lets_it_go() {
        let clicks: Vec<_> = stills("clip-x", (10, 40), Some(true), true, false, &clip(true), 60, None).iter().map(|still| still.click.as_ref().map(|click| (click.action.clone(), click.value))).collect();
        let take = |action: &str| Some((action.to_string(), None));
        assert_eq!(clicks, [take("select_mark_in"), Some(("activate_focused_item".to_string(), Some(10))), take("select_mark_out")]);
    }

    /// A taken IN or OUT: the picture before the cut and the one after it.
    #[test]
    fn a_taken_in_or_out_shows_the_pictures_around_its_cut() {
        let frames = |stills: Vec<EditStill>| stills.iter().map(|still| (still.picture.as_ref().map(|p| p.source_frame), still.cut)).collect::<Vec<_>>();
        assert_eq!(frames(stills("clip-c", (300, 450), Some(true), true, false, &clip(false), 600, None)), [(Some(299), false), (Some(300), true)], "before IN, IN first");
        assert_eq!(frames(stills("clip-c", (300, 450), Some(false), true, false, &clip(false), 600, None)), [(Some(449), true), (Some(450), false)], "OUT last, after OUT");
    }

    /// User 2026-11-25: "Enter snima, ali ne zatvara prozor": the zoom stays open on the side last
    /// taken after the mark is let go (no cut then), until Esc or Close or another clip.
    #[test]
    fn the_zoom_stays_open_on_its_side_after_enter_until_it_is_closed() {
        let mut zoom = SourceZoom::default();
        assert_eq!(zoom.side(None, Some("a")), None, "nothing taken: closed");
        assert_eq!(zoom.side(Some(false), Some("a")), Some(Some(false)));
        assert_eq!(zoom.side(None, Some("a")), Some(Some(false)), "let go: it stays on OUT");
        assert!(stills("clip-x", (10, 40), Some(false), false, false, &clip(true), 60, None).iter().all(|still| !still.cut), "no mark taken: no cut");
        assert!(zoom.close());
        assert_eq!(zoom.side(None, Some("a")), None, "closed; nothing opens it again until a mark is taken");
        zoom.side(Some(true), Some("a"));
        assert_eq!(zoom.side(None, Some("b")), None, "another clip closes it");
    }

    /// User 2026-11-26: a right click inside the slot opens the zoom with nothing taken: IN, the
    /// player (active, framed, at the playhead where it stands) and OUT; a click on the left or
    /// right picture takes IN or OUT, and the zoom keeps its three windows while it is open.
    #[test]
    fn a_slots_zoom_opens_with_the_player_active_and_the_playhead_where_it_is() {
        let mut zoom = SourceZoom::default();
        zoom.open_slot("a");
        assert_eq!(zoom.side(None, Some("a")), Some(None), "open, nothing taken");
        let shown = stills("a", (300, 450), None, false, true, &clip(false), 600, Some(377));
        let row = |stills: &[EditStill]| stills.iter().map(|still| (still.picture.as_ref().map(|p| p.source_frame), still.middle, still.cut)).collect::<Vec<_>>();
        assert_eq!(row(&shown), [(Some(300), false, false), (Some(377), true, true), (Some(449), false, false)], "the player at the playhead, active");
        let clicks: Vec<_> = shown.iter().map(|still| still.click.as_ref().map(|click| click.action.clone())).collect();
        assert_eq!((clicks[0].as_deref(), clicks[2].as_deref()), (Some("select_mark_in"), Some("select_mark_out")));
        // OUT taken by its picture: three windows still, OUT the cut, the player no longer framed.
        assert_eq!(zoom.side(Some(false), Some("a")), Some(Some(false)));
        let taken = stills("a", (300, 450), Some(false), true, true, &clip(false), 600, Some(449));
        assert_eq!(row(&taken), [(Some(300), false, false), (Some(449), true, false), (Some(449), false, true)]);
        assert!(zoom.close());
        assert!(!zoom.slot, "closed: the next zoom is the timeline's own again");
    }

    /// User 2026-11-25: Ctrl+H opens the clip's history, Up/Down light a row, Enter puts IN and OUT
    /// of that entry back and takes the side the zoom shows (Enter then writes it).
    #[test]
    fn a_history_entry_puts_in_and_out_back_and_takes_the_side() {
        let mut zoom = SourceZoom::default();
        zoom.side(Some(false), Some("clip-x"));
        let entry = |id: i64, at: u64| qnc_clip_marks::Entry { id, clip_id: "clip-x".into(), in_frame: at, out_frame: at + 30, back: false, author: String::new(), made_at_utc: String::new() };
        zoom.entries = vec![entry(1, 10), entry(2, 20)];
        let preview = SourcePreview::new();
        let mut timeline = TimelineProjection { duration_frames: 100, ..Default::default() };
        assert_eq!(zoom.history_key("step_next_part", None, &preview, &mut timeline), None, "the list closed: Down is not the history's");
        assert_eq!(zoom.history_key(qnc_edit_verbs::HISTORY_OPEN, None, &preview, &mut timeline), Some(true));
        assert_eq!(zoom.history_key("step_next_part", None, &preview, &mut timeline), Some(true), "the older entry lit");
        assert_eq!(zoom.history_key("activate_focused_item", None, &preview, &mut timeline), Some(true));
        assert_eq!((timeline.source_in_frame, timeline.source_out_frame), (Some(10), Some(40)));
        assert_eq!(timeline.taken_source_mark(), Some(false), "OUT, the side the zoom shows, taken");
        assert!(zoom.list.is_none(), "the list closes");
    }
}
