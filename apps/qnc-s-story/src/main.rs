const GROUP: &str = "k";

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-s-story");
    if std::env::args().any(|arg| arg == "--check-contracts") {
        match qnc_editorial_desktop::check_contracts_message(GROUP) {
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

    let root = qnc_editorial_desktop::locate_qnc_root()
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let app = qnc_editorial_desktop::EditorialApp::new(GROUP, root)
        .unwrap_or_else(|error| panic!("failed to create QNC S-Story: {error}"));
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title("QNC S-Story")
            .with_min_inner_size([1100.0, 700.0])
            .with_maximized(true),
        // Opened maximized every time: the size eframe keeps of the last window is not used.
        persist_window: false,
        ..Default::default()
    };

    eframe::run_native(
        "QNC S-Story",
        options,
        // Maximized by a command too: the builder's flag alone left the window at its default size.
        Box::new(|cc| {
            cc.egui_ctx.send_viewport_cmd(eframe::egui::ViewportCommand::Maximized(true));
            Ok(Box::new(app))
        }),
    )
}
