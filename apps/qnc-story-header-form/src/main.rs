//! The form of the story header (user 2026-11-27: "obrazac za imenovanje projekta i ostalog ne
//! smije biti direktno vezan za Script"): opened by "Idi na Script (OFF)" of AI Media Assist when
//! the active Ingest base has no story of this user. Its confirm (Create project) makes the story's
//! project, which links the base and becomes the active one, then MA Script opens and this window
//! closes; Cancel closes it and nothing is made. It paints only the header block
//! (`qnc-story-header-view`) over the header's own logic (`qnc-story-header`).

#![windows_subsystem = "windows"]

use eframe::egui;
use qnc_db_broker::ProjectDbTarget;

struct Form {
    root: std::path::PathBuf,
    target: Option<ProjectDbTarget>,
    header: qnc_story_header::StoryHeader,
    theme: Option<qnc_ingest_layout::theme::Theme>,
    /// MA Script was opened (or the form cancelled): the window closes, nothing is done again.
    finished: bool,
}

impl eframe::App for Form {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        ctx.request_repaint_after(std::time::Duration::from_millis(100));
        if self.finished {
            return;
        }
        self.header.poll(self.target.as_ref());
        if self.header.done() {
            self.finished = true;
            let line = match qnc_ai_open::open(&self.root, "qnc-ma-script") {
                Ok(program) => format!("story-header-form kept: MA Script {}", program.display()),
                Err(error) => format!("story-header-form kept, MA Script not opened: {error}"),
            };
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        let Some(theme) = &self.theme else { return };
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(theme.bg)).show(ctx, |ui| {
            let view = self.header.view();
            if !view.open && !view.saving && !view.message.is_empty() {
                ui.label(egui::RichText::new(&view.message).color(theme.danger));
            }
        });
        if let Some(event) = qnc_story_header_view::overlay(ctx, self.header.view(), theme) {
            let cancel = event.action_id == qnc_story_header::CANCEL;
            self.header.act(&event.action_id, event.target.as_deref());
            if cancel {
                self.finished = true;
                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, "story-header-form cancelled: nothing made");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

/// The database of the active project (the Ingest base the header's project takes).
fn active_target(root: &std::path::Path) -> Result<ProjectDbTarget, String> {
    let reader = qnc_work_settings::SettingsReader::from_root(root).map_err(|error| error.to_string())?;
    let settings = qnc_active_project_read::ActiveProjectReader::from_settings_reader(reader.clone()).read().map_err(|error| error.to_string())?.settings;
    ProjectDbTarget::for_project(&reader, &settings)
}

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-story-header-form");
    let Some(root) = qnc_dev_diagnostics::resolve_qnc_root() else {
        qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, "story-header-form not started: no QNC root");
        return Ok(());
    };
    let target = match active_target(&root) {
        Ok(target) => Some(target),
        Err(error) => {
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("story-header-form not started: {error}"));
            return Ok(());
        }
    };
    let theme = qnc_ingest_layout::IngestContracts::load().ok().map(|contracts| qnc_ingest_layout::theme::Theme::from_contract(&contracts.shell));
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_title("QNC AI").with_inner_size([640.0, 760.0]).with_active(true),
        persist_window: false,
        centered: true,
        ..Default::default()
    };
    eframe::run_native(
        "QNC AI story header",
        options,
        Box::new(move |cc| {
            if let Some(theme) = &theme {
                qnc_ingest_layout::theme::apply(&cc.egui_ctx, theme);
            }
            cc.egui_ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            Ok(Box::new(Form { root, target, header: qnc_story_header::StoryHeader::asking(), theme, finished: false }))
        }),
    )
}
