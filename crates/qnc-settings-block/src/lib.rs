//! The style block of QNC-AI's settings (user 2026-11-23: "forma postavke se slaže od kocaka";
//! "ne može svaka forma biti različitog stila"; "gradi kocku, ne krpaj izgled"). Every block of the
//! Settings form and every window a block opens draws through it, and through nothing else: a
//! block has no measures and no widgets of its own, so one change here changes them all.
//!
//! A [`Pen`] is the measures of the Settings form (`PanelMetrics`, its layout contract), the desktop
//! theme and the width it writes in. With it a block draws:
//! - a section (a rule, its title) and its hint, and what a write said;
//! - a row: its label on the left, the field in the middle, the button column on the right
//!   (the label on the left of its column, where a section title starts), and in the field the row's own controls ([`Row`]): a typed
//!   field, a list, a box, all one height, their text level with the row's label, a list always
//!   exactly as wide (a long choice is cut, so what follows it starts at the same place);
//! - a whole row of each kind (text, secret, list, box), a row of field specs, buttons under the
//!   button column, and a list of rows that scrolls in whole rows.

use eframe::egui::{self, RichText};
use qnc_desktop_theme::Theme;
use qnc_settings_rows::PanelMetrics;
use qnc_wizard_text::text;

/// The room a scrolled list leaves its scroll bar.
const SCROLL_BAR: f32 = 16.0;

/// What draws a settings block: the Settings form's measures, the theme and the width.
#[derive(Clone, Copy)]
pub struct Pen<'a> {
    pub metrics: &'a PanelMetrics,
    pub theme: &'a Theme,
    pub width: f32,
}

/// The controls of a row's field, each as tall as the theme's controls.
pub struct Row<'p> {
    theme: &'p Theme,
    metrics: &'p PanelMetrics,
    /// The width of the row's field.
    pub width: f32,
}

impl Row<'_> {
    /// A typed field `width` wide; gives whether it changed.
    pub fn input(&self, ui: &mut egui::Ui, value: &mut String, width: f32, hint: &str) -> bool {
        self.edit(ui, value, width, hint).changed()
    }

    /// A typed field `width` wide; gives all it answered (changed, left, Enter).
    pub fn edit(&self, ui: &mut egui::Ui, value: &mut String, width: f32, hint: &str) -> egui::Response {
        ui.add(input(value, width, self.theme).hint_text(hint))
    }

    /// A typed field whose text is hidden.
    pub fn secret(&self, ui: &mut egui::Ui, value: &mut String, width: f32) -> bool {
        ui.add(input(value, width, self.theme).password(true)).changed()
    }

    /// A list exactly `width` wide; gives the choice picked.
    pub fn choice(&self, ui: &mut egui::Ui, id: impl std::hash::Hash, width: f32, current: &str, choices: &[(String, String)]) -> Option<String> {
        choice(ui, id, width, current, choices, self.theme)
    }

    /// A box with its words after it; gives whether it flipped.
    pub fn check(&self, ui: &mut egui::Ui, on: &mut bool, words: &str) -> bool {
        check(ui, on, words, self.metrics, self.theme, self.theme.text)
    }

    /// A box with muted words after it (what it needs, what it is for).
    pub fn check_muted(&self, ui: &mut egui::Ui, on: &mut bool, words: &str) -> bool {
        check(ui, on, words, self.metrics, self.theme, self.theme.text_muted)
    }

    /// The room between two controls of the row.
    pub fn gap(&self, ui: &egui::Ui) -> f32 {
        ui.spacing().item_spacing.x
    }
}

/// The buttons of a row's button column.
pub struct Buttons<'p> {
    theme: &'p Theme,
}

impl Buttons<'_> {
    /// The main button (Save, Add); gives whether it was clicked.
    pub fn primary(&self, ui: &mut egui::Ui, words: &str, enabled: bool) -> bool {
        qnc_settings_rows::primary_btn(ui, words, enabled, self.theme).clicked()
    }

    /// A plain button (Delete, X, Add rule).
    pub fn action(&self, ui: &mut egui::Ui, words: &str) -> bool {
        qnc_settings_rows::action_btn(ui, words, self.theme).clicked()
    }

    /// A box in the button column.
    pub fn check(&self, ui: &mut egui::Ui, on: &mut bool, words: &str, metrics: &PanelMetrics) -> bool {
        check(ui, on, words, metrics, self.theme, self.theme.text)
    }
}

impl<'a> Pen<'a> {
    pub fn new(metrics: &'a PanelMetrics, theme: &'a Theme, width: f32) -> Self {
        Self { metrics, theme, width }
    }

    /// The same pen for the rows of a list that scrolls (its scroll bar has its room).
    pub fn listed(&self) -> Self {
        Self { width: self.width - SCROLL_BAR, ..*self }
    }

    /// A section's title and its hint, by their text codes (the top of a block in the Settings form).
    pub fn title(&self, ui: &mut egui::Ui, code: &str, hint: &str) {
        ui.add_space(self.metrics.section_gap);
        ui.label(RichText::new(text(code).to_uppercase()).size(self.metrics.label_font_size).strong().color(self.theme.text_muted));
        self.hint(ui, &text(hint));
    }

    /// A section: a rule, its title, then what is in it.
    pub fn section(&self, ui: &mut egui::Ui, title: &str, body: impl FnOnce(&mut egui::Ui)) {
        qnc_settings_rows::section(ui, self.width, title, self.metrics, self.theme, body);
    }

    /// Words that tell (muted, the row's font).
    pub fn hint(&self, ui: &mut egui::Ui, words: &str) {
        if !words.is_empty() {
            ui.label(RichText::new(words).size(self.metrics.label_font_size).color(self.theme.text_muted));
        }
    }

    /// What the last write said: done muted, an error in the danger colour.
    pub fn said(&self, ui: &mut egui::Ui, said: &Option<Result<String, String>>) {
        match said {
            Some(Ok(message)) => {
                ui.label(RichText::new(message).size(self.metrics.label_font_size).color(self.theme.text_muted));
            }
            Some(Err(error)) => {
                ui.label(RichText::new(error).size(self.metrics.label_font_size).color(self.theme.danger));
            }
            None => {}
        }
    }

    /// A row: `label` on the left, `field` in the middle, `buttons` in the button column.
    pub fn row(&self, ui: &mut egui::Ui, label: &str, field: impl FnOnce(&mut egui::Ui, &Row), buttons: impl FnOnce(&mut egui::Ui, &Buttons)) {
        let (theme, metrics) = (self.theme, self.metrics);
        let (label_w, field_w, button_w) = self.columns();
        let height = metrics.row_height;
        ui.horizontal(|ui| {
            ui.set_min_height(height);
            ui.set_max_width(self.width);
            ui.spacing_mut().item_spacing.x = 0.0;
            // The label starts where a section's title does, on the left of its column, in the middle of the row.
            ui.allocate_ui_with_layout(egui::vec2(label_w, height), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_width(label_w);
                ui.add(egui::Label::new(RichText::new(label).size(metrics.label_font_size).strong().color(theme.text_muted)).truncate());
            });
            ui.add_space(metrics.inline_column_gap);
            ui.allocate_ui_with_layout(egui::vec2(field_w, height), egui::Layout::left_to_right(egui::Align::Center), |ui| {
                ui.set_max_width(field_w);
                ui.spacing_mut().item_spacing.x = metrics.inline_column_gap / 2.0;
                field(ui, &Row { theme, metrics, width: field_w });
            });
            ui.add_space(metrics.inline_column_gap);
            ui.allocate_ui_with_layout(egui::vec2(button_w, height), egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.set_width(button_w);
                ui.spacing_mut().item_spacing.x = 8.0;
                buttons(ui, &Buttons { theme });
            });
        });
    }

    /// The widths of a row's label, field and button columns: the label and the buttons as the
    /// Settings form's measures name them, the field the rest.
    fn columns(&self) -> (f32, f32, f32) {
        let metrics = self.metrics;
        let (label_w, button_w) = (metrics.inline_label_width, metrics.inline_button_width);
        let field_w = (self.width - label_w - button_w - metrics.inline_column_gap * 2.0).max(metrics.field_min_width.min(self.width));
        (label_w, field_w, button_w)
    }

    /// A row of one typed field (its label by its text code); gives whether it changed.
    pub fn text(&self, ui: &mut egui::Ui, code: &str, value: &mut String, hint: &str) -> bool {
        let mut changed = false;
        self.row(ui, &text(code), |ui, row| changed = row.input(ui, value, row.width, hint), |_, _| {});
        changed
    }

    /// A field of several lines as wide as the rows (a text that is written: a description).
    pub fn area(&self, ui: &mut egui::Ui, value: &mut String, hint: &str) -> bool {
        let width = self.width.max(80.0);
        ui.horizontal(|ui| {
            ui.add(qnc_ui_kit::text_area(value).desired_width(width).desired_rows(2).hint_text(hint)).changed()
        })
        .inner
    }

    /// A row of one typed field whose text is hidden.
    pub fn secret(&self, ui: &mut egui::Ui, code: &str, value: &mut String) -> bool {
        let mut changed = false;
        self.row(ui, &text(code), |ui, row| changed = row.secret(ui, value, row.width), |_, _| {});
        changed
    }

    /// A row of one list; gives the choice picked.
    pub fn choice(&self, ui: &mut egui::Ui, label: &str, id: impl std::hash::Hash, current: &str, choices: &[(String, String)]) -> Option<String> {
        let mut picked = None;
        self.row(ui, label, |ui, row| picked = row.choice(ui, id, row.width, current, choices), |_, _| {});
        picked
    }

    /// A row of one box, its words after it; gives whether it flipped.
    pub fn check(&self, ui: &mut egui::Ui, label: &str, on: &mut bool, words: &str) -> bool {
        let mut flipped = false;
        self.row(ui, label, |ui, row| flipped = row.check_muted(ui, on, words), |_, _| {});
        flipped
    }

    /// A box in the field column, its words after it (one of a list of boxes; the label column is
    /// empty, so the boxes stand under the fields); `enabled` false: it cannot be changed.
    pub fn check_line(&self, ui: &mut egui::Ui, on: &mut bool, words: &str, enabled: bool) -> bool {
        let mut flipped = false;
        self.row(
            ui,
            "",
            |ui, _| {
                ui.add_enabled_ui(enabled, |ui| flipped = check(ui, on, words, self.metrics, self.theme, self.theme.text));
            },
            |_, _| {},
        );
        flipped
    }

    /// The heading of a group of a window's menu (upper case, muted, the row's font).
    pub fn menu_heading(&self, ui: &mut egui::Ui, words: &str) {
        ui.add_space(self.metrics.section_gap);
        ui.label(RichText::new(words.to_uppercase()).size(self.metrics.label_font_size).strong().color(self.theme.text_muted));
    }

    /// An item of a window's menu, as wide as the menu, the theme's font; gives whether it was clicked.
    pub fn menu_item(&self, ui: &mut egui::Ui, selected: bool, words: &str, hover: &str) -> bool {
        let response = ui.add_sized([self.width, self.theme.chrome_control_height], egui::SelectableLabel::new(selected, RichText::new(words).size(self.theme.font_ui)));
        let response = if hover.is_empty() { response } else { response.on_hover_text(hover) };
        response.clicked()
    }

    /// The rule between two groups of a window's menu.
    pub fn menu_rule(&self, ui: &mut egui::Ui) {
        ui.add_space(self.metrics.section_gap);
        let (y, x0) = (ui.cursor().top(), ui.min_rect().left());
        ui.painter().hline(egui::Rangef::new(x0, x0 + self.width), y, egui::Stroke::new(1.0, self.theme.border));
        ui.add_space(self.metrics.section_gap);
    }

    /// Buttons under the button column of the rows.
    pub fn buttons(&self, ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui, &Buttons)) {
        // The same columns as a row: the buttons in its button column.
        self.row(ui, "", |_, _| {}, add);
    }

    /// The height of `rows` rows.
    pub fn rows_height(&self, ui: &egui::Ui, rows: f32) -> f32 {
        rows * (self.metrics.row_height + ui.spacing().item_spacing.y)
    }

    /// A list of rows that scrolls when it is taller than `rows` whole rows, then the room of a
    /// section after it; `body` gets the pen of the list.
    pub fn list(&self, ui: &mut egui::Ui, id: impl std::hash::Hash, rows: f32, body: impl FnOnce(&mut egui::Ui, &Pen)) {
        let listed = self.listed();
        let height = self.rows_height(ui, rows.floor().max(1.0));
        egui::ScrollArea::vertical().id_salt(id).max_height(height).auto_shrink([false, true]).show(ui, |ui| body(ui, &listed));
        ui.add_space(self.metrics.section_gap);
    }

    /// Fields (`qnc-settings-form`'s field specs) as rows: a choice is a list, a number is typed (or
    /// picked when it has choices), a length is typed as `m:ss`. Gives whether a value changed.
    pub fn fields(&self, ui: &mut egui::Ui, settings: &mut serde_json::Value, fields: &[qnc_settings_form::FieldSpec], options: &qnc_settings_form::OptionSets) -> bool {
        use qnc_settings_form::FieldKind;
        let mut changed = false;
        for field in fields {
            let choices = field.options.as_ref().and_then(|name| options.get(name)).map(Vec::as_slice).unwrap_or(&[]);
            let current = value_text(settings, &field.path).unwrap_or_else(|| field.default.clone());
            self.row(
                ui,
                &field.label,
                |ui, row| {
                    let listed = field.kind == FieldKind::Choice || (matches!(field.kind, FieldKind::Number | FieldKind::Integer) && !choices.is_empty());
                    if listed {
                        if let Some(value) = row.choice(ui, &field.id, row.width, &current, choices) {
                            changed |= match field.kind {
                                FieldKind::Number | FieldKind::Integer => qnc_settings_form::set_number(settings, &field.path, &value, field.kind == FieldKind::Integer),
                                _ => set_value(settings, &field.path, serde_json::Value::from(value)),
                            };
                        }
                        return;
                    }
                    // What is typed stays as typed while it is typed (a number half written, `1:`).
                    let id = egui::Id::new((&field.id, "qnc-settings-block-typed"));
                    let shown = |value: String| match field.kind {
                        FieldKind::Duration => value.parse::<i64>().map(qnc_settings_form::minutes_seconds).unwrap_or_default(),
                        _ => value,
                    };
                    let stored = shown(current.clone());
                    let mut typed = ui.data(|data| data.get_temp::<(String, String)>(id)).filter(|(was, _)| *was == stored).map_or_else(|| stored.clone(), |(_, typed)| typed);
                    if row.input(ui, &mut typed, row.width, &field.hint) {
                        changed |= match field.kind {
                            FieldKind::Number | FieldKind::Integer => qnc_settings_form::set_number(settings, &field.path, &typed, field.kind == FieldKind::Integer),
                            FieldKind::Duration => qnc_settings_form::parse_seconds(&typed).is_some_and(|seconds| set_value(settings, &field.path, serde_json::Value::from(seconds))),
                            _ => set_value(settings, &field.path, serde_json::Value::from(typed.clone())),
                        };
                    }
                    let now = shown(value_text(settings, &field.path).unwrap_or_else(|| field.default.clone()));
                    ui.data_mut(|data| data.insert_temp(id, (now, typed)));
                },
                |_, _| {},
            );
        }
        changed
    }
}

/// A typed field as tall as the theme's controls, its text in the middle.
fn input<'t>(value: &'t mut String, width: f32, theme: &Theme) -> egui::TextEdit<'t> {
    qnc_ui_kit::row_field(value, width.max(40.0), theme.chrome_control_height)
}

fn choice(ui: &mut egui::Ui, id: impl std::hash::Hash, width: f32, current: &str, choices: &[(String, String)], theme: &Theme) -> Option<String> {
    let mut value = current.to_string();
    let shown = choices.iter().find(|(option, _)| *option == current).map_or_else(|| current.to_string(), |(_, label)| label.clone());
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = theme.chrome_control_height;
        egui::ComboBox::from_id_salt(id).width(width.max(40.0)).truncate().selected_text(shown).show_ui(ui, |ui| {
            for (option, label) in choices {
                ui.selectable_value(&mut value, option.clone(), label);
            }
        });
    });
    (value != current).then_some(value)
}

fn check(ui: &mut egui::Ui, on: &mut bool, words: &str, metrics: &PanelMetrics, theme: &Theme, colour: egui::Color32) -> bool {
    let label = RichText::new(words).size(metrics.label_font_size).color(colour);
    ui.scope(|ui| {
        ui.spacing_mut().interact_size.y = theme.chrome_control_height;
        qnc_ui_kit::check_box(ui, on, label, true).changed()
    })
    .inner
}

fn value_text(settings: &serde_json::Value, path: &str) -> Option<String> {
    match path.split('.').try_fold(settings, |value, key| value.get(key))? {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Number(number) => Some(number.to_string()),
        serde_json::Value::Bool(on) => Some(on.to_string()),
        _ => None,
    }
}

fn set_value(settings: &mut serde_json::Value, path: &str, value: serde_json::Value) -> bool {
    let keys: Vec<&str> = path.split('.').collect();
    let mut at = settings;
    for key in &keys[..keys.len() - 1] {
        if !at.is_object() {
            *at = serde_json::Value::Object(serde_json::Map::new());
        }
        let Some(map) = at.as_object_mut() else { return false };
        at = map.entry(key.to_string()).or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
    }
    if !at.is_object() {
        *at = serde_json::Value::Object(serde_json::Map::new());
    }
    at.as_object_mut().is_some_and(|map| {
        map.insert(keys[keys.len() - 1].to_string(), value);
        true
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_value_is_written_at_its_path_and_read_back_as_text() {
        let mut settings = serde_json::json!({});
        assert!(set_value(&mut settings, "export.width", serde_json::Value::from(1920)));
        assert!(set_value(&mut settings, "name", serde_json::Value::from("A")));
        assert_eq!(value_text(&settings, "export.width").as_deref(), Some("1920"));
        assert_eq!(value_text(&settings, "name").as_deref(), Some("A"));
        assert_eq!(value_text(&settings, "export.none"), None);
    }
}
