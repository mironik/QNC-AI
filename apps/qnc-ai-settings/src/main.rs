//! QNC-AI Settings (user 2026-11-11: "iz QNC project izdvoji Postavke kao samostalni obrazac, u
//! trayu dodaj link Postavke; to se veoma rijetko koristi zato treba biti odvojen od wizarda"): the
//! settings of QNC's Project form on their own (copied `qnc-project-desktop` in its settings-only
//! way), in the QNC-AI root. One copy runs.

#![windows_subsystem = "windows"]

use eframe::egui;

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-ai-settings");
    let Ok(Some(_one)) = qnc_tray::OneCopy::take("QNC-AI-settings") else { return Ok(()) };
    let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    // The projects' folder shown is never the bases' folder.
    if let Err(error) = qnc_ingest_base::keep_projects_apart(&root) {
        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("ai-settings projects folder: {error}"));
    }
    let app =match qnc_project_desktop::create_project_app(root) {
        Ok(app) => app.settings_only(),
        Err(error) => {
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("ai-settings not started: {error}"));
            return Ok(());
        }
    };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_title(qnc_wizard_text::text("settings.title")).with_inner_size([900.0, 800.0]).with_min_inner_size([700.0, 500.0]),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "QNC AI Settings",
        options,
        Box::new(move |cc| {
            let _ = qnc_project_desktop::apply_project_style(&cc.egui_ctx);
            Ok(Box::new(app))
        }),
    )
}
