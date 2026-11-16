//! QNC-AI administration (user 2026-11-16: "Administracija bazama će biti administratorska forma,
//! kako inače i jeste u pravim ingestima"; the journalist never uses it, nor the tray): the Ingest
//! bases of this QNC-AI root, their state and their delete (`qnc-base-admin-view`). One copy runs.

#![windows_subsystem = "windows"]

use eframe::egui;

struct Admin {
    view: qnc_base_admin_view::AdminView,
}

impl eframe::App for Admin {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| self.view.show(ui));
    }
}

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-ai-admin");
    let Ok(Some(_one)) = qnc_tray::OneCopy::take("QNC-AI-admin") else { return Ok(()) };
    let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let bar = qnc_title_bar::TitleBar::new(qnc_wizard_text::text("admin.title"), root.clone());
    let view = qnc_base_admin_view::AdminView::new(&root);
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: qnc_title_bar::frameless(
            egui::ViewportBuilder::default().with_title(qnc_wizard_text::text("admin.title")).with_inner_size([900.0, 600.0]).with_min_inner_size([600.0, 300.0]),
        ),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "QNC AI Admin",
        options,
        Box::new(move |cc| {
            let _ = qnc_project_desktop::apply_project_style(&cc.egui_ctx);
            Ok(Box::new(qnc_title_bar::Framed { bar, app: Admin { view } }))
        }),
    )
}
