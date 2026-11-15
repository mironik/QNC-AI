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
}

impl eframe::App for Projects {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        if !self.shown {
            self.shown = true;
            self.front.bring(ctx);
        }
        self.front.follow(ctx);
        // A new project (user 2026-11-15: "nedostaje dugme Kreiraj novi projekt koji poziva
        // Ingest"): Ingest opens on the newest base no story took yet (its Uvezi asks for the new
        // project), or on a card or folder when every base has its story.
        // The one primary action of the window, clear to someone at the desk for the first time
        // (user 2026-11-15): a light button with dark text, in the window's own colours.
        let visuals = ctx.style().visuals.clone();
        let bar = egui::Frame::NONE.fill(visuals.panel_fill).inner_margin(egui::Margin { left: 24, right: 24, top: 20, bottom: 8 });
        let new_asked = egui::TopBottomPanel::top("qnc-ai-projects-new").frame(bar).show_separator_line(false).show(ctx, |ui| {
            let label = egui::RichText::new(qnc_wizard_text::text("projects.new")).size(15.0).color(visuals.panel_fill);
            let button = ui.add(egui::Button::new(label).fill(visuals.strong_text_color()).stroke(egui::Stroke::NONE).corner_radius(4.0).min_size(egui::vec2(180.0, 38.0)));
            if button.hovered() {
                ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
            }
            button.clicked()
        }).inner;
        if new_asked {
            self.new_project(ctx);
            return;
        }
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
        match qnc_ai_open::open(&self.root, "qnc-ingest") {
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
            Ok(Box::new(Projects { root, app, front: Default::default(), shown: false }))
        }),
    )
}

fn log(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
}
