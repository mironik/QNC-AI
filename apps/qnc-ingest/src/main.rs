use std::env;

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-ingest");
    if env::args().any(|arg| arg == "--check-contracts") {
        match qnc_ingest_desktop::check_contracts_message() {
            Ok(message) => {
                println!("{message}");
                return Ok(());
            }
            Err(error) => {
                eprintln!("{error}");
                std::process::exit(2);
            }
        }
    }

    // The one root rule (`qnc-dev-diagnostics`): never the working directory by default.
    let qnc_root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| {
        eprintln!("{}", qnc_ui_text::text("shell.root_not_found"));
        std::process::exit(2);
    });
    let app = qnc_ingest_desktop::create_ingest_app(qnc_root)
        .unwrap_or_else(|error| panic!("failed to create Ingest app: {error}"));
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("QNC Ingest")
            .with_min_inner_size([1100.0, 700.0])
            .with_maximized(true),
        // Opened maximized every time: the size eframe keeps of the last window is not used.
        persist_window: false,
        ..Default::default()
    };

    // Maximized by a command too: the builder's flag alone left the window at its default size.
    eframe::run_native(
        "QNC Ingest",
        options,
        Box::new(|cc| {
            cc.egui_ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Maximized(true));
            Ok(Box::new(app))
        }),
    )
}
