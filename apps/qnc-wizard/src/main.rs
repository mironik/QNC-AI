//! The QNC AI wizard: the window only. What it shows and does is qnc-wizard-application, how it
//! looks qnc-wizard-board.

use std::time::Duration;

use qnc_wizard_application::WizardApplication;
use qnc_wizard_board::Look;

struct Wizard {
    application: WizardApplication,
    look: Look,
    styled: bool,
}

impl eframe::App for Wizard {
    fn update(&mut self, ctx: &eframe::egui::Context, _frame: &mut eframe::Frame) {
        if !self.styled {
            qnc_desktop_theme::apply(ctx, &self.look.theme);
            self.styled = true;
        }
        self.application.poll();
        if let Some(intent) = qnc_wizard_board::show(ctx, &self.look, &self.application.view()) {
            self.application.act(intent);
        }
        let again = if self.application.busy() { Duration::from_millis(100) } else { Duration::from_millis(500) };
        ctx.request_repaint_after(again);
    }
}

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-wizard");
    let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let look = Look::load().unwrap_or_else(|error| panic!("{error}"));
    let title = qnc_wizard_text::text("wizard.title");
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default().with_title(&title).with_inner_size([1280.0, 760.0]).with_min_inner_size([960.0, 600.0]).with_maximized(true),
        ..Default::default()
    };
    eframe::run_native(&title, options, Box::new(move |_| Ok(Box::new(Wizard { application: WizardApplication::new(root), look, styled: false }))))
}
