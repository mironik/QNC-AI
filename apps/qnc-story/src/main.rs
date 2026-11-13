const GROUP: &str = "o";

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-story");
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
        .unwrap_or_else(|error| panic!("failed to create QNC Story: {error}"));
    // Maximized, shown only once it is (block `qnc-form-window`).
    qnc_form_window::run("QNC Story", app)
}
