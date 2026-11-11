use std::path::PathBuf;

use eframe::egui::{self, CentralPanel, Frame};
use qnc_ingest_application::{action_ids, IngestApplication, IngestIntent};

use crate::{
    layout_contract::IngestContracts,
    theme::{self, Theme},
};

pub struct IngestApp {
    contracts: IngestContracts,
    application: IngestApplication,
    host: qnc_form_host::FormHost,
    /// QNC-AI: Ingest goes to every card attached by itself, without waiting for the user.
    cards: qnc_card_autoselect::AutoSelect,
    attached: qnc_attached_cards::AttachedCards,
}

fn log(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
}

/// The Ingest application as the form host sees it.
struct Hosted<'a>(&'a mut IngestApplication);

impl qnc_form_host::FormApplication for Hosted<'_> {
    type Intent = IngestIntent;
    fn has_player(&self) -> bool { self.0.has_player() }
    fn on_player_change(&self, wake: qnc_paint_pace::Wake) { self.0.notify_on_player_change(wake) }
    fn poll(&mut self, pointed: Option<&str>) -> bool { self.0.poll(pointed) }
    fn playing(&self) -> bool { self.0.view().playback.playing() }
    fn next_repaint_delay(&self) -> Option<std::time::Duration> { self.0.next_repaint_delay() }
    fn key_scope(&self) -> Option<&'static str> { self.0.view().focus.key_scope() }
    fn intent_of(event: qnc_action_event::ActionEvent) -> IngestIntent { IngestIntent::empty(event.action_id) }
    fn dispatch(&mut self, intent: IngestIntent) -> bool { self.0.dispatch(intent).request_repaint }
}

impl IngestApp {
    pub fn new(root: PathBuf) -> Result<Self, String> {
        let contracts = IngestContracts::load()?;
        let application = IngestApplication::with_store_root(&root)?.with_focus(contracts.focus_names());
        let attached = qnc_attached_cards::AttachedCards::start(&root);
        Ok(Self { contracts, application, host: qnc_form_host::FormHost::new(), cards: qnc_card_autoselect::AutoSelect::new(Vec::new()), attached })
    }

    /// QNC-AI: the cards attached now are gone to one after another, as the user would (Ingest's
    /// browser, the card's disk, Select); a card put in later too.
    fn follow_cards(&mut self, ctx: &egui::Context) {
        match self.attached.poll() {
            Some(Ok(cards)) => self.cards.want(&cards),
            Some(Err(error)) => log(format!("ai-ingest no camera catalog: {error}")),
            None => {}
        }
        if let Some(intent) = self.cards.next(self.application.view()) {
            log(format!("ai-ingest auto {intent:?}"));
            qnc_form_host::dispatch(ctx, &mut Hosted(&mut self.application), intent);
        }
        for (serial, reason) in self.cards.take_skipped() {
            log(format!("ai-ingest card skipped serial={serial}: {reason}"));
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(if self.cards.busy() { 200 } else { 1000 }));
    }

    pub fn show_desktop(&mut self, ctx: &egui::Context, ui: &mut egui::Ui, frame: &mut qnc_board::Frame<'_>) {
        // While the new project's sheet is open its keys are the sheet's, none goes to Ingest.
        let scopes: &[&str] = if self.application.story_sheet_mut().is_some() { &[] } else { &["ingest"] };
        let keys = qnc_form_host::FormKeys { catalog: &self.contracts.shortcuts, scopes, play_action: action_ids::PLAY_PAUSE, theme: None };
        self.host.before_paint(ctx, &mut Hosted(&mut self.application), &keys);
        self.follow_cards(ctx);
        let theme = Theme::from_contract(&self.contracts.shell);
        theme::apply(ctx, &theme);
        // The view is borrowed, not copied: it holds every clip and the timeline data.
        if let Some(intent) = qnc_ingest_blocks::render_desktop(ui, frame, &self.contracts, &theme, self.application.view()) {
            qnc_form_host::dispatch(ctx, &mut Hosted(&mut self.application), intent);
        }
        // QNC-AI: the new project's sheet Uvezi opens, over the form.
        let answer = self.application.story_sheet_mut().and_then(|sheet| qnc_newsroom_sheet::show(ctx, sheet, &theme, &self.contracts.shortcuts));
        if let Some(answer) = answer {
            let action = match answer {
                qnc_newsroom_sheet::Answer::Confirm => action_ids::STORY_SHEET_CONFIRM,
                qnc_newsroom_sheet::Answer::Cancel => action_ids::STORY_SHEET_CANCEL,
            };
            qnc_form_host::dispatch(ctx, &mut Hosted(&mut self.application), IngestIntent::empty(action));
        }
    }

    pub fn footer_status(&self) -> &str {
        self.application.footer_status()
    }

    /// The shell shows or hides this surface.
    pub fn set_active(&mut self, active: bool) {
        self.application.set_active(active);
    }

    /// Uvezi has started the import: the shell may open the next application.
    pub fn take_navigation_request(&mut self) -> bool {
        self.application.take_navigation_request()
    }

    pub fn navigation_sequence(&self) -> Result<Vec<qnc_ingest_application::SequenceStep>, String> {
        self.application.navigation_sequence()
    }
}

impl eframe::App for IngestApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let theme = Theme::from_contract(&self.contracts.shell);
        theme::apply(ctx, &theme);
        CentralPanel::default()
            .frame(Frame::NONE.fill(theme.bg))
            .show(ctx, |ui| self.show_desktop(ctx, ui, &mut qnc_board::Frame::bare()));
    }
}
