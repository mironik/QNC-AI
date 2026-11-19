
/// A panel of an editorial surface that takes the keyboard.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Panel {
    /// The tabs of the pool head (All, SubClip, B-roll, Segment).
    PoolTabs,
    Pool,
    /// v5: choosing a clip puts the keyboard on its source timeline.
    #[default]
    SourceTimeline,
    /// The action buttons in the head of the source dock.
    SourceButtons,
    Segments,
    /// The column of the statements made (AI Media Assist): a new statement takes the focus.
    Statements,
    /// The transcript text (AI Media Assist): the arrows move its cursor, its other keys are the
    /// source timeline's (user 2026-11-15: in the Tab walk).
    Transcript,
}

impl Panel {
    /// The name of the panel in a layout contract and in the focus walk.
    pub fn name(self) -> &'static str {
        match self {
            Self::PoolTabs => qnc_focus_walk::POOL_TABS,
            Self::Pool => qnc_focus_walk::CARDS,
            Self::SourceTimeline => qnc_focus_walk::SOURCE_TIMELINE,
            Self::SourceButtons => qnc_focus_walk::SOURCE_BUTTONS,
            Self::Segments => "segments",
            Self::Statements => qnc_focus_walk::STATEMENTS,
            Self::Transcript => qnc_transcript::PANEL,
        }
    }

    /// The panel a layout contract names in its `tab_order`.
    pub fn from_name(name: &str) -> Option<Self> {
        Some(match name {
            "pool_tabs" => Self::PoolTabs,
            "pool" => Self::Pool,
            "source_timeline" => Self::SourceTimeline,
            "source_buttons" => Self::SourceButtons,
            "segments" => Self::Segments,
            "statements" => Self::Statements,
            "transcript" => Self::Transcript,
            _ => return None,
        })
    }
}

/// The Tab walk of one form (user 2026-10-13: it belongs to the form, from its layout
/// contract): its panels in order, the action ids of its pool tabs and of its source
/// dock buttons, left to right.
#[derive(Debug, Clone)]
pub struct FocusLayout {
    pub order: Vec<Panel>,
    pub tabs: Vec<String>,
    pub buttons: Vec<String>,
    /// The kinds of the right panels the form has (its panel and its tabs): only their pieces
    /// are made.
    pub right_panels: Vec<String>,
    /// The pool tab the form opens on, always open to choose (its layout contract).
    pub opening_tab: Option<String>,
}

impl FocusLayout {
    pub fn new((order, tabs, buttons, right_panels): (Vec<&str>, Vec<&str>, Vec<&str>, Vec<&str>)) -> Self {
        let owned = |names: Vec<&str>| names.into_iter().map(str::to_string).collect();
        Self { order: order.into_iter().filter_map(Panel::from_name).collect(), tabs: owned(tabs), buttons: owned(buttons), right_panels: owned(right_panels), opening_tab: None }
    }

    /// The pool tab the form opens on.
    pub fn with_opening_tab(mut self, tab: Option<&str>) -> Self {
        self.opening_tab = tab.map(str::to_string);
        self
    }
}

impl Default for FocusLayout {
    /// The walk without a layout: pool, source timeline, segments.
    fn default() -> Self {
        Self { order: vec![Panel::Pool, Panel::SourceTimeline, Panel::Segments], tabs: Vec::new(), buttons: Vec::new(), right_panels: Vec::new(), opening_tab: None }
    }
}

/// A key on the media pool: the card focus moves, or Enter opens the focused card (the
/// public card grid applies it).
pub use qnc_key_list::GridStep as PoolStep;

/// What a form draws of the focus: the panel, on the source dock buttons the action of
/// the one the arrows are on, and the last key on the media pool with its number (the
/// card grid applies each number once, however often the view is read).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FocusView {
    pub panel: Panel,
    pub button: Option<String>,
    pub pool_step: Option<(PoolStep, u64)>,
    /// Where the board drew the panels (piece `qnc-panel-map`): the board writes them, Alt +
    /// the arrows read them.
    pub places: qnc_panel_map::PanelPlaces,
    /// The pool tab the form opens on, always open to choose.
    pub opening_tab: Option<String>,
    /// The transcript of the clip on the preview.
    pub transcript: std::sync::Arc<qnc_transcript::TranscriptView>,
    /// The shots of the clip on the preview (Video AI).
    pub shots: std::sync::Arc<qnc_clip_shots::ShotsView>,
    /// The script of the story (Media Assist Audio).
    pub script: std::sync::Arc<qnc_script::ScriptView>,
    /// The versions of the S-Story.
    pub versions: std::sync::Arc<qnc_skeleton_versions::VersionsView>,
    /// The AI B-roll of the project.
    pub ai_broll: std::sync::Arc<qnc_ai_broll::AiBrollView>,
    /// What the project's AI features allow: an action of a feature it has off is grey.
    pub ai: std::sync::Arc<qnc_ai_features::Gate>,
}

impl FocusView {
    /// Whether this view decides whether `action` can be done: an action of an AI feature the
    /// project has off (docs/102 step 3), or a Transcribe button.
    pub fn decides(&self, action: &str) -> bool {
        !self.ai.allows(action) || matches!(action, "transcribe_marks" | "transcribe_fast" | "transcribe_best")
    }

    /// Whether `action` it decides can be done now; `clip_chosen`: a clip is on the preview.
    pub fn enabled(&self, action: &str, clip_chosen: bool) -> bool {
        self.ai.allows(action) && (action != "transcribe_marks" || clip_chosen || self.transcript.ticked > 0)
    }

    /// The span of the compound statement shown in the clip `shown` (its first part's IN to its last
    /// part's OUT): the source timeline enlarges it as a statement (user 2026-11-19).
    pub fn compound_marks(&self, shown: Option<&str>) -> Option<(u64, u64)> {
        let (_, parts) = self.script.compound_parts.as_ref().filter(|(clip, _)| Some(clip.as_str()) == shown)?;
        Some((parts.iter().map(|part| part.0).min()?, parts.iter().map(|part| part.1).max()?))
    }

    /// The cards of a pool tab the right panel's pieces fill (SubSegment, Voice, Stories) and
    /// the action a click sends with the card's id.
    pub fn tab_cards<'a>(&'a self, tab: qnc_pool_tabs::LibraryTab, clips: &'a [qnc_clip_row::ClipRow]) -> (Vec<qnc_clip_row::TabCard<'a>>, &'static str) {
        match tab {
            qnc_pool_tabs::LibraryTab::Stories => (self.versions.cards(), qnc_skeleton_versions::OPEN),
            qnc_pool_tabs::LibraryTab::AiBroll => (self.ai_broll.cards(clips), qnc_ai_broll::OPEN),
            tab => self.script.tab_cards(tab == qnc_pool_tabs::LibraryTab::Voice, clips),
        }
    }

    /// The statements' column (AI Media Assist, user 2026-11-19: "jedan poster koji vodi na zapise u
    /// bazi"): the statements made (`shorts`, no B-roll), then one card for each compound statement
    /// of the script, with the poster of its first clip.
    pub fn statement_column<'a, S: qnc_clip_row::PoolItem>(&'a self, shorts: Vec<&'a S>, clips: &'a [qnc_clip_row::ClipRow]) -> StatementColumn<'a, S> {
        StatementColumn { shorts, compounds: self.script.pool_cards(clips), chosen_compound: self.script.compound_block.map(|id| id.to_string()) }
    }

    /// The catalog scope that reads the keys first: the source timeline's own while it (or
    /// its buttons) has the keyboard (piece `qnc-source-timeline`).
    pub fn key_scope(&self) -> Option<&'static str> {
        matches!(self.panel, Panel::SourceTimeline | Panel::SourceButtons | Panel::Transcript).then_some(qnc_source_timeline::KEY_SCOPE)
    }
}

/// The statements' column: the statements made and the compound statements of the script.
pub struct StatementColumn<'a, S> {
    shorts: Vec<&'a S>,
    compounds: Vec<qnc_clip_row::TabCard<'a>>,
    /// The compound statement shown, when one is: the chosen card.
    chosen_compound: Option<String>,
}

impl<'a, S: qnc_clip_row::PoolItem> StatementColumn<'a, S> {
    /// The chosen card: the compound statement shown, otherwise the statement `short` chosen.
    pub fn chosen<'b>(&'b self, short: Option<&'b str>) -> &'b str {
        self.chosen_compound.as_deref().or(short).unwrap_or("")
    }

    /// Its cards in order: the statements, then the compound statements.
    pub fn items(&self) -> Vec<&dyn qnc_clip_row::PoolItem> {
        let shorts = self.shorts.iter().map(|short| *short as &dyn qnc_clip_row::PoolItem);
        shorts.chain(self.compounds.iter().map(|card| card as &dyn qnc_clip_row::PoolItem)).collect()
    }

    /// What a click on card `id` asks (`remove`: its X): a statement's id to preview, or the
    /// action of a compound statement (opened from its records, or removed) or of a statement's X.
    pub fn answer(&self, id: String, remove: bool) -> Option<Result<String, qnc_action_event::ActionEvent>> {
        let compound = self.compounds.iter().any(|card| card.id == id);
        let action = match (compound, remove) {
            (false, false) => return Some(Ok(id)),
            (false, true) => qnc_pool_tabs::LibraryTab::Virtual.remove_action()?,
            (true, false) => qnc_script::SHOW_COMPOUND,
            (true, true) => qnc_pool_tabs::LibraryTab::Subsegment.remove_action()?,
        };
        Some(Err(qnc_action_event::ActionEvent::key(action).with_target(id)))
    }
}
