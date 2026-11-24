//! An info box with a choice (user 2026-10-23: "when a project setting is in conflict with
//! the state of the card, say so in an info box and offer a solution with a choice"). It
//! says what is in the way, names what it is about, how it can be solved, and offers the
//! options as catalog actions; the caller runs the chosen one. Passive paint only: the box
//! decides and remembers nothing and knows no form, card or database.

use eframe::egui::{self, Align2, RichText, ScrollArea, Stroke, Vec2};

pub const MODULE_ID: &str = "qnc.module.choice-box";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

const WIDTH: f32 = 560.0;
const ITEMS_HEIGHT: f32 = 160.0;

/// What the box shows and its options (core: `qnc-notice`, docs/99).
pub use qnc_notice::{Choice, ChoiceOption};

/// Shows `choice` in the middle of the screen over everything; the action of the option the
/// user clicked.
pub fn show(ctx: &egui::Context, theme: &qnc_desktop_theme::Theme, choice: &Choice) -> Option<String> {
    show_option(ctx, theme, choice).map(|option| option.action_id)
}

/// Shows `choice` like `show`; the option the user clicked, with its target.
pub fn show_option(ctx: &egui::Context, theme: &qnc_desktop_theme::Theme, choice: &Choice) -> Option<ChoiceOption> {
    let mut chosen = None;
    egui::Area::new(egui::Id::new("qnc_choice_box"))
        .order(egui::Order::Foreground)
        .anchor(Align2::CENTER_CENTER, Vec2::ZERO)
        .show(ctx, |ui| {
            egui::Frame::NONE
                .fill(theme.panel)
                .stroke(Stroke::new(2.0, theme.focus))
                .corner_radius(6.0)
                .inner_margin(16.0)
                .show(ui, |ui| {
                    ui.set_width(WIDTH);
                    ui.label(RichText::new(&choice.title).color(theme.text).size(theme.font_ui + 4.0).strong());
                    ui.add_space(8.0);
                    ui.label(RichText::new(&choice.message).color(theme.text).size(theme.font_ui));
                    if !choice.items.is_empty() {
                        ui.add_space(6.0);
                        ScrollArea::vertical().max_height(ITEMS_HEIGHT).show(ui, |ui| {
                            for item in &choice.items {
                                ui.label(RichText::new(item).color(theme.text_muted).size(theme.font_ui));
                            }
                        });
                    }
                    if !choice.solution.is_empty() {
                        ui.add_space(8.0);
                        ui.label(RichText::new(&choice.solution).color(theme.text).size(theme.font_ui));
                    }
                    // The fields the user writes in (user 2026-11-19: the name and the role of who
                    // speaks), kept by the box while it is open (filled in from the question when
                    // it opens): the first takes the keyboard, Enter goes to the next, Enter in the
                    // last is the proposed option. An option that takes the fields carries them.
                    let (opened_id, values_id) = (egui::Id::new("qnc_choice_box_opened"), egui::Id::new("qnc_choice_box_values"));
                    let opened = ui.ctx().memory(|memory| memory.data.get_temp::<bool>(opened_id).unwrap_or(false));
                    let mut values: Vec<String> = match opened {
                        true => ui.ctx().memory(|memory| memory.data.get_temp(values_id)).unwrap_or_default(),
                        false => choice.fields.iter().map(|field| field.value.clone()).collect(),
                    };
                    values.resize(choice.fields.len(), String::new());
                    let mut entered = false;
                    for (index, field) in choice.fields.iter().enumerate() {
                        ui.add_space(8.0);
                        ui.label(RichText::new(&field.label).color(theme.text_muted).size(theme.font_ui));
                        let id = egui::Id::new(("qnc_choice_box_field", index));
                        let response = ui.add(qnc_ui_kit::row_field(&mut values[index], WIDTH, theme.chrome_control_height).id(id));
                        if index == 0 && !opened {
                            response.request_focus();
                        }
                        // The first letter of a field is a capital (user 2026-11-20: an auto-function).
                        if response.changed() {
                            values[index] = capital_first(&values[index]);
                        }
                        if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                            // This Enter is the field's; the next field, drawn after it in this
                            // frame, must not see it too (it would leave at once and save).
                            ui.input_mut(|input| input.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                            match index + 1 < choice.fields.len() {
                                true => ui.ctx().memory_mut(|memory| memory.request_focus(egui::Id::new(("qnc_choice_box_field", index + 1)))),
                                false => entered = true,
                            }
                        }
                    }
                    let fields: Vec<qnc_notice::ChoiceField> = choice.fields.iter().zip(&values).map(|(field, value)| qnc_notice::ChoiceField { label: field.label.clone(), value: value.clone() }).collect();
                    let with_fields = |option: &ChoiceOption| match option.takes_fields {
                        true => ChoiceOption { target: Some(qnc_notice::fields_target(&fields)), ..option.clone() },
                        false => option.clone(),
                    };
                    ui.ctx().memory_mut(|memory| {
                        memory.data.insert_temp(opened_id, true);
                        memory.data.insert_temp(values_id, values.clone());
                    });
                    if entered {
                        chosen = choice.options.iter().find(|option| option.strong).map(with_fields);
                    }
                    ui.add_space(12.0);
                    // Many options (the groups of a shot) wrap onto more lines.
                    ui.horizontal_wrapped(|ui| {
                        for option in &choice.options {
                            let clicked = if option.strong {
                                qnc_ui_kit::strong_button(ui, &option.label, true, theme.accent, theme.chrome_control_height).clicked()
                            } else {
                                qnc_ui_kit::chrome_button(ui, &option.label, true, false, qnc_theme_styles::button(theme)).clicked()
                            };
                            if clicked {
                                chosen = Some(with_fields(option));
                            }
                        }
                    });
                });
        });
    // The next question takes the keyboard into its first field again.
    if chosen.is_some() {
        ctx.memory_mut(|memory| memory.data.insert_temp(egui::Id::new("qnc_choice_box_opened"), false));
    }
    chosen
}

/// The text with its first letter a capital; the rest as written.
fn capital_first(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) if first.is_lowercase() => first.to_uppercase().chain(chars).collect(),
        _ => value.to_string(),
    }
}

#[cfg(test)]
mod capital_tests {
    #[test]
    fn the_first_letter_becomes_a_capital_the_rest_stays() {
        assert_eq!(super::capital_first("ivan kaštelan"), "Ivan kaštelan");
        assert_eq!(super::capital_first("šef"), "Šef");
        assert_eq!(super::capital_first("Ana"), "Ana");
        assert_eq!(super::capital_first(""), "");
    }
}
