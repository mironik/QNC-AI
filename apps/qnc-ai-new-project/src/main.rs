//! QNC-AI New project (user 2026-11-12: "kad na ingestu korisnik klikne UVEZI, ingest se
//! zatvara, a Novi Projekt se otvara; kad se popunjena forma Novi Projekt zatvori, otvara se AI
//! Media Assist"). QNC's new project sheet (`qnc-newsroom-sheet`) in a window of its own: the
//! piece `qnc-story-start` looks at the Ingest base Ingest worked in; a base without its story
//! shows the sheet, Create makes the story's project (it takes the base) and AI Media Assist
//! opens; a base that has its story opens AI Media Assist at once. Cancel closes the window.

#![windows_subsystem = "windows"]

use eframe::egui;
use qnc_story_start::{Next, StoryStart};

/// The answers of the choice before the sheet (this window's own; no key).
const GO_ON: &str = "new_project_go_on";
const NEW: &str = "new_project_new";
const CANCEL: &str = "new_project_cancel";

fn log(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
}

struct NewProject {
    root: std::path::PathBuf,
    contracts: qnc_ingest_layout::IngestContracts,
    story: StoryStart,
    message: String,
    done: bool,
}

impl NewProject {
    /// AI Media Assist beside this program, in the same root.
    fn open_media_assist(&self) {
        match qnc_ai_open::open(&self.root, "qnc-media-assist-audio-ai") {
            Ok(program) => log(format!("ai-new-project opened {}", program.display())),
            Err(error) => log(format!("ai-new-project media assist not opened: {error}")),
        }
    }
}

impl eframe::App for NewProject {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let theme = qnc_ingest_layout::theme::Theme::from_contract(&self.contracts.shell);
        qnc_ingest_layout::theme::apply(ctx, &theme);
        match self.story.poll(&self.root, qnc_ui_text::text("project.create_project")) {
            Some(Next::Import) => {
                self.open_media_assist();
                self.done = true;
            }
            Some(Next::Say(message)) => {
                log(format!("ai-new-project: {message}"));
                self.message = message;
            }
            None => {}
        }
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(theme.bg)).show(ctx, |ui| {
            if !self.message.is_empty() && self.story.sheet_mut().is_none() {
                ui.label(&self.message);
            }
        });
        // A story was worked on before: go on with it or make a new one (user 2026-11-12).
        if let Some(name) = self.story.offer().map(str::to_string) {
            let text = qnc_wizard_text::text;
            let choice = qnc_choice_box::Choice {
                title: text("new_project.offer_title").to_string(),
                message: qnc_wizard_text::fill("new_project.offer_message", &[("name", &name)]),
                options: vec![
                    qnc_choice_box::ChoiceOption::new(GO_ON, qnc_wizard_text::fill("new_project.go_on", &[("name", &name)]), true),
                    qnc_choice_box::ChoiceOption::new(NEW, text("new_project.new"), false),
                    qnc_choice_box::ChoiceOption::new(CANCEL, text("new_project.cancel"), false),
                ],
                ..Default::default()
            };
            match qnc_choice_box::show(ctx, &theme, &choice).as_deref() {
                Some(GO_ON) => self.story.go_on(&self.root),
                Some(NEW) => self.story.new_story(&self.root),
                Some(_) => self.done = true,
                None => {}
            }
        }
        if let Some(answer) = self.story.sheet_mut().and_then(|sheet| qnc_newsroom_sheet::show(ctx, sheet, &theme, &self.contracts.shortcuts)) {
            let cancel = answer == qnc_newsroom_sheet::Answer::Cancel;
            self.story.answer(&self.root, answer);
            self.done |= cancel;
        }
        if self.done {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(if self.story.busy() { 50 } else { 500 }));
    }
}

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-ai-new-project");
    let Ok(Some(_one)) = qnc_tray::OneCopy::take("QNC-AI-new-project") else { return Ok(()) };
    let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let contracts = match qnc_ingest_layout::IngestContracts::load() {
        Ok(contracts) => contracts,
        Err(error) => {
            log(format!("ai-new-project not started: {error}"));
            return Ok(());
        }
    };
    let mut story = StoryStart::default();
    story.import_asked(&root);
    let app = NewProject { root, contracts, story, message: String::new(), done: false };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_title(qnc_wizard_text::text("new_project.title")).with_inner_size([900.0, 760.0]).with_min_inner_size([800.0, 600.0]),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native("QNC AI New project", options, Box::new(move |_| Ok(Box::new(app))))
}
