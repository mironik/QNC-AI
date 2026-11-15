//! The Media Assist and Story form on the desktop board (rule 11: a form is only a layout
//! grid of panels). Each place gets its public panel with the editorial layout's measures and
//! texts and the state of the editorial view; the panel's answer becomes an editorial intent.

use eframe::egui::{self, Align, Layout, Rect, Ui};

use qnc_editorial_application::{EditorialIntent, EditorialView, LibraryTab};
use qnc_editorial_layout::{theme::Theme, EditorialContracts};
use qnc_media_card::CardGridAction;
use {qnc_panel_focus::Panel, qnc_ui_kit::paint_focus};
use qnc_timeline::TimelineIntent;

/// Media Assist and Story on the one desktop board: preview in the monitor, pool tabs and
/// transport in the head row, the clip cards in the body, the group's right panel (Segmenti
/// for Story) on the right, the source timeline in the dock; drawn in `frame` as one layout
/// tree with it.
pub fn render_desktop(
    ui: &mut Ui,
    frame: &mut qnc_board::Frame<'_>,
    contracts: &EditorialContracts,
    theme: &Theme,
    view: &EditorialView,
) -> Option<EditorialIntent> {
    // The statements' column only while there are statements (user 2026-11-14).
    let any_statement = view.shorts.iter().any(|shot| !shot.b_roll);
    // The panels as the user dragged their borders (a view choice of this window, user 2026-11-14).
    let sizes_id = egui::Id::new(("qnc-editorial-panels", contracts.group.as_str()));
    let sizes: qnc_editorial_layout::PanelSizes = ui.ctx().data(|data| data.get_temp(sizes_id)).unwrap_or_default();
    let (faces, board) = (theme.board_faces(), contracts.board_layout_with(|id| view.tab_selected(id), any_statement, sizes));
    let framed = |intent, ui: &Ui, rect, panels: &[Panel]| { paint_focus(ui, rect, panels.contains(&view.focus.panel), theme.focus); intent };
    let placed = std::cell::RefCell::new(std::collections::HashMap::<String, Rect>::new());
    let mut place = |ui: &mut Ui, block: &str, rect: Rect| { placed.borrow_mut().insert(block.to_string(), rect); qnc_dev_diagnostics::timed_block(block, || match block {
        "preview" => preview(ui, rect, contracts, theme, view),
        "pool-head" => framed(pool_tabs(ui, rect, contracts, theme, view), ui, rect, &[Panel::PoolTabs]),
        "transport" => qnc_transport_panel::show(ui, rect, theme, contracts.tab_gap()).map(|action| EditorialIntent::action(&action)),
        "clip-cards" => framed(pool(ui, contracts, theme, view), ui, rect, &[Panel::Pool]),
        // The statements made in a column of their own (AI Media Assist, user 2026-11-14).
        qnc_editorial_layout::STATEMENTS_HEAD => {
            let title = contracts.statements_pool().map_or("", |pool| pool.title.as_str());
            qnc_pool_tabs_panel::show(ui, rect, (theme, contracts.tab_gap()), &[qnc_pool_tabs_panel::PoolTab { label: title, action_id: None }], |_| true, |_| true).map(EditorialIntent::action)
        }
        qnc_editorial_layout::STATEMENT_CARDS => framed(statements(ui, contracts, theme, view), ui, rect, &[Panel::Statements]),
        // The group's panels by its layout contract (Segmenti for Story, the transcript for Audio AI and in the MA Script media window).
        block if contracts.panel_at(block).is_some() => {
            let (answer, segments) = qnc_right_panel::show_tabbed(ui, contracts.panel_at(block)?.0, contracts.panel_at(block)?.1, theme, &qnc_right_panel::Shown { segments: &view.segments, cover_lengths: &contracts.editorial.cover_lengths, transcript: &view.focus.transcript, shots: &view.focus.shots, script: &view.focus.script, timecode: view.source_timeline.timecode.as_ref(), ai: &view.focus.ai });
            framed(answer.map(EditorialIntent::Action), ui, rect, if segments { &[Panel::Segments] } else { &[Panel::Transcript] })
        }
        "source-dock" => framed(source_dock(ui, contracts, theme, view), ui, rect, if view.segments.zoom.is_some() { &[Panel::Segments, Panel::SourceButtons] } else { &[Panel::SourceTimeline, Panel::SourceButtons] }),
        place => qnc_action_bar_panel::show(ui, rect, theme, contracts.editorial.board.block_pad, contracts.action_bar(place), |id| view.action_enabled(id)).map(EditorialIntent::action),
    }) };
    let answer = qnc_board::show_in_frame(ui, frame, &board, |name| faces.named(name), &mut place);
    if contracts.media_overlay().is_none() {
        let placed = placed.borrow().clone();
        resize_panels(ui, theme, sizes_id, sizes, &placed);
    }
    answer.or(qnc_media_overlay::show_board(ui.ctx(), theme, contracts.media_overlay().map(|overlay| overlay.title.as_str()), contracts.media_overlay_layout(|id| view.tab_selected(id)), |name| faces.named(name), place))
}

/// The borders the user drags (user 2026-11-14: "mogucnost promjene dimenzija svih panela"):
/// between the left column and the group's panel, under the monitor and before the statements'
/// column; the sizes go into the window's memory and the next frame lays the board out with them.
fn resize_panels(ui: &mut Ui, theme: &Theme, id: egui::Id, sizes: qnc_editorial_layout::PanelSizes, placed: &std::collections::HashMap<String, Rect>) {
    let (Some(monitor), Some(panel)) = (placed.get("preview"), placed.get("right-panel")) else { return };
    let statements = (placed.get(qnc_editorial_layout::STATEMENTS_HEAD), placed.get(qnc_editorial_layout::STATEMENT_CARDS));
    let right_edge = statements.1.map_or(panel.max.x, |cards| cards.max.x);
    let (top, bottom) = (monitor.min.y, panel.max.y);
    let width = (right_edge - monitor.min.x).max(1.0);
    let (line, held) = (theme.border, theme.focus);
    let mut new = sizes;
    let x = (monitor.max.x + panel.min.x) / 2.0;
    let moved = qnc_board::drag_edge(ui, id.with("left"), Rect::from_x_y_ranges(x..=x, top..=bottom), false, line, held);
    if moved != 0.0 {
        new.left = Some(((monitor.max.x - monitor.min.x + moved) / width).clamp(0.15, 0.8));
    }
    let moved = qnc_board::drag_edge(ui, id.with("monitor"), Rect::from_x_y_ranges(monitor.min.x..=monitor.max.x, monitor.max.y..=monitor.max.y), true, line, held);
    if moved != 0.0 {
        new.monitor = Some((monitor.height() + moved).max(80.0));
    }
    if let (Some(head), Some(cards)) = statements {
        let moved = qnc_board::drag_edge(ui, id.with("statements"), Rect::from_x_y_ranges(cards.min.x..=cards.min.x, head.min.y..=cards.max.y), false, line, held);
        if moved != 0.0 {
            new.statements = Some((cards.width() - moved).max(160.0));
        }
    }
    if new != sizes {
        ui.ctx().data_mut(|data| data.insert_temp(id, new));
        ui.ctx().request_repaint();
    }
}

fn preview(ui: &mut Ui, rect: Rect, contracts: &EditorialContracts, theme: &Theme, view: &EditorialView) -> Option<EditorialIntent> {
    let label = view.current_clip_label().unwrap_or(contracts.editorial.preview.empty_label.as_str());
    let turned = qnc_segment_panel::trim_zoom(ui.ctx(), &view.segments, &contracts.editorial.cover_lengths, &qnc_theme_styles::timeline(theme)); // the trim zoom, its wheel the panel's
    let shown = view.shown_clip().map(|clip| clip as &dyn qnc_clip_row::PoolItem);
    qnc_monitor_panel::show(ui, rect, egui::Id::new(("qnc-monitor", "editorial-source")), theme, &view.preview.monitor(), shown, label);
    turned.map(EditorialIntent::Action) // the monitor gives no intent
}

fn pool_tabs(ui: &mut Ui, rect: Rect, contracts: &EditorialContracts, theme: &Theme, view: &EditorialView) -> Option<EditorialIntent> {
    let tabs: Vec<_> = contracts.pool_tabs().iter().map(|tab| qnc_pool_tabs_panel::PoolTab { label: tab.label(), action_id: tab.action_id() }).collect();
    qnc_pool_tabs_panel::show(ui, rect, (theme, contracts.tab_gap()), &tabs, |id| view.tab_selected(id), |id| view.action_enabled(id)).map(EditorialIntent::action)
}

/// The one source timeline with this group's action buttons in its header.
fn source_dock(ui: &mut Ui, contracts: &EditorialContracts, theme: &Theme, view: &EditorialView) -> Option<EditorialIntent> {
    let (rect, mut clicked) = (ui.available_rect_before_wrap(), None);
    let label = view.current_clip_label().unwrap_or(&contracts.editorial.source_dock.clip_label_fallback);
    let actions = contracts.composition().source_dock.actions_rtl.iter().map(|action| qnc_source_timeline_view::HeaderAction { label: action.label(), action_id: action.action_id(), strong: false });
    let header = |ui: &mut Ui| {
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            let (enabled, on) = (|id: &str| view.action_enabled(id), |id: &str| view.segments.action_selected(id) || view.focus.transcript.selected(id));
            clicked = qnc_source_timeline_view::header_actions(ui, theme, actions, enabled, on, view.focus.button.as_deref());
        });
    };
    // The enlarged slot under the header in place of the timeline: the buttons stay (user 2026-10-22).
    let mut zoom = qnc_subsegment_panel::or_body(qnc_segment_panel::zoom_body(&view.segments, qnc_theme_styles::timeline(theme), &contracts.editorial.cover_lengths), view.focus.script.subsegment.as_ref(), qnc_theme_styles::timeline(theme), &contracts.editorial.cover_lengths);
    let (dock, instead) = ((theme, contracts.editorial.source_dock.header_timeline_gap), zoom.as_mut().map(|zoom| zoom as &mut qnc_source_timeline_view::Instead));
    let (intent, zoomed) = qnc_source_timeline_view::show_with(ui, rect, dock, label, header, (&view.preview.timeline, &view.preview.assets), &view.source_timeline, instead);
    // A click on A1/A2 and the wheel are the source timeline's own catalog actions.
    match intent {
        TimelineIntent::None => zoomed.map(EditorialIntent::Action).or(clicked.map(EditorialIntent::action)),
        intent => qnc_source_timeline::SourceTimeline::event_of(&intent).map(EditorialIntent::Action).or(clicked.map(EditorialIntent::action)),
    }
}

/// The statements made (the shorts, no B-roll), beside the media pool; a click opens one.
fn statements(ui: &mut Ui, contracts: &EditorialContracts, theme: &Theme, view: &EditorialView) -> Option<EditorialIntent> {
    let spec = contracts.pool_spec();
    let now = qnc_media_pool_panel::PoolNow {
        chosen_id: view.chosen_shot_id.as_deref().unwrap_or(""),
        panel_focused: view.focus.panel == Panel::Statements,
        step: (view.focus.panel == Panel::Statements).then_some(view.focus.pool_step).flatten(),
        empty_message: contracts.editorial.media_card.empty_message(LibraryTab::Virtual, view.loading, &view.message),
        id_salt: "editorial_statement_grid",
    };
    let shown: Vec<_> = view.shorts.iter().filter(|shot| !shot.b_roll).collect();
    match qnc_media_pool_panel::show_removable(ui, (theme, LibraryTab::Virtual.corner()), &spec, &now, &shown)? {
        CardGridAction::Remove(id) => Some(EditorialIntent::Action(qnc_action_event::ActionEvent::key(LibraryTab::Virtual.remove_action()?).with_target(id))),
        CardGridAction::Activate(id) | CardGridAction::ToggleSelection(id) => Some(EditorialIntent::PreviewShort(id)),
    }
}

fn pool(ui: &mut Ui, contracts: &EditorialContracts, theme: &Theme, view: &EditorialView) -> Option<EditorialIntent> {
    if view.library_tab == LibraryTab::Segment {
        let rect = qnc_ui_kit::fill_block(ui, theme.bg).shrink(contracts.editorial.board.block_pad);
        let shown = ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| qnc_segment_panel::show_segment_list(ui, &view.segments, qnc_theme_styles::timeline(theme), theme.danger));
        return shown.inner.map(EditorialIntent::Action);
    }
    let cards = &contracts.editorial.media_card;
    let composition = &contracts.composition().media_card;
    let spec = contracts.pool_spec();
    // Virtual lists the shorts, B-roll the shots of the covers (v5 pool tabs).
    let (tab, shot_tab) = (view.library_tab, view.library_tab.lists_shots());
    let open_statement = view.focus.script.subsegment.as_ref().map_or(String::new(), |open| open.block_id.to_string());
    let on = |action: &str, id: String| EditorialIntent::Action(qnc_action_event::ActionEvent::key(action).with_target(id));
    let (themed, mut card_action) = ((theme, tab.corner()), None);
    let now = qnc_media_pool_panel::PoolNow {
        chosen_id: if tab == LibraryTab::Subsegment { &open_statement } else if shot_tab { view.chosen_shot_id.as_deref().unwrap_or("") } else { view.chosen_clip_id().unwrap_or("") },
        panel_focused: view.focus.panel == Panel::Pool,
        step: view.focus.pool_step,
        empty_message: cards.empty_message(view.library_tab, view.loading, &view.message),
        id_salt: "editorial_clip_grid",
    };
    let clicked = match tab {
        // A SubSegment is a virtual clip of the script (user 2026-10-29): a click opens it in place of the source timeline.
        LibraryTab::Subsegment | LibraryTab::Voice | LibraryTab::Stories | LibraryTab::AiBroll => { let (cards, action) = view.focus.tab_cards(tab, &view.clips); card_action = Some(action); qnc_media_pool_panel::show_removable(ui, themed, &spec, &now, &cards.iter().collect::<Vec<_>>()) }
        _ if shot_tab => qnc_media_pool_panel::show_removable(ui, themed, &spec, &now, &view.shorts.iter().filter(|shot| shot.b_roll == (tab == LibraryTab::Broll)).collect::<Vec<_>>()),
        _ => qnc_media_pool_panel::show_removable(ui, themed, &spec, &now, &view.focus.transcript.pool_clips(tab == LibraryTab::Transcript, &view.clips)),
    };
    Some(match clicked? { CardGridAction::Remove(id) => on(tab.remove_action()?, id), CardGridAction::Activate(id) | CardGridAction::ToggleSelection(id) if card_action.is_some() => on(card_action?, id), CardGridAction::ToggleSelection(id) if composition.ticks_several && !shot_tab => on(qnc_transcript::MARK, id), CardGridAction::Activate(id) | CardGridAction::ToggleSelection(id) => if shot_tab { EditorialIntent::PreviewShort(id) } else { EditorialIntent::PreviewClip(id) } })
}
