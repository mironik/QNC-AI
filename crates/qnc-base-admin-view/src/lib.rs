//! The administrator's view of the Ingest bases (user 2026-11-16: the administration of the bases
//! is an administrator's form): a table of every base (its name, the story that took it, its clips
//! imported, whether Ingest works in it) and Delete, asked before, for a base this user may delete.
//! The list is read on its own thread (`qnc-base-admin`); the delete runs off the form's thread.

use std::path::{Path, PathBuf};

use eframe::egui::{self, RichText};
use qnc_base_admin::{Delete, IngestBase, Reader, Refusal};
use qnc_wizard_text::{fill, text};

pub struct AdminView {
    root: PathBuf,
    reader: Reader,
    /// The base asked about ("delete it?").
    asking: Option<IngestBase>,
    deleting: Option<Delete>,
    /// What the last delete said, and whether it went well.
    said: Option<(String, bool)>,
}

/// The keys this view asks of its form's look.
pub const LOOK_KEYS: &[&str] = &["title_size", "header_gap", "message_gap", "button_width", "button_height", "column_gap", "row_gap"];

impl AdminView {
    pub fn new(root: &Path) -> Self {
        Self { root: root.to_path_buf(), reader: Reader::start(root), asking: None, deleting: None, said: None }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, look: &qnc_form_look::FormLook) {
        use qnc_form_look::Grows;
        let button = egui::vec2(look.get("button_width"), look.get("button_height"));
        self.reader.poll();
        if let Some(result) = self.deleting.as_ref().and_then(Delete::poll) {
            let label = self.deleting.take().map(|delete| delete.label).unwrap_or_default();
            self.said = Some(match result {
                Ok(()) => (fill("tray.base_deleted", &[("base", &label)]), true),
                Err(error) => (fill("tray.base_delete_failed", &[("base", &label), ("error", &error)]), false),
            });
        }
        ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
        ui.horizontal(|ui| {
            let title = ui.label(RichText::new(text("admin.title")).size(look.get("title_size")).strong());
            look.mark(ui, title.rect, "title_size", Grows::Up);
            // The settings are the administrator's (user 2026-11-16: "admin upravlja postavkama").
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.add(qnc_ui_kit::sized_button(text("admin.settings"), button)).clicked() {
                    if let Err(error) = qnc_ai_open::open(&self.root, "qnc-ai-settings") {
                        self.said = Some((error, false));
                    }
                }
            });
        });
        look.space(ui, "header_gap");
        if let Some(delete) = &self.deleting {
            ui.label(fill("tray.base_deleting", &[("base", &delete.label)]));
        }
        if let Some((line, ok)) = &self.said {
            let colour = if *ok { ui.visuals().text_color() } else { ui.visuals().error_fg_color };
            ui.colored_label(colour, line);
        }
        if let Some(base) = self.asking.clone() {
            ui.label(fill("tray.delete_base_ask", &[("base", &base.shown())]));
            ui.horizontal(|ui| {
                if ui.add(qnc_ui_kit::sized_button(text("admin.yes"), button)).clicked() {
                    self.deleting = Some(Delete::start(&self.root, &base));
                    (self.asking, self.said) = (None, None);
                }
                if ui.add(qnc_ui_kit::sized_button(text("admin.cancel"), button)).clicked() {
                    self.asking = None;
                }
            });
            look.space(ui, "message_gap");
        }
        let rows = match self.reader.last() {
            None => {
                ui.label(text("admin.reading"));
                return;
            }
            Some(Err(error)) => {
                ui.colored_label(ui.visuals().error_fg_color, error);
                return;
            }
            Some(Ok(rows)) if rows.is_empty() => {
                ui.label(text("admin.no_bases"));
                return;
            }
            Some(Ok(rows)) => rows.clone(),
        };
        let busy = self.deleting.is_some() || self.asking.is_some();
        egui::ScrollArea::vertical().show(ui, |ui| {
            let (column_gap, row_gap) = (look.get("column_gap"), look.get("row_gap"));
            let mut heads: Vec<egui::Rect> = Vec::new();
            let mut first_button = None;
            egui::Grid::new("qnc-base-admin").striped(true).spacing(egui::vec2(column_gap, row_gap)).show(ui, |ui| {
                for head in ["admin.base", "admin.clips", "admin.state"] {
                    heads.push(ui.label(RichText::new(text(head)).strong()).rect);
                }
                ui.label("");
                ui.end_row();
                for row in rows {
                    ui.label(row.base.shown());
                    ui.label(row.progress.as_ref().map_or_else(|| "?".into(), |progress| format!("{} / {}", progress.imported, progress.clips)));
                    ui.label(if row.base.active { text("admin.active") } else { String::new() });
                    let delete = ui.add_enabled(!busy && row.may_delete.is_ok(), qnc_ui_kit::sized_button(text("admin.delete"), button));
                    first_button.get_or_insert(delete.rect);
                    let delete = match &row.may_delete {
                        Err(refusal) => delete.on_disabled_hover_text(why(refusal, &row.base)),
                        Ok(()) => delete,
                    };
                    if delete.clicked() {
                        self.asking = Some(row.base.clone());
                    }
                    ui.end_row();
                }
            });
            // The gaps of the table and the size of its buttons are moved where they are: the gap
            // before each column after the first, the gap under the heads, the first button.
            for pair in heads.windows(2) {
                let at = pair[1].left();
                look.mark(ui, egui::Rect::from_x_y_ranges(at - column_gap..=at, pair[1].y_range()), "column_gap", Grows::Right);
            }
            if let Some(head) = heads.first() {
                let under = egui::Rect::from_x_y_ranges(head.left()..=heads.last().map_or(head.right(), |last| last.right()), head.bottom()..=head.bottom() + row_gap);
                look.mark(ui, under, "row_gap", Grows::Down);
            }
            if let Some(rect) = first_button {
                look.mark(ui, egui::Rect::from_x_y_ranges(rect.x_range(), rect.top()..=rect.bottom() - 4.0), "button_width", Grows::Right);
                look.mark(ui, egui::Rect::from_x_y_ranges(rect.x_range(), rect.bottom() - 4.0..=rect.bottom()), "button_height", Grows::Down);
            }
        });
    }
}

/// Why this user may not delete the base.
fn why(refusal: &Refusal, base: &IngestBase) -> String {
    match refusal {
        Refusal::NotOwner { user, computer } => fill("tray.delete_base_not_owner", &[("base", base.label()), ("user", user), ("computer", computer)]),
        Refusal::NoOwner => fill("tray.delete_base_no_owner", &[("base", base.label())]),
        Refusal::Unread(error) => fill("tray.base_delete_failed", &[("base", base.label()), ("error", error)]),
    }
}
