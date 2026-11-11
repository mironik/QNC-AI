//! The board of the wizard: the steps on the left (done, shown, locked), the shown step in the
//! middle and Back / Next below it. Colours and measures come from the wizard's layout
//! (`contracts/wizard.layout.json`, QNC's shell colours), buttons from QNC's gui-kit. It draws
//! the View it is given and gives back what was pressed; it keeps nothing.

use eframe::egui::{self, Align, Color32, Layout, RichText, Stroke, Vec2};
use qnc_desktop_theme::{ShellLayoutContract, Theme};
use qnc_wizard_application::{Intent, View};
use serde::Deserialize;

pub const LAYOUT: &str = include_str!("../../../contracts/wizard.layout.json");

#[derive(Debug, Clone, Copy, Deserialize)]
pub struct WizardMeasures {
    pub steps_width: f32,
    pub footer_height: f32,
    pub step_row_height: f32,
    pub button_width: f32,
}

#[derive(Deserialize)]
struct Layout_ {
    wizard: WizardMeasures,
}

/// The theme and measures of the wizard.
#[derive(Debug, Clone)]
pub struct Look {
    pub theme: Theme,
    pub measures: WizardMeasures,
}

impl Look {
    pub fn load() -> Result<Self, String> {
        let shell: ShellLayoutContract = serde_json::from_str(LAYOUT).map_err(|error| format!("wizard layout: {error}"))?;
        let layout: Layout_ = serde_json::from_str(LAYOUT).map_err(|error| format!("wizard layout: {error}"))?;
        Ok(Self { theme: Theme::from_contract(&shell), measures: layout.wizard })
    }
}

/// Draws the wizard; the intent of what was pressed.
pub fn show(ctx: &egui::Context, look: &Look, view: &View) -> Option<Intent> {
    let (theme, measures) = (&look.theme, &look.measures);
    let mut intent = None;

    egui::SidePanel::left("wizard_steps")
        .exact_width(measures.steps_width)
        .resizable(false)
        .frame(egui::Frame::new().fill(theme.surface).inner_margin(12.0))
        .show(ctx, |ui| {
            ui.label(RichText::new(qnc_wizard_text::text("wizard.title")).heading().color(theme.text));
            if let Some(project) = &view.project {
                ui.label(RichText::new(project).color(theme.text_muted));
            }
            ui.add_space(10.0);
            for (index, row) in view.steps.iter().enumerate() {
                let mark = if row.done { "✔" } else { " " };
                let colour = if row.shown {
                    theme.focus
                } else if row.may_open {
                    theme.text
                } else {
                    theme.text_muted
                };
                let fill = if row.shown { theme.surface_alt } else { Color32::TRANSPARENT };
                let label = RichText::new(format!("{mark}  {}. {}", index + 1, row.name)).color(colour);
                let button = qnc_ui_kit::framed_button(label, Vec2::new(ui.available_width(), measures.step_row_height), fill, None, true);
                if ui.add_enabled(row.may_open, button).clicked() {
                    intent = Some(Intent::Open(index));
                }
            }
        });

    egui::TopBottomPanel::bottom("wizard_footer")
        .exact_height(measures.footer_height)
        .frame(egui::Frame::new().fill(theme.surface).inner_margin(10.0).stroke(Stroke::new(1.0, theme.border)))
        .show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let size = Vec2::new(measures.button_width, theme.chrome_control_height + 6.0);
                if ui.add_enabled(view.may_back, qnc_ui_kit::sized_button(qnc_wizard_text::text("wizard.back"), size)).clicked() {
                    intent = Some(Intent::Back);
                }
                ui.label(
                    RichText::new(qnc_wizard_text::fill("wizard.step_of", &[("n", &(view.shown + 1).to_string()), ("all", &view.steps.len().to_string())]))
                        .color(theme.text_muted),
                );
                if view.saving {
                    ui.label(RichText::new(qnc_wizard_text::text("wizard.saving")).color(theme.text_muted));
                }
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if qnc_ui_kit::strong_button(ui, &qnc_wizard_text::text("wizard.next"), view.may_next, theme.accent, size.y).clicked() {
                        intent = Some(Intent::Next);
                    }
                });
            });
        });

    egui::CentralPanel::default().frame(egui::Frame::new().fill(theme.bg).inner_margin(24.0)).show(ctx, |ui| {
        if let Some(message) = &view.message {
            ui.label(RichText::new(message).color(theme.text));
            return;
        }
        ui.label(RichText::new(&view.shown_name).heading().color(theme.text));
        ui.add_space(6.0);
        ui.label(RichText::new(&view.done_when).color(theme.text_muted));
        ui.add_space(18.0);
        ui.label(RichText::new(qnc_wizard_text::text("wizard.empty_step")).color(theme.text_muted));
        ui.add_space(12.0);
        let (code, wanted) = if view.shown_done { ("wizard.reopen", Intent::Reopen) } else { ("wizard.mark_done", Intent::MarkDone) };
        if ui.add_enabled(view.may_mark, qnc_ui_kit::sized_button(qnc_wizard_text::text(code), Vec2::new(measures.button_width * 1.4, theme.chrome_control_height + 6.0))).clicked() {
            intent = Some(wanted);
        }
    });
    intent
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_layout_of_the_wizard_is_read() {
        let look = super::Look::load().unwrap();
        assert!(look.measures.steps_width > 0.0);
    }
}
