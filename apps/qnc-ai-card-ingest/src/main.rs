//! QNC-AI card ingest (user 2026-11-12: "info prozor Ingest proces u tijeku; kada završi šalje info
//! i daje dugme Pokreni QNC; dugme pokreće Ingest formu koja čita podatke iz globalne baze"). The
//! tray opens it when a card of a known camera is put in; the block `qnc-card-ingest` does the work
//! in the global Ingest base, this window only says where it is. One copy runs: a card put in while
//! it runs is taken by it.

#![windows_subsystem = "windows"]

use eframe::egui::{self, RichText};
use qnc_card_ingest::{CardIngest, Phase, Status};
use qnc_wizard_text::{fill, text};

fn log(line: String) {
    qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
}

struct Window {
    root: std::path::PathBuf,
    contracts: qnc_ingest_layout::IngestContracts,
    ingest: CardIngest,
    last: Option<Phase>,
}

impl eframe::App for Window {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let theme = qnc_ingest_layout::theme::Theme::from_contract(&self.contracts.shell);
        qnc_ingest_layout::theme::apply(ctx, &theme);
        let status: Status = self.ingest.poll();
        if self.last != Some(status.phase) {
            log(format!("ai-card-ingest {:?} {:?} cameras={:?}", status.phase, status.progress, status.cameras));
            self.last = Some(status.phase);
        }
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(theme.bg).inner_margin(24.0)).show(ctx, |ui| {
            let title = match status.phase {
                Phase::Done => text("card_ingest.done"),
                Phase::Waiting => text("card_ingest.waiting"),
                Phase::Selecting | Phase::Making => text("card_ingest.running"),
            };
            ui.label(RichText::new(title).size(theme.font_ui + 6.0).color(theme.text).strong());
            ui.add_space(12.0);
            if !status.cameras.is_empty() {
                ui.label(RichText::new(status.cameras.join(", ")).color(theme.text));
            }
            let progress = status.progress;
            ui.label(RichText::new(fill("card_ingest.clips", &[("clips", &progress.clips.to_string())])).color(theme.text));
            if status.phase == Phase::Selecting {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spinner();
                    ui.label(RichText::new(text("card_ingest.selecting")).color(theme.text));
                });
            }
            // One bar per function the project's settings have on (user 2026-11-12).
            ui.add_space(10.0);
            for (code, done) in progress.bars() {
                let total = progress.clips.max(1);
                ui.horizontal(|ui| {
                    ui.add_sized([110.0, 20.0], egui::Label::new(RichText::new(text(code)).color(theme.text)));
                    ui.add(egui::ProgressBar::new(done.min(total) as f32 / total as f32).desired_width(260.0).text(format!("{done} / {}", progress.clips)));
                });
            }
            if progress.failed > 0 {
                ui.add_space(6.0);
                ui.label(RichText::new(fill("card_ingest.failed", &[("failed", &progress.failed.to_string())])).color(theme.text));
            }
            ui.add_space(20.0);
            let button = egui::Button::new(RichText::new(text("card_ingest.start_qnc")).size(theme.font_ui + 2.0)).min_size(egui::vec2(200.0, 36.0));
            if ui.add_enabled(status.phase == Phase::Done, button).clicked() {
                match qnc_ai_open::open(&self.root, "qnc-ingest") {
                    Ok(program) => log(format!("ai-card-ingest opened {}", program.display())),
                    Err(error) => log(format!("ai-card-ingest QNC not opened: {error}")),
                }
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-ai-card-ingest");
    let Ok(Some(_one)) = qnc_tray::OneCopy::take("QNC-AI-card-ingest") else { return Ok(()) };
    let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let started = qnc_ingest_layout::IngestContracts::load().and_then(|contracts| Ok((contracts, CardIngest::start(&root)?)));
    let (contracts, ingest) = match started {
        Ok(started) => started,
        Err(error) => {
            log(format!("ai-card-ingest not started: {error}"));
            return Ok(());
        }
    };
    let app = Window { root, contracts, ingest, last: None };
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_title(text("card_ingest.title")).with_inner_size([540.0, 480.0]).with_resizable(false),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native("QNC AI Card ingest", options, Box::new(move |_| Ok(Box::new(app))))
}
