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
    /// When the work started (the card Selected), how long all of it took, and when each
    /// function was done (user 2026-11-12: "dodaj vrijeme trajanja u prikazu"); only shown.
    started: Option<std::time::Instant>,
    took: Option<std::time::Duration>,
    done_after: std::collections::HashMap<&'static str, std::time::Duration>,
    /// Brought above the other applications when the ingest ended (when), until the user is in it
    /// or goes to another window (user 2026-11-14: a click on another ingest window brings that one
    /// to the front).
    on_top: Option<std::time::Instant>,
    /// A second copy started for a folder asks this one to show the browser.
    folder_asked: Option<qnc_tray::Signal>,
}

/// The signal a copy started for a folder gives the copy that runs.
const FOLDER_SIGNAL: &str = "QNC-AI-card-ingest-folder";

/// m:ss, or h:mm:ss past an hour.
fn clock(time: std::time::Duration) -> String {
    let seconds = time.as_secs();
    match seconds / 3600 {
        0 => format!("{}:{:02}", seconds / 60, seconds % 60),
        hours => format!("{hours}:{:02}:{:02}", seconds / 60 % 60, seconds % 60),
    }
}

impl eframe::App for Window {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let theme = qnc_ingest_layout::theme::Theme::from_contract(&self.contracts.shell);
        qnc_ingest_layout::theme::apply(ctx, &theme);
        // The tray's "Import from a folder" while this window runs (a card being ingested): the
        // browser is shown here, the card's import goes on (user 2026-11-13).
        if self.folder_asked.as_ref().is_some_and(qnc_tray::Signal::fired) {
            log("ai-card-ingest a folder is asked for beside what runs".into());
            self.ingest.pick_folder();
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
        }
        let status: Status = self.ingest.poll();
        // Its base is no longer the active project (closed to be deleted): the window ends and
        // lets the base's database go.
        if status.progress.left {
            log("ai-card-ingest its base is no longer active: the window closes".into());
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        }
        if self.last != Some(status.phase) {
            log(format!("ai-card-ingest {:?} {:?} cameras={:?}", status.phase, status.progress, status.cameras));
            self.last = Some(status.phase);
        }
        if !matches!(status.phase, Phase::Waiting | Phase::Choosing) && self.started.is_none() {
            self.started = Some(std::time::Instant::now());
        }
        let elapsed = self.started.map(|started| started.elapsed());
        if status.phase == Phase::Done && self.took.is_none() {
            self.took = elapsed;
            if let Some(took) = elapsed {
                log(format!("ai-card-ingest done after {}", clock(took)));
            }
            // The user worked on something else meanwhile (mail, the web): the window, minimised
            // or behind, comes back above the other applications so the end is seen (user
            // 2026-11-13). It stays on top until the user clicks or types in it.
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(false));
            ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::AlwaysOnTop));
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Critical));
            self.on_top = Some(std::time::Instant::now());
        } else if let Some(since) = self.on_top {
            // In it (a click, a key), or another window taken (it lost the focus after a moment):
            // it is an ordinary window again.
            let (in_it, away) = ctx.input(|input| (input.pointer.any_pressed() || (input.focused && !input.keys_down.is_empty()), input.viewport().focused == Some(false)));
            if in_it || (away && since.elapsed() >= std::time::Duration::from_secs(1)) {
                ctx.send_viewport_cmd(egui::ViewportCommand::WindowLevel(egui::WindowLevel::Normal));
                ctx.send_viewport_cmd(egui::ViewportCommand::RequestUserAttention(egui::UserAttentionType::Reset));
                self.on_top = None;
            }
        }
        for bar in status.progress.bars() {
            if status.progress.clips > 0 && bar.done >= status.progress.clips {
                if let Some(elapsed) = elapsed {
                    self.done_after.entry(bar.function).or_insert(elapsed);
                }
            }
        }
        // A source that is no card: Ingest's own browser, until the user Selects a folder; Cancel
        // closes the window (user 2026-11-13).
        if status.phase == Phase::Choosing {
            let mut answer = None;
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(theme.bg).inner_margin(16.0)).show(ctx, |ui| {
                ui.label(RichText::new(text("card_ingest.choose_folder")).size(theme.font_ui + 6.0).color(theme.text).strong());
                ui.add_space(8.0);
                let rect = ui.available_rect_before_wrap();
                answer = qnc_ingest_blocks::source_browser(ui, rect, &self.contracts, &theme, self.ingest.view(), self.ingest.card_cameras());
            });
            match answer {
                Some(intent) if intent.action_id == qnc_ingest_application::action_ids::INGEST_DIR_CANCEL => {
                    // Started for a folder alone: the window ends; beside a card: back to its import.
                    if !self.ingest.cancel_folder() {
                        log("ai-card-ingest folder not picked: the window closes".into());
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                }
                Some(intent) => self.ingest.choose(intent),
                None => {}
            }
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
            return;
        }
        egui::CentralPanel::default().frame(egui::Frame::NONE.fill(theme.bg).inner_margin(24.0)).show(ctx, |ui| {
            let title = match status.phase {
                Phase::Done => text("card_ingest.done"),
                Phase::Waiting | Phase::Choosing => text("card_ingest.waiting"),
                Phase::Selecting | Phase::Making => text("card_ingest.running"),
            };
            ui.label(RichText::new(title).size(theme.font_ui + 6.0).color(theme.text).strong());
            if let Some(time) = self.took.or(elapsed) {
                ui.label(RichText::new(fill("card_ingest.elapsed", &[("time", &clock(time))])).color(theme.text));
            }
            ui.add_space(12.0);
            // Each card on its line; one an earlier base already has says so (it is not imported again).
            for card in &status.cards {
                let line = match (&card.known_in, card.new) {
                    (None, _) => card.camera.clone(),
                    (Some(base), 0) => fill("card_ingest.already_imported", &[("camera", &card.camera), ("base", base)]),
                    (Some(base), new) => fill("card_ingest.already_imported_new", &[("camera", &card.camera), ("base", base), ("new", &new.to_string())]),
                };
                ui.label(RichText::new(line).color(theme.text));
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
            // The X beside a bar stops that function when the user does not need it (user
            // 2026-11-12); what is made stays.
            for bar in progress.bars() {
                let total = progress.clips.max(1);
                ui.horizontal(|ui| {
                    ui.add_sized([110.0, 20.0], egui::Label::new(RichText::new(text(bar.text)).color(theme.text)));
                    let label = if bar.stopped {
                        fill("card_ingest.stopped", &[("done", &bar.done.to_string()), ("clips", &progress.clips.to_string())])
                    } else {
                        format!("{} / {}", bar.done, progress.clips)
                    };
                    ui.add(egui::ProgressBar::new(bar.done.min(total) as f32 / total as f32).desired_width(260.0).text(label));
                    let running = !bar.stopped && bar.done < progress.clips;
                    let x = ui.add_enabled(running, egui::Button::new("✕").min_size(egui::vec2(22.0, 20.0)));
                    if x.on_hover_text(text("card_ingest.stop_hint")).clicked() {
                        qnc_card_ingest::stop(&self.root, bar.function);
                    }
                    // How long the function took; while it runs, how long it runs.
                    let time = self.done_after.get(bar.function).copied().or(if bar.stopped { None } else { elapsed });
                    if let Some(time) = time {
                        ui.label(RichText::new(clock(time)).color(theme.text));
                    }
                });
            }
            if progress.statements > 0 {
                ui.add_space(6.0);
                ui.label(RichText::new(fill("card_ingest.statements", &[("statements", &progress.statements.to_string())])).color(theme.text));
            }
            if progress.failed > 0 {
                ui.add_space(6.0);
                ui.label(RichText::new(fill("card_ingest.failed", &[("failed", &progress.failed.to_string())])).color(theme.text));
            }
            ui.add_space(20.0);
            let button = egui::Button::new(RichText::new(text("card_ingest.start_qnc")).size(theme.font_ui + 2.0)).min_size(egui::vec2(200.0, 36.0));
            // Ready once the clips are imported and heard: the rest goes on in the background (the
            // worker is its own program; closing this window does not stop it, user 2026-11-13).
            let ready = status.phase == Phase::Done || (status.phase == Phase::Making && progress.ready_for_qnc());
            if ui.add_enabled(ready, button).clicked() {
                if status.phase != Phase::Done {
                    log("ai-card-ingest QNC started before the end: the rest goes on in the background".into());
                }                match qnc_ai_open::open(&self.root, "qnc-ingest") {
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
    // `--folder`: the source is a folder the user picks (the tray's "Import from a folder").
    let folder = std::env::args().any(|arg| arg == "--folder");
    // `--card <serial>`: the one card this window ingests, in the base the tray gave it
    // (`QNC_WORK_PROJECT`); one window per base (user 2026-11-14).
    let card = std::env::args().skip_while(|arg| arg != "--card").nth(1);
    let base = qnc_work_settings::pinned_project().unwrap_or_default();
    let (one_name, signal_name) = (format!("QNC-AI-card-ingest-{base}"), format!("{FOLDER_SIGNAL}-{base}"));
    let Ok(Some(_one)) = qnc_tray::OneCopy::take(&one_name) else {
        // A copy of this base runs: it shows the browser beside what it does.
        if folder {
            let told = qnc_tray::Signal::fire(&signal_name);
            log(format!("ai-card-ingest a copy runs: asked it for a folder ({told})"));
        }
        return Ok(());
    };
    let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let folder_asked = qnc_tray::Signal::listen(&signal_name).map_err(|error| log(format!("ai-card-ingest folder signal: {error}"))).ok();
    let started = qnc_ingest_layout::IngestContracts::load().and_then(|contracts| {
        let ingest = if folder { CardIngest::start_folder(&root)? } else { CardIngest::start(&root)? };
        Ok((contracts, match &card {
            Some(serial) => ingest.only_card(serial),
            None => ingest,
        }))
    });
    log(format!("ai-card-ingest base={base} card={card:?} folder={folder}"));
    let (contracts, ingest) = match started {
        Ok(started) => started,
        Err(error) => {
            log(format!("ai-card-ingest not started: {error}"));
            return Ok(());
        }
    };
    // Each window says its base (a window per base, user 2026-11-14).
    let label = qnc_ingest_base::all(&root).ok().and_then(|bases| bases.into_iter().find(|known| known.id == base)).map(|known| known.label().to_string());
    let title = match label {
        Some(label) => format!("{}: {label}", text("card_ingest.title")),
        None => text("card_ingest.title").to_string(),
    };
    let app = Window { root, contracts, ingest, last: None, started: None, took: None, done_after: Default::default(), on_top: None, folder_asked };
    // `--place <n>`: the n-th ingest window open (the tray counts them); each opens down and to
    // the right of the one before, not behind it (user 2026-11-14).
    let place = std::env::args().skip_while(|arg| arg != "--place").nth(1).and_then(|n| n.parse::<u32>().ok()).unwrap_or(0) % 8;
    let step = 36.0 * place as f32;
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: egui::ViewportBuilder::default().with_title(title).with_inner_size([540.0, 570.0]).with_position([120.0 + step, 80.0 + step]).with_resizable(false),
        persist_window: false,
        ..Default::default()
    };
    eframe::run_native("QNC AI Card ingest", options, Box::new(move |_| Ok(Box::new(app))))
}
