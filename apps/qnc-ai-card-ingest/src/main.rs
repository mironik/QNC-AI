//! QNC-AI card ingest (user 2026-11-12: "info prozor Ingest proces u tijeku; kada završi šalje info
//! i daje dugme Pokreni QNC; dugme pokreće Ingest formu koja čita podatke iz globalne baze"). The
//! tray opens it when a card of a known camera is put in; the block `qnc-card-ingest` does the work
//! in the global Ingest base, this window only says where it is. One copy runs: a card put in while
//! it runs is taken by it.

#![windows_subsystem = "windows"]

mod eject;

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
    on_top: qnc_window_front::WindowFront,
    /// A second copy started for a folder asks this one to show the browser.
    folder_asked: Option<qnc_tray::Signal>,
    /// The base this window ingests into, its name and description as typed, and as written
    /// (user 2026-11-15: "polje za upis naziva baze koju korisnik treba upisati prije zatvaranja").
    base: String,
    info: BaseInfo,
    /// The base's card (its camera and time), said in the window, not in its title (user 2026-11-16).
    label: String,
    /// The window's measures (user 2026-11-16: the look editor), and the size last given to it.
    look: qnc_form_look::FormLook,
    /// Cancel was pressed and waits for its answer; the ingest was cancelled (the window closes).
    cancel_asked: bool,
    cancelled: bool,
    /// After the cancel: whether the partial import of the base is deleted or kept (user
    /// 2026-11-17: "pitaj hoćete li obrisati ili zadržati djelomični import baze").
    delete_asked: bool,
    /// The card taken out by itself after a Cancel or the import.
    card_out: eject::CardOut,
    /// The Wizard was opened after the copy (Breaking news), once.
    desktop_opened: bool,
}

/// The name, place and description of the base, typed and written.
#[derive(Default)]
struct BaseInfo {
    title: String,
    location: String,
    description: String,
    written: (String, String, String),
    /// Closing was asked while the name was empty.
    needed: bool,
    /// When the name was asked for (the second chance).
    asked_at: Option<std::time::Instant>,
}

impl BaseInfo {
    fn of(known: qnc_ingest_base::IngestBase) -> Self {
        let written = (known.title.clone(), known.location.clone(), known.description.clone());
        Self { title: known.title, location: known.location, description: known.description, written, needed: false, asked_at: None }
    }

    /// Writes what was typed when it differs from what is written.
    fn keep(&mut self, root: &std::path::Path, base: &str) {
        let now = (self.title.trim().to_string(), self.location.trim().to_string(), self.description.trim().to_string());
        if base.is_empty() || now == self.written {
            return;
        }
        match qnc_ingest_base::set_info(root, base, &now.0, &now.1, &now.2) {
            Ok(()) => self.written = now,
            Err(error) => log(format!("ai-card-ingest base name not written: {error}")),
        }
    }

    /// The name, the place and the description fields, the name's need said when closing waited for it.
    fn show(&mut self, ui: &mut egui::Ui, theme: &qnc_ingest_layout::theme::Theme, look: &qnc_form_look::FormLook) -> bool {
        let label = |ui: &mut egui::Ui, code: &str| ui.label(RichText::new(text(code)).size(theme.font_ui + look.get("field_label_size")).color(theme.text_muted));
        label(ui, "card_ingest.base_name");
        let name = ui.add(qnc_ui_kit::text_field(&mut self.title).desired_width(f32::INFINITY).hint_text(text("card_ingest.base_name_hint")));
        if self.needed && self.title.trim().is_empty() {
            ui.label(RichText::new(text("card_ingest.base_name_needed")).color(theme.danger));
        }
        look.space(ui, "field_gap");
        label(ui, "card_ingest.base_location");
        let location = ui.add(qnc_ui_kit::text_field(&mut self.location).desired_width(f32::INFINITY).hint_text(text("card_ingest.base_location_hint")));
        look.space(ui, "field_gap");
        label(ui, "card_ingest.base_description");
        let description = ui.add(qnc_ui_kit::text_area(&mut self.description).desired_width(f32::INFINITY).desired_rows(2).hint_text(text("card_ingest.base_description_hint")));
        name.lost_focus() || location.lost_focus() || description.lost_focus()
    }
}

/// A card of the window: a slightly raised surface with a fine frame and round corners, its title
/// in small capitals, its contents on the same left edge. `button` (its text and size) stands at the
/// right end of the title's line (user 2026-11-17: Cancel on the line of "Ingest baza"); gives
/// whether it was pressed.
fn card(ui: &mut egui::Ui, theme: &qnc_ingest_layout::theme::Theme, look: &qnc_form_look::FormLook, title: &str, button: Option<(String, egui::Vec2)>, contents: impl FnOnce(&mut egui::Ui)) -> bool {
    let padding = look.get("card_padding");
    let mut pressed = false;
    let shown = egui::Frame::new()
        .fill(theme.surface)
        .stroke(egui::Stroke::new(look.get("card_frame"), theme.border))
        .corner_radius(look.get("card_corner"))
        .inner_margin(egui::Margin::same(padding.round() as i8))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            // Fields and bar tracks on the card: the desktop's dark with a fine frame, not black.
            let visuals = ui.visuals_mut();
            visuals.extreme_bg_color = theme.bg;
            visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, theme.border);
            visuals.widgets.hovered.bg_stroke = egui::Stroke::new(1.0, theme.text_muted);
            let title = ui
                .horizontal(|ui| {
                    let title = ui.label(RichText::new(title.to_uppercase()).size(theme.font_ui + look.get("card_title_size")).color(theme.text_muted).strong());
                    if let Some((label, size)) = button {
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            pressed = ui.add(qnc_ui_kit::sized_button(label, size)).clicked();
                        });
                    }
                    title
                })
                .inner;
            look.mark(ui, title.rect, "card_title_size", qnc_form_look::Grows::Up);
            look.space(ui, "card_title_gap");
            ui.spacing_mut().item_spacing.y = look.get("line_gap");
            contents(ui);
        });
    // The card's inner margin is moved at its left edge.
    let rect = shown.response.rect;
    look.mark(ui, egui::Rect::from_x_y_ranges(rect.left()..=rect.left() + padding, rect.y_range()), "card_padding", qnc_form_look::Grows::Right);
    pressed
}

/// The signal a copy started for a folder gives the copy that runs.
const FOLDER_SIGNAL: &str = "QNC-AI-card-ingest-folder";

/// The measures this window asks of its look file.
const LOOK_KEYS: &[&str] = &[
    "window_width", "window_height", "margin_left", "margin_right", "margin_top", "margin_bottom", "browser_margin",
    "title_size", "subtitle_size", "header_gap", "card_gap", "card_padding", "card_corner", "card_frame", "card_title_size",
    "card_title_gap", "line_gap", "bars_gap", "after_bars_gap", "bar_row", "bar_name_at", "bar_track_from", "bar_track_to", "bar_count_at", "bar_time_at", "bar_x_at", "bar_x",
    "bar_thickness", "field_label_size", "field_gap", "cancel_width",
];

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
        // The look editor beside the window, and the window's size as the look says.
        self.look.editor(ctx);
        self.look.window(ctx, qnc_title_bar::HEIGHT);
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
        // The window closes once the base has its name (user 2026-11-15); what was typed is written
        // as it closes. Without one the user is asked once more (user 2026-11-18: "daj mu ponovnu
        // mogućnost"); a second close, or `NAME_WAIT` after the ask, names the base by its camera,
        // date and time.
        if self.info.needed && self.info.title.trim().is_empty() {
            if self.info.asked_at.is_some_and(|at| at.elapsed() >= NAME_WAIT) {
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            ctx.request_repaint_after(std::time::Duration::from_secs(1));
        }
        if ctx.input(|input| input.viewport().close_requested()) {
            if !self.cancelled && self.info.title.trim().is_empty() && !self.base.is_empty() {
                if self.info.asked_at.is_none() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
                    self.info.needed = true;
                    self.info.asked_at = Some(std::time::Instant::now());
                } else {
                    self.info.title = camera_and_time(&self.label);
                    log(format!("ai-card-ingest base named by its camera and time: {}", self.info.title));
                    self.info.keep(&self.root, &self.base);
                }
            } else {
                self.info.keep(&self.root, &self.base);
            }
        }
        // The card out once the import has copied it whole (when the base needs it no more); the
        // window closes when it is out, the work goes on in the background.
        self.card_out.imported(status.progress.clips, status.progress.imported, status.progress.failed);
        let card_said = self.card_out.said();
        // Breaking news on this computer (the tray's setting): the Wizard opens as soon as the card is
        // copied, the work goes on in the background (user 2026-11-18). Once a card.
        let copied = status.progress.clips > 0 && status.progress.imported > 0 && status.progress.imported + status.progress.failed >= status.progress.clips;
        if !self.cancelled && copied && !self.desktop_opened && matches!(qnc_tray_mode::read(&self.root), Ok(Some(qnc_tray_mode::Mode::Local { work: qnc_tray_mode::Work::BreakingNews }))) {
            self.desktop_opened = true;
            match qnc_wizard_open::open(&self.root) {
                Ok(opened) => log(format!("ai-card-ingest breaking news: the Wizard {opened:?}")),
                Err(error) => log(format!("ai-card-ingest breaking news: the Wizard not opened: {error}")),
            }
        }
        if !self.cancelled && self.card_out.window_closes() {
            log("ai-card-ingest the card is out: the window closes".into());
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        // While it waits to close, the window says the card is out and the work goes on (user
        // 2026-11-18: "treba dati obavijest koja traje desetak sekundi").
        if let Some((notice, left)) = self.card_out.notice() {
            egui::Area::new(egui::Id::new("card_out_notice")).order(egui::Order::Foreground).anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO).show(ctx, |ui| {
                egui::Frame::popup(ui.style()).inner_margin(egui::Margin::same(24)).show(ui, |ui| {
                    ui.set_max_width(420.0);
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new(notice).size(20.0).strong());
                        ui.add_space(8.0);
                        ui.label(qnc_wizard_text::fill("card_ingest.window_closes_in", &[("seconds", &left.to_string())]));
                    });
                });
            });
            ctx.request_repaint_after(std::time::Duration::from_millis(250));
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
            // The base gets its card index: QNC reads it as a card (user 2026-11-15).
            let (root, base) = (self.root.clone(), self.base.clone());
            std::thread::spawn(move || match qnc_ingest_base::write_card_index(&root, &base) {
                Ok(clips) => log(format!("ai-card-ingest card index of {base}: {clips} clips")),
                Err(error) => log(format!("ai-card-ingest card index of {base} not written: {error}")),
            });
            // The user worked on something else meanwhile (mail, the web): the window, minimised
            // or behind, comes back above the other applications so the end is seen (user
            // 2026-11-13). It stays on top until the user clicks or types in it (`qnc-window-front`).
            self.on_top.bring(ctx);
        } else {
            self.on_top.follow(ctx);
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
            egui::CentralPanel::default().frame(egui::Frame::NONE.fill(theme.bg).inner_margin(self.look.get("browser_margin"))).show(ctx, |ui| {
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
        // One left edge for everything, the work and the base each a card of its own (user
        // 2026-11-16: "jedan dio forme ima lijevo poravnanje, jedan izvan; panelu daj premium izgled").
        // The body scrolls inside the window's margin, so nothing runs past the bottom or the right
        // edge (user 2026-11-16: "donja margina forme ne postoji").
        let look = &self.look;
        look.panel(ctx, theme.bg, |ui| {
            use qnc_form_look::Grows;
            // Cancel while work still runs (user 2026-11-16: "dodaj odustani dugme ako korisnik hoće
            // prekinut"): right of the base's name (user 2026-11-17); without a base on a row of its
            // own under the scrolling body.
            let show_cancel = status.phase != Phase::Done && !self.cancelled;
            let cancel_size = egui::vec2(look.get("cancel_width"), theme.chrome_control_height);
            let in_base = !self.base.is_empty();
            let reserve = if show_cancel && !in_base { theme.chrome_control_height + look.get("card_gap") + ui.spacing().item_spacing.y } else { 0.0 };
            egui::ScrollArea::vertical().auto_shrink([false, false]).max_height((ui.available_height() - reserve).max(0.0)).show(ui, |ui| {
            let title = match status.phase {
                Phase::Done => text("card_ingest.done"),
                Phase::Waiting | Phase::Choosing => text("card_ingest.waiting"),
                Phase::Selecting | Phase::Making => text("card_ingest.running"),
            };
            let title = ui.label(RichText::new(title).size(theme.font_ui + look.get("title_size")).color(theme.text).strong());
            look.mark(ui, title.rect, "title_size", Grows::Up);
            let mut sub = Vec::new();
            if !self.label.is_empty() {
                sub.push(self.label.clone());
            }
            if let Some(time) = self.took.or(elapsed) {
                sub.push(fill("card_ingest.elapsed", &[("time", &clock(time))]));
            }
            if let Some(said) = &card_said {
                sub.push(said.clone());
            }
            if !sub.is_empty() {
                let sub = ui.label(RichText::new(sub.join("   ·   ")).size(theme.font_ui + look.get("subtitle_size")).color(theme.text_muted));
                look.mark(ui, sub.rect, "subtitle_size", Grows::Up);
            }
            look.space(ui, "header_gap");
            let progress = status.progress;
            card(ui, &theme, look, &text("work.title"), None, |ui| {
                // Each card on its line; one an earlier base already has says so.
                for card in &status.cards {
                    let line = match (&card.known_in, card.new) {
                        (None, _) => card.camera.clone(),
                        (Some(base), 0) => fill("card_ingest.already_imported", &[("camera", &card.camera), ("base", base)]),
                        (Some(base), new) => fill("card_ingest.already_imported_new", &[("camera", &card.camera), ("base", base), ("new", &new.to_string())]),
                    };
                    ui.label(RichText::new(line).color(theme.text).strong());
                }
                ui.label(RichText::new(fill("card_ingest.clips", &[("clips", &progress.clips.to_string())])).color(theme.text_muted));
                if status.phase == Phase::Selecting {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(RichText::new(text("card_ingest.selecting")).color(theme.text));
                    });
                }
                // One bar per function the project's settings have on (user 2026-11-12); the X
                // beside a bar stops that function, what is made stays.
                look.space(ui, "bars_gap");
                let time_of = |bar: &qnc_card_ingest::Bar| self.done_after.get(bar.function).copied().or(if bar.stopped { None } else { elapsed });
                if let Some(function) = qnc_work_progress_view::bars_marked(ui, &progress, theme.text, &time_of, look) {
                    qnc_card_ingest::stop(&self.root, function);
                }
                if progress.statements > 0 {
                    look.space(ui, "after_bars_gap");
                    ui.label(RichText::new(fill("card_ingest.statements", &[("statements", &progress.statements.to_string())])).color(theme.text_muted));
                }
                if progress.failed > 0 {
                    look.space(ui, "after_bars_gap");
                    ui.label(RichText::new(fill("card_ingest.failed", &[("failed", &progress.failed.to_string())])).color(theme.danger));
                }
            });
            // The base's name, place and description (user 2026-11-16).
            if !self.base.is_empty() {
                look.space(ui, "card_gap");
                let mut left = false;
                let cancel = card(ui, &theme, look, &text("admin.base"), show_cancel.then(|| (text("card_ingest.cancel"), cancel_size)), |ui| left = self.info.show(ui, &theme, look));
                if left {
                    self.info.keep(&self.root, &self.base);
                }
                if cancel {
                    self.cancel_asked = true;
                }
            }
            // No "Start QNC" here: the tray asks once for every base being ingested (user
            // 2026-11-14). Closing this window does not stop the work (the worker is its own program).
            });
            if show_cancel && !in_base {
                look.space(ui, "card_gap");
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.add(qnc_ui_kit::sized_button(text("card_ingest.cancel"), cancel_size)).clicked() {
                        self.cancel_asked = true;
                    }
                });
            }
        });
        if self.cancel_asked {
            let style = qnc_confirm_dialog::DialogStyle { surface: theme.surface, border: theme.border, text: theme.text, muted: theme.text_muted, accent: theme.accent, font_size: theme.font_ui, button_height: theme.chrome_control_height };
            let (title, subject, no, yes) = (text("card_ingest.cancel_title"), text("card_ingest.cancel_subject"), text("card_ingest.cancel_no"), text("card_ingest.cancel_yes"));
            match qnc_confirm_dialog::show(ctx, &style, qnc_confirm_dialog::Question { id: "card_ingest_cancel", title: &title, subject: &subject, no: &no, yes: &yes }) {
                Some(true) => {
                    // Every function that still runs stops; what is made stays, the window closes.
                    let running: Vec<&'static str> = status.progress.bars().iter().filter(|bar| !bar.stopped && bar.done < status.progress.clips).map(|bar| bar.function).collect();
                    log(format!("ai-card-ingest cancelled by the user: stops {running:?}"));
                    qnc_card_ingest::stop_all(&self.root, running);
                    // The card goes out (tried again while the stopping work still has it open).
                    self.card_out.start("cancel");
                    self.cancel_asked = false;
                    self.cancelled = true;
                    self.delete_asked = true;
                }
                Some(false) => self.cancel_asked = false,
                None => {}
            }
        } else if self.delete_asked {
            let style = qnc_confirm_dialog::DialogStyle { surface: theme.surface, border: theme.border, text: theme.text, muted: theme.text_muted, accent: theme.accent, font_size: theme.font_ui, button_height: theme.chrome_control_height };
            let (title, subject, no, yes) = (text("card_ingest.partial_title"), text("card_ingest.partial_subject"), text("card_ingest.partial_keep"), text("card_ingest.partial_delete"));
            match qnc_confirm_dialog::show(ctx, &style, qnc_confirm_dialog::Question { id: "card_ingest_partial", title: &title, subject: &subject, no: &no, yes: &yes }) {
                Some(true) => {
                    self.delete_asked = false;
                    // The delete runs in a program of its own (this one again, without a window):
                    // this window holds the base's database and must close to let it go (live
                    // 2026-11-17: the window closed as the base closed and the delete stopped).
                    match delete_in_background(&self.root, &self.base) {
                        Ok(()) => log(format!("ai-card-ingest the partial import of {} is deleted in the background", self.base)),
                        Err(error) => log(format!("ai-card-ingest base {} not deleted: {error}", self.base)),
                    }
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Some(false) => {
                    log("ai-card-ingest the partial import is kept".into());
                    self.delete_asked = false;
                    self.info.keep(&self.root, &self.base);
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                None => {}
            }
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(200));
    }
}

/// Starts this program again as `--delete-base <id>` (no window), detached from this one.
fn delete_in_background(root: &std::path::Path, base: &str) -> Result<(), String> {
    let program = std::env::current_exe().map_err(|error| error.to_string())?;
    let mut command = std::process::Command::new(program);
    command.current_dir(root).env("QNC_ROOT", root).env_remove(qnc_work_settings::PINNED_PROJECT_ENV).arg("--delete-base").arg(base);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000 | 0x0000_0008); // no window, detached
    }
    command.spawn().map(drop).map_err(|error| error.to_string())
}

/// `--delete-base <id>`: deletes the base the way the administrator's form does (its window and
/// worker let it go first) and ends.
fn delete_base(root: &std::path::Path, id: &str) {
    let base = match qnc_ingest_base::by_id(root, id) {
        Ok(base) => base,
        Err(error) => return log(format!("ai-card-ingest base {id} not found to delete: {error}")),
    };
    let delete = qnc_base_admin::Delete::start(root, &base);
    loop {
        if let Some(result) = delete.poll() {
            match result {
                Ok(()) => log(format!("ai-card-ingest base {} deleted", base.label())),
                Err(error) => log(format!("ai-card-ingest base {} not deleted: {error}", base.label())),
            }
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
    }
}

fn main() -> eframe::Result<()> {
    qnc_dev_diagnostics::log_panics("qnc-ai-card-ingest");
    if let Some(id) = std::env::args().skip_while(|arg| arg != "--delete-base").nth(1) {
        let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
        delete_base(&root, &id);
        return Ok(());
    }
    // `--folder`: the source is a folder the user picks (the tray's "Import from a folder").
    let folder = std::env::args().any(|arg| arg == "--folder");
    // `--folder-uri <uri>`: the folder or card the Ingest form chose (user 2026-11-16).
    let folder_uri = std::env::args().skip_while(|arg| arg != "--folder-uri").nth(1);
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
    // `--goes-on`: a base cancelled and kept goes on (its card put in again): what was stopped then
    // runs again (user 2026-11-17).
    if std::env::args().any(|arg| arg == "--goes-on") {
        match qnc_card_ingest::resume_all(&root) {
            Ok(()) => log(format!("ai-card-ingest base {base} goes on: nothing stopped")),
            Err(error) => log(format!("ai-card-ingest base {base} goes on, stops not cleared: {error}")),
        }
    }
    let folder_asked = qnc_tray::Signal::listen(&signal_name).map_err(|error| log(format!("ai-card-ingest folder signal: {error}"))).ok();
    let started = qnc_ingest_layout::IngestContracts::load().and_then(|contracts| {
        let ingest = match &folder_uri {
            // A card or folder the Ingest form chose: Selected without being picked again.
            Some(uri) => CardIngest::start_folder_at(&root, uri)?,
            None if folder => CardIngest::start_folder(&root)?,
            None => CardIngest::start(&root)?,
        };
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
    let known = qnc_ingest_base::all(&root).ok().and_then(|bases| bases.into_iter().find(|known| known.id == base));
    // The title says only what the window is; the card is said in the window (user 2026-11-16).
    let title = text("card_ingest.title").to_string();
    let label = known.as_ref().map(|known| known.label().to_string()).unwrap_or_default();
    let info = known.map_or_else(BaseInfo::default, BaseInfo::of);
    // Every measure of the window is in its look file; a missing one is an error, never a guess.
    let look = match qnc_form_look::FormLook::load(&root, "card-ingest", LOOK_KEYS) {
        Ok(look) => look,
        Err(error) => {
            log(format!("ai-card-ingest look not read: {error}"));
            return Ok(());
        }
    };
    let card_out = eject::CardOut::new(card.clone(), card.is_some() && eject::free_when_imported(&root, &base));
    let app = Window { root, contracts, ingest, last: None, started: None, took: None, done_after: Default::default(), on_top: Default::default(), folder_asked, base, info, label, look, cancel_asked: false, cancelled: false, delete_asked: false, card_out, desktop_opened: false };
    // `--place <n>`: the n-th ingest window open (the tray counts them); each opens down and to
    // the right of the one before, not behind it (user 2026-11-14).
    let place = std::env::args().skip_while(|arg| arg != "--place").nth(1).and_then(|n| n.parse::<u32>().ok()).unwrap_or(0) % 8;
    let step = 36.0 * place as f32;
    let options = eframe::NativeOptions {
        renderer: eframe::Renderer::Wgpu,
        viewport: qnc_title_bar::frameless(egui::ViewportBuilder::default().with_title(title.clone()).with_inner_size([app.look.get("window_width"), app.look.get("window_height") + qnc_title_bar::HEIGHT]).with_position([120.0 + step, 80.0 + step])).with_resizable(false),
        persist_window: false,
        ..Default::default()
    };
    let bar = qnc_title_bar::TitleBar::new(title.to_string(), app.root.clone());
    eframe::run_native("QNC AI Card ingest", options, Box::new(move |_| Ok(Box::new(qnc_title_bar::Framed { bar, app }))))
}

/// How long the window waits for the name once asked again before it names the base itself.
const NAME_WAIT: std::time::Duration = std::time::Duration::from_secs(30);

/// A base's name by its camera, date and time: its label without the short id at its end
/// ("Sony ILME-FX6V 2026-11-18 14:16:14 4f5c" -> "Sony ILME-FX6V 2026-11-18 14:16:14").
fn camera_and_time(label: &str) -> String {
    match label.trim().rsplit_once(' ') {
        Some((rest, id)) if id.len() == 4 && id.chars().all(|c| c.is_ascii_hexdigit()) => rest.to_string(),
        _ => label.trim().to_string(),
    }
}

#[cfg(test)]
mod name_tests {
    #[test]
    fn a_base_without_a_name_is_named_by_its_camera_date_and_time() {
        assert_eq!(super::camera_and_time("Sony ILME-FX6V 2026-11-18 14:16:14 4f5c"), "Sony ILME-FX6V 2026-11-18 14:16:14");
        assert_eq!(super::camera_and_time("Sony 2026-11-18 14:16:14"), "Sony 2026-11-18 14:16:14");
    }
}
