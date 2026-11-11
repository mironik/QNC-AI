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
    root: PathBuf,
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
        Ok(Self { contracts, application, host: qnc_form_host::FormHost::new(), cards: qnc_card_autoselect::AutoSelect::new(Vec::new()), attached, root })
    }

    /// QNC-AI: the cards attached now are gone to one after another, as the user would (Ingest's
    /// browser, the card's disk, Select); a card put in later too.
    fn follow_cards(&mut self, ctx: &egui::Context) {
        match self.attached.poll() {
            Some(Ok(cards)) => {
                let serials: Vec<String> = cards.iter().map(|card| card.serial.clone()).collect();
                self.cards.want(&serials);
                let cameras = cards.into_iter().filter_map(|card| card.camera.map(|camera| (card.serial.to_ascii_lowercase(), camera))).collect();
                self.application.set_cameras(cameras);
            }
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
        let keys = qnc_form_host::FormKeys { catalog: &self.contracts.shortcuts, scopes: &["ingest"], play_action: action_ids::PLAY_PAUSE, theme: None };
        self.host.before_paint(ctx, &mut Hosted(&mut self.application), &keys);
        self.follow_cards(ctx);
        let theme = Theme::from_contract(&self.contracts.shell);
        theme::apply(ctx, &theme);
        // The view is borrowed, not copied: it holds every clip and the timeline data.
        if let Some(intent) = qnc_ingest_blocks::render_desktop(ui, frame, &self.contracts, &theme, self.application.view()) {
            qnc_form_host::dispatch(ctx, &mut Hosted(&mut self.application), intent);
        }
    }

    /// QNC-AI (user 2026-11-12): once Uvezi has handed the selection to the worker (which goes on
    /// without Ingest), the new project's window opens and Ingest closes.
    fn hand_over(&mut self, ctx: &egui::Context, root: &std::path::Path) {
        if !self.application.take_navigation_request() {
            return;
        }
        match qnc_ai_open::open(root, "qnc-ai-new-project") {
            Ok(program) => {
                log(format!("ai-ingest handed over to {}", program.display()));
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => log(format!("ai-ingest new project not opened: {error}")),
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
        let root = self.root.clone();
        self.hand_over(ctx, &root);
    }
}
