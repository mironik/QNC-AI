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
    /// Started by the tray in the background: brought above the others at once (user
    /// 2026-11-15: it opened minimised and was not seen; `qnc-window-front`).
    front: qnc_window_front::WindowFront,
    shown: bool,
    /// The height the window was last given (that of its content).
    height: f32,
}

impl eframe::App for Projects {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.shown {
            self.shown = true;
            self.front.bring(ctx);
        }
        self.front.follow(ctx);
        // Read top to bottom (user 2026-11-15): the projects, and right after the last one the
        // row of a new project (Ingest opens on the newest base no story took yet, or on a card or
        // folder when every base has its story). The window is as tall as that, no empty space.
        self.app.update(ctx, frame);
        if self.app.take_new_asked() {
            self.new_project(ctx);
            return;
        }
        let height = self.app.content_height().ceil();
        if height > 0.0 && (height - self.height).abs() > 1.0 {
            self.height = height;
            let width = ctx.input(|input| input.viewport().inner_rect.map_or(640.0, |rect| rect.width()));
            ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(width, height)));
        }
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

impl Projects {
    /// Ingest on the newest base with no story (made the one Ingest works in), or on its own.
    fn new_project(&mut self, ctx: &egui::Context) {
        match qnc_ingest_base::newest_without_story(&self.root) {
            Ok(Some(base)) => match qnc_ingest_base::open(&self.root, &base.id) {
                Ok(_) => log(format!("ai-projects new project from the base {}", base.name)),
                Err(error) => log(format!("ai-projects the base {} not opened: {error}", base.name)),
            },
            Ok(None) => log("ai-projects new project: every base has its story, Ingest takes a card or folder".into()),
            Err(error) => log(format!("ai-projects bases not read: {error}")),
        }
        match qnc_ai_open::open_for_new_story(&self.root, "qnc-ingest") {
            Ok(program) => {
                log(format!("ai-projects handed over to {}", program.display()));
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Err(error) => log(format!("ai-projects Ingest not opened: {error}")),
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
        Ok(app) => app.projects_only(mine).with_new_label(qnc_wizard_text::text("projects.new").to_string()),
        Err(error) => {
            log(format!("ai-projects not started: {error}"));
            return Ok(());
        }
    };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_title(qnc_wizard_text::text("projects.title")).with_inner_size([640.0, 360.0]).with_min_inner_size([480.0, 160.0]),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native(
        "QNC AI Projects",
        options,
        Box::new(move |cc| {
            let _ = qnc_project_desktop::apply_project_style(&cc.egui_ctx);
            Ok(Box::new(Projects { root, app, front: Default::default(), shown: false, height: 0.0 }))
        }),
    )
}

fn log(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
}
