//! The QNC AI wizard's Ingest (docs/01-wizard-plan.md, step 2): QNC's Ingest as it is (its
//! application, layout and blocks, the same as QNC's own Ingest program) that goes to the cards of
//! the story by itself (`qnc-card-autoselect`). `qnc-ai-ingest --card <volume serial> ...`; the tray
//! starts it in the story's work place (`QNC_ROOT`).

use eframe::egui::{self, CentralPanel, Frame};
use qnc_card_autoselect::AutoSelect;
use qnc_ingest_application::{action_ids, IngestApplication, IngestIntent};
use qnc_ingest_layout::{theme, IngestContracts};

struct Ingest {
    contracts: IngestContracts,
    application: IngestApplication,
    host: qnc_form_host::FormHost,
    cards: AutoSelect,
}

/// The Ingest application as the form host sees it (as in QNC's Ingest form).
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

impl eframe::App for Ingest {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let theme = theme::Theme::from_contract(&self.contracts.shell);
        theme::apply(ctx, &theme);
        let keys = qnc_form_host::FormKeys { catalog: &self.contracts.shortcuts, scopes: &["ingest"], play_action: action_ids::PLAY_PAUSE, theme: None };
        self.host.before_paint(ctx, &mut Hosted(&mut self.application), &keys);
        // The cards of the story, one after another, as the user would choose them.
        if let Some(intent) = self.cards.next(self.application.view()) {
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("ai-ingest auto {intent:?}"));
            qnc_form_host::dispatch(ctx, &mut Hosted(&mut self.application), intent);
        }
        for (serial, reason) in self.cards.take_skipped() {
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("ai-ingest card skipped serial={serial}: {reason}"));
        }
        if self.cards.busy() {
            ctx.request_repaint_after(std::time::Duration::from_millis(200));
        }
        CentralPanel::default().frame(Frame::NONE.fill(theme.bg)).show(ctx, |ui| {
            if let Some(intent) = qnc_ingest_blocks::render_desktop(ui, &mut qnc_board::Frame::bare(), &self.contracts, &theme, self.application.view()) {
                qnc_form_host::dispatch(ctx, &mut Hosted(&mut self.application), intent);
            }
        });
    }
}

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-ai-ingest");
    let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| {
        eprintln!("no QNC root");
        std::process::exit(2);
    });
    let mut cards = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--card" {
            cards.extend(args.next());
        }
    }
    let contracts = IngestContracts::load().unwrap_or_else(|error| panic!("Ingest contracts: {error}"));
    let application = IngestApplication::with_store_root(&root).unwrap_or_else(|error| panic!("Ingest: {error}")).with_focus(contracts.focus_names());
    let app = Ingest { contracts, application, host: qnc_form_host::FormHost::new(), cards: AutoSelect::new(cards) };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default().with_title("QNC AI: Ingest").with_inner_size([1280.0, 760.0]).with_min_inner_size([1100.0, 700.0]).with_maximized(true),
        ..Default::default()
    };
    eframe::run_native("QNC AI: Ingest", options, Box::new(|_| Ok(Box::new(app))))
}
