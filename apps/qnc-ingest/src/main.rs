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
    // Maximized, shown only once it is (block `qnc-form-window`).
    qnc_form_window::run("QNC Ingest", app)
}
