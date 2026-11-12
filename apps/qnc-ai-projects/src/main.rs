//! QNC-AI Projects (user 2026-11-12: "iz stare QNC forme Projects preuzeti panel koji je
//! prikazivao arhivu projekata (koji nisu obrisani), kako bi korisnik mogao nastaviti raditi na
//! projektu ako je projekt iz bilo kojih razloga bio zatvoren"; "projekti u vlasništvu
//! korisnika"): the project list of QNC's Project form on its own (copied `qnc-project-desktop`
//! in its projects-only way), only the stories the user of this computer made. A story opened goes
//! on in its newest Ingest base (`qnc-ingest-base` `open_story`) and AI Media Assist opens, as
//! after Uvezi. One copy runs.

#![windows_subsystem = "windows"]

use eframe::egui;
use qnc_project_desktop::ProjectApp;

struct Projects {
    root: std::path::PathBuf,
    app: ProjectApp,
}

impl eframe::App for Projects {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.app.update(ctx, frame);
        let Some(story) = self.app.take_opened() else { return };
        match qnc_ingest_base::open_story(&self.root, &story) {
            Ok(name) => {
                log(format!("ai-projects opened {name}"));
                match qnc_ai_open::open(&self.root, "qnc-media-assist-audio-ai") {
                    Ok(program) => log(format!("ai-projects handed over to {}", program.display())),
                    Err(error) => log(format!("ai-projects AI Media Assist not opened: {error}")),
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => log(format!("ai-projects {story} not opened: {error}")),
        }
    }
}

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-ai-projects");
    let Ok(Some(_one)) = qnc_tray::OneCopy::take("QNC-AI-projects") else { return Ok(()) };
    let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let mine = match qnc_ingest_base::my_stories(&root) {
        Ok(mine) => {
            log(format!("ai-projects the user's stories: {}", mine.len()));
            mine
        }
        Err(error) => {
            log(format!("ai-projects not started: {error}"));
            return Ok(());
        }
    };
    let app = match qnc_project_desktop::create_project_app(root.clone()) {
        Ok(app) => app.projects_only(mine),
        Err(error) => {
            log(format!("ai-projects not started: {error}"));
            return Ok(());
        }
    };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_title(qnc_wizard_text::text("projects.title")).with_inner_size([700.0, 640.0]).with_min_inner_size([500.0, 400.0]),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "QNC AI Projects",
        options,
        Box::new(move |cc| {
            let _ = qnc_project_desktop::apply_project_style(&cc.egui_ctx);
            Ok(Box::new(Projects { root, app }))
        }),
    )
}

fn log(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
}
