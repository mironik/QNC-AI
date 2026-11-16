//! The look of a form as data (user 2026-11-16: "s obzirom da ti ne znaš crtati daj mi editor za
//! svaku formu pa ću to napraviti sam"): every measure the form draws with (margins, gaps, sizes,
//! column widths) is a named value in the form's look file `contracts/ui/forms/<form>.look.json`
//! under the QNC root. The form asks its values by key; a key the file lacks is an error when the
//! file is read, never a value made up in code.
//!
//! Editing is on the form itself (user 2026-11-16: "zašto ne vučenjem ili select pa strelicama...
//! ti samo čitaš koliko je pomaknuto, kao podešavanje nivoa tona u timeline"): while this
//! computer's setting `data/form-look-editor.json` says `{"open": true}`, every part the form
//! marks (a margin, a gap, a title, a column edge) shows a faint frame under the mouse; a click
//! takes it (red), dragging it or the arrows move it (Shift: ten steps), Esc lets it go. The form
//! reads the new value in the same frame. A small window beside the form has Save (writes the
//! look file), Back (returns to what was saved), the taken part's value and all values as numbers.

use std::{
    cell::RefCell,
    path::{Path, PathBuf},
};

use eframe::egui;
use serde::{Deserialize, Serialize};

/// One measure of a form.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Value {
    pub key: String,
    /// What the editor calls it.
    pub label: String,
    pub value: f32,
    pub min: f32,
    pub max: f32,
    /// The step of the arrows and the slider (1 for pixels).
    pub step: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct File {
    form: String,
    values: Vec<Value>,
}

/// Which way a marked part grows: dragging or the arrow that way makes its value bigger.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grows {
    Right,
    Left,
    Down,
    Up,
}

impl Grows {
    fn along(self, delta: egui::Vec2) -> f32 {
        match self {
            Grows::Right => delta.x,
            Grows::Left => -delta.x,
            Grows::Down => delta.y,
            Grows::Up => -delta.y,
        }
    }
}

/// The look of one form, as saved and as being edited.
pub struct FormLook {
    form: String,
    file: PathBuf,
    values: RefCell<Vec<Value>>,
    saved: Vec<Value>,
    editing: bool,
    window: bool,
    taken: RefCell<Option<String>>,
    note: String,
}

/// The look file of `form` under `root`.
pub fn look_file(root: &Path, form: &str) -> PathBuf {
    root.join("contracts").join("ui").join("forms").join(format!("{form}.look.json"))
}

/// The setting of this computer that opens the editor on every form.
pub fn editor_setting(root: &Path) -> PathBuf {
    root.join("data").join("form-look-editor.json")
}

/// Whether this computer's setting opens the editor; a broken setting is said and keeps it shut.
pub fn editor_on(root: &Path) -> bool {
    let path = editor_setting(root);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return false;
    };
    match serde_json::from_str::<serde_json::Value>(&text) {
        Ok(setting) => setting.get("open").and_then(serde_json::Value::as_bool).unwrap_or(false),
        Err(error) => {
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("form-look setting {} unreadable: {error}", path.display()));
            false
        }
    }
}

/// Asks the form of this window to open its look editor, or shut it when open (the hidden button
/// before the title, `qnc-title-bar`). Read by [`FormLook::editor`] in the same or the next frame.
pub fn ask_editor(ctx: &egui::Context) {
    ctx.data_mut(|data| data.insert_temp(egui::Id::new("qnc-form-look-asked"), true));
}

fn take_asked(ctx: &egui::Context) -> bool {
    ctx.data_mut(|data| data.remove_temp::<bool>(egui::Id::new("qnc-form-look-asked")).unwrap_or(false))
}

fn snap(value: &mut Value) {
    value.value = ((value.value / value.step).round() * value.step).clamp(value.min, value.max);
}

impl FormLook {
    /// Reads the look of `form`; every key in `keys` must be there.
    pub fn load(root: &Path, form: &str, keys: &[&str]) -> Result<Self, String> {
        let file = look_file(root, form);
        let text = std::fs::read_to_string(&file).map_err(|error| format!("look file {}: {error}", file.display()))?;
        let read: File = serde_json::from_str(&text).map_err(|error| format!("look file {}: {error}", file.display()))?;
        if read.form != form {
            return Err(format!("look file {} is of form {}, not {form}", file.display(), read.form));
        }
        for value in &read.values {
            if value.min > value.max || !(value.min..=value.max).contains(&value.value) || value.step <= 0.0 {
                return Err(format!("look file {}: {} is out of its range", file.display(), value.key));
            }
        }
        if let Some(missing) = keys.iter().find(|key| !read.values.iter().any(|value| value.key == **key)) {
            return Err(format!("look file {} lacks {missing}", file.display()));
        }
        let editing = editor_on(root);
        Ok(Self {
            form: form.to_string(),
            file,
            saved: read.values.clone(),
            values: RefCell::new(read.values),
            editing,
            window: editing,
            taken: RefCell::new(None),
            note: String::new(),
        })
    }

    /// The value of `key` (checked present when read).
    pub fn get(&self, key: &str) -> f32 {
        self.values.borrow().iter().find(|value| value.key == key).map_or(0.0, |value| value.value)
    }

    /// Whether the form is being edited.
    pub fn editing(&self) -> bool {
        self.editing
    }

    /// Whether the values differ from what is saved.
    pub fn changed(&self) -> bool {
        *self.values.borrow() != self.saved
    }

    fn change(&self, key: &str, by: f32, snapped: bool) {
        if let Some(value) = self.values.borrow_mut().iter_mut().find(|value| value.key == key) {
            value.value = (value.value + by).clamp(value.min, value.max);
            if snapped {
                snap(value);
            }
        }
    }

    fn step_of(&self, key: &str) -> f32 {
        self.values.borrow().iter().find(|value| value.key == key).map_or(1.0, |value| value.step)
    }

    /// A part of the form drawn over `rect` whose size is `key`, growing `grows`. While editing it
    /// can be taken with a click and moved by dragging or with the arrows.
    pub fn mark(&self, ui: &egui::Ui, rect: egui::Rect, key: &str, grows: Grows) {
        if !self.editing || !rect.is_positive() {
            return;
        }
        // A thin part (a gap of a few pixels) is still easy to hit.
        let hit = rect.expand2(egui::vec2(if rect.width() < 6.0 { 3.0 } else { 0.0 }, if rect.height() < 6.0 { 3.0 } else { 0.0 }));
        let response = ui.interact(hit, ui.id().with(("qnc-form-look-mark", key, rect.min.x as i32, rect.min.y as i32)), egui::Sense::click_and_drag());
        let taken = self.taken.borrow().as_deref() == Some(key);
        if response.clicked() || response.drag_started() {
            *self.taken.borrow_mut() = Some(key.to_string());
        }
        if response.dragged() {
            self.change(key, grows.along(response.drag_delta()), false);
        }
        if response.drag_stopped() {
            self.change(key, 0.0, true);
        }
        if taken {
            let step = self.step_of(key) * if ui.input(|input| input.modifiers.shift) { 10.0 } else { 1.0 };
            let (more, less) = match grows {
                Grows::Right => (egui::Key::ArrowRight, egui::Key::ArrowLeft),
                Grows::Left => (egui::Key::ArrowLeft, egui::Key::ArrowRight),
                Grows::Down => (egui::Key::ArrowDown, egui::Key::ArrowUp),
                Grows::Up => (egui::Key::ArrowUp, egui::Key::ArrowDown),
            };
            let (up, down) = ui.input_mut(|input| (input.count_and_consume_key(egui::Modifiers::NONE, more) + input.count_and_consume_key(egui::Modifiers::SHIFT, more), input.count_and_consume_key(egui::Modifiers::NONE, less) + input.count_and_consume_key(egui::Modifiers::SHIFT, less)));
            if up + down > 0 {
                self.change(key, step * (up as f32 - down as f32), true);
            }
            if ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Escape)) {
                *self.taken.borrow_mut() = None;
            }
        }
        let painter = ui.ctx().layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("qnc-form-look-marks")));
        let red = egui::Color32::from_rgb(230, 70, 60);
        if taken {
            painter.rect_filled(rect, 0.0, red.gamma_multiply(0.25));
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.5, red), egui::StrokeKind::Inside);
            let label = self.values.borrow().iter().find(|value| value.key == key).map(|value| format!("{}  {}", value.label, value.value)).unwrap_or_default();
            painter.text(rect.left_top() + egui::vec2(2.0, -2.0), egui::Align2::LEFT_BOTTOM, label, egui::FontId::proportional(12.0), red);
        } else if response.hovered() {
            painter.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, red.gamma_multiply(0.7)), egui::StrokeKind::Inside);
        }
    }

    /// A gap of `key` (going down), marked.
    pub fn space(&self, ui: &mut egui::Ui, key: &str) {
        let top = ui.cursor().top();
        ui.add_space(self.get(key));
        let rect = egui::Rect::from_x_y_ranges(ui.max_rect().x_range(), top..=ui.cursor().top());
        self.mark(ui, rect, key, Grows::Down);
    }

    /// Opens the editor (its window, and parts of the form taken by click) or shuts both.
    fn shut_or_open(&mut self, open: bool) {
        self.window = open;
        self.editing = open;
        if !open {
            *self.taken.borrow_mut() = None;
        }
    }

    fn save(&mut self) -> Result<(), String> {
        let file = File { form: self.form.clone(), values: self.values.borrow().clone() };
        let text = serde_json::to_string_pretty(&file).map_err(|error| error.to_string())?;
        std::fs::write(&self.file, text + "\n").map_err(|error| format!("{}: {error}", self.file.display()))?;
        self.saved = self.values.borrow().clone();
        Ok(())
    }

    /// The small window beside the form while editing: Save, Back, and every value as a number.
    /// Call once per frame.
    pub fn editor(&mut self, ctx: &egui::Context) {
        if take_asked(ctx) {
            let open = !self.window;
            self.shut_or_open(open);
        }
        if !self.window {
            return;
        }
        let id = egui::ViewportId::from_hash_of(("qnc-form-look", &self.form));
        let builder = egui::ViewportBuilder::default().with_title(qnc_wizard_text::fill("form_look.title", &[("form", &self.form)])).with_inner_size([380.0, 640.0]);
        ctx.show_viewport_immediate(id, builder, |ctx, _| {
            if ctx.input(|input| input.viewport().close_requested()) {
                self.shut_or_open(false);
            }
            egui::TopBottomPanel::bottom("qnc-form-look-buttons").show(ctx, |ui| {
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if ui.add_enabled(self.changed(), egui::Button::new(qnc_wizard_text::text("form_look.save"))).clicked() {
                        // Save writes and shuts the editor (user 2026-11-16); a failed save keeps
                        // it open and says why.
                        match self.save() {
                            Ok(()) => {
                                qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, format!("form-look saved {}", self.file.display()));
                                self.note.clear();
                                self.shut_or_open(false);
                            }
                            Err(error) => self.note = error,
                        }
                    }
                    if ui.add_enabled(self.changed(), egui::Button::new(qnc_wizard_text::text("form_look.back"))).clicked() {
                        *self.values.borrow_mut() = self.saved.clone();
                    }
                });
                ui.label(qnc_wizard_text::text("form_look.hint"));
                if !self.note.is_empty() {
                    ui.label(&self.note);
                }
                ui.add_space(6.0);
            });
            egui::CentralPanel::default().show(ctx, |ui| {
                let taken = self.taken.borrow().clone();
                egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
                    egui::Grid::new("qnc-form-look-values").num_columns(2).spacing([12.0, 8.0]).striped(true).show(ui, |ui| {
                        let mut values = self.values.borrow_mut();
                        for (value, saved) in values.iter_mut().zip(&self.saved) {
                            let mut label = egui::RichText::new(&value.label);
                            if value.value != saved.value {
                                label = label.strong();
                            }
                            if taken.as_deref() == Some(value.key.as_str()) {
                                label = label.color(egui::Color32::from_rgb(230, 70, 60));
                            }
                            ui.label(label).on_hover_text(&value.key);
                            ui.add(egui::DragValue::new(&mut value.value).range(value.min..=value.max).speed(value.step as f64));
                            ui.end_row();
                        }
                    });
                });
            });
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_look_gives_its_values_and_lacks_none_of_the_keys_the_form_asks() {
        let root = std::env::temp_dir().join(format!("qnc-form-look-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("contracts/ui/forms")).unwrap();
        let file = File { form: "probe".into(), values: vec![Value { key: "margin".into(), label: "Margin".into(), value: 24.0, min: 0.0, max: 80.0, step: 1.0 }] };
        std::fs::write(look_file(&root, "probe"), serde_json::to_string(&file).unwrap()).unwrap();
        let look = FormLook::load(&root, "probe", &["margin"]).unwrap();
        assert_eq!(look.get("margin"), 24.0);
        look.change("margin", 3.4, true);
        assert_eq!(look.get("margin"), 27.0, "a move is snapped to the step");
        look.change("margin", 500.0, true);
        assert_eq!(look.get("margin"), 80.0, "never past the range");
        assert!(look.changed());
        assert!(FormLook::load(&root, "probe", &["margin", "gap"]).is_err(), "a key the file lacks is an error");
        assert!(FormLook::load(&root, "other", &[]).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
