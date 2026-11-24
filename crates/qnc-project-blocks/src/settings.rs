//! Block: the Project settings panel and its sub-blocks (template picker, project name,
//! AI switches, location fields with their browsers, template actions, advanced settings,
//! own template), in the order of the layout contract (`pts_slots`).

use eframe::egui::{self, Sense, Vec2};
use qnc_project_session::ProjectIntent;

use crate::{
    layout_contract::SettingsPanelMetrics,
    theme::{self, Theme},
    widgets, Blocks,
};

impl Blocks<'_> {
    pub(crate) fn settings_panel(&mut self, ui: &mut egui::Ui, width: f32, height: f32) {
        let t = Theme::from_contract(&self.contracts.shell.colors);
        let settings = self.contracts.project.right_settings_panel.clone();
        let (panel_rect, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        ui.painter().rect_filled(panel_rect, 0.0, t.bg);
        let inner = panel_rect.shrink(settings.panel_pad);
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.set_clip_rect(inner);
                ui.set_min_width(settings.field_min_width);
                self.pts_panel(
                    ui,
                    inner.width().max(40.0),
                    inner.height().max(40.0),
                    &settings,
                );
            },
        );
    }

    fn pts_panel(
        &mut self,
        ui: &mut egui::Ui,
        width: f32,
        height: f32,
        settings: &SettingsPanelMetrics,
    ) {
        let t = Theme::from_contract(&self.contracts.shell.colors);
        let (panel, _) = ui.allocate_exact_size(Vec2::new(width, height), Sense::hover());
        ui.painter().rect_filled(panel, 0.0, t.bg);
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(panel)
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.set_clip_rect(panel);
                ui.spacing_mut().item_spacing = Vec2::ZERO;
                self.pts_head(ui, width, settings);
                self.pts_scroll(ui, width, panel.bottom(), settings);
            },
        );
    }

    fn pts_head(&mut self, ui: &mut egui::Ui, width: f32, settings: &SettingsPanelMetrics) {
        let t = Theme::from_contract(&self.contracts.shell.colors);
        widgets::panel_title_row(ui, &settings.title, &self.contracts.theme);
        egui::Frame::NONE
            .inner_margin(egui::Margin {
                left: settings.inner_pad_x as i8,
                right: settings.inner_pad_x as i8,
                top: 0,
                bottom: settings.inner_pad_y as i8,
            })
            .show(ui, |ui| {
                ui.set_max_width(width);
                theme::label(ui, &settings.subtitle, 12.0, t.muted);
            });
    }

    /// The slots shown always, first in the scroll (user 2026-10-27: the template list
    /// scrolls with the panel).
    fn pts_fixed(&mut self, ui: &mut egui::Ui, width: f32, settings: &SettingsPanelMetrics) {
        let (theme, texts) = (&self.contracts.theme, &self.contracts.project.texts);
        let pen = qnc_settings_block::Pen::new(settings, theme, width);
        for slot in self.contracts.project.pts_slots.fixed_order.clone() {
            match slot.as_str() {
                "TemplatePicker" => self.intents.extend(qnc_project_templates_view::picker(ui, &pen, theme, texts, self.session)),
                "ProjectCreate" if !self.settings_only => self.intents.extend(qnc_project_templates_view::project_create(ui, &pen, texts, self.session)),
                _ => {}
            }
        }
    }

    fn pts_scroll(
        &mut self,
        ui: &mut egui::Ui,
        width: f32,
        panel_bottom: f32,
        settings: &SettingsPanelMetrics,
    ) {
        let scroll_top = ui.cursor().top() + 1.0;
        let scroll_h = (panel_bottom - scroll_top).max(48.0);
        let scroll_rect = egui::Rect::from_min_size(
            egui::pos2(ui.min_rect().left(), scroll_top),
            Vec2::new(width, scroll_h),
        );
        ui.allocate_new_ui(
            egui::UiBuilder::new()
                .max_rect(scroll_rect)
                .layout(egui::Layout::top_down(egui::Align::Min)),
            |ui| {
                ui.set_clip_rect(scroll_rect);
                egui::ScrollArea::vertical()
                    .id_salt("pts_scroll")
                    .max_height(scroll_h)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        egui::Frame::NONE
                            .inner_margin(egui::Margin {
                                left: settings.inner_pad_x as i8,
                                right: settings.inner_pad_x as i8,
                                top: settings.inner_pad_y as i8,
                                bottom: settings.inner_pad_y as i8,
                            })
                            .show(ui, |ui| {
                                // The content stays inside the margins (it went past the window's right edge).
                                let width = (width - 2.0 * settings.inner_pad_x).max(40.0);
                                ui.set_max_width(width);
                                ui.spacing_mut().item_spacing.y = settings.section_gap;
                                self.templates(ui, width, settings);
                            });
                    });
            },
        );
    }

    fn ai_settings(&mut self, ui: &mut egui::Ui, content_w: f32, settings: &SettingsPanelMetrics) {
        widgets::section(ui, content_w, "AI", settings, &self.contracts.theme, |ui| {
            ui.spacing_mut().item_spacing.y = 8.0;
            let (switches, session) = (&self.contracts.project.ai_switches, &mut self.session);
            if qnc_ai_features_view::show(ui, &mut session.ai, &mut session.draft_settings) | qnc_settings_switches::show(ui, &mut session.draft_settings, switches) {
                self.session.status = self.contracts.project.texts.settings_changed.clone();
            }
        });
    }

    /// The template picker; in the Settings form (its Project tab) then the Advanced bar, and the
    /// slots of the scroll only while it is open. The Project form shows only the picker and New
    /// project (user 2026-11-24: "postavke su postavke, ne moze biti vise postavki"): a template's
    /// settings, the locations and the user's settings are changed only in Settings.
    pub(crate) fn templates(&mut self, ui: &mut egui::Ui, width: f32, settings: &SettingsPanelMetrics) {
        self.pts_fixed(ui, width, settings);
        if !self.settings_only {
            return;
        }
        qnc_project_advanced_view::header(ui, width, &self.contracts.project.advanced_settings, &self.contracts.theme, &mut self.session.advanced_open);
        let slots = if self.session.advanced_open { self.contracts.project.pts_slots.scroll_order.clone() } else { Vec::new() };
        let theme = self.contracts.theme.clone();
        let pen = qnc_settings_block::Pen::new(settings, &theme, width);
        for slot in slots {
            let texts = &self.contracts.project.texts;
            match slot.as_str() {
                "AiSettings" => self.ai_settings(ui, width, settings),
                "TemplateActions" => self.intents.extend(qnc_project_templates_view::actions(ui, &pen, texts, self.session)),
                "AdvancedSections" => self.advanced(ui, width, settings),
                "CustomTemplate" if self.session.template_create_open => self.intents.extend(qnc_project_templates_view::own(ui, &pen, texts, self.session)),
                _ => {}
            }
        }
    }

    fn advanced(&mut self, ui: &mut egui::Ui, content_w: f32, settings: &SettingsPanelMetrics) {
        let applications = self.session.component.applications.view();
        let mut application_action = None;
        if qnc_project_advanced_view::show(
            ui,
            content_w,
            settings,
            &self.contracts.project.advanced_settings,
            &self.contracts.theme,
            qnc_project_advanced_view::AdvancedDraft {
                draft_settings: &mut self.session.draft_settings,
                export_preset_draft_name: &mut self.session.export_preset_draft_name,
                applications: &applications,
            },
            &mut application_action,
        ) {
            self.session.status = self.contracts.project.texts.settings_changed.clone();
        }
        if let Some(action) = application_action {
            use qnc_project_advanced_view::ApplicationSelectionAction;
            self.intents.push(match action {
                ApplicationSelectionAction::Choose {
                    priority_group,
                    application_id,
                } => ProjectIntent::SelectWorkflowGroup {
                    priority_group,
                    application_id,
                },
            });
        }
    }

}
