//! QNC-AI Settings, tab User (user 2026-11-24: "racunalo moze imati vise korisnika s razlicitim
//! preferencijama"; "preset tipki je po korisniku, jezik po korisniku"): the settings of the
//! user the operating system signed in. Each row reads and writes its own setting through the
//! block that owns it (`qnc-ui-text`, `qnc-key-preset`, `qnc-edit-step`, `qnc-voice-record`);
//! all of them live in the user's folder (`qnc-user-settings`), and what the user did not
//! choose comes from the computer. Drawn with the settings style block.

use std::path::Path;

use eframe::egui;
use qnc_settings_block::Pen;
use qnc_wizard_text::{fill, text};

/// What the rows choose from, which the caller's contracts know: the keyboard presets
/// (id, name) and the catalog's own preset.
pub struct Lists<'a> {
    pub keyboard_presets: &'a [(String, String)],
    pub keyboard_default: &'a str,
}

/// Draws the tab's group into `ui`.
pub fn show(ui: &mut egui::Ui, root: &Path, pen: &Pen, lists: &Lists<'_>) {
    match qnc_user_settings::user_name() {
        Ok(user) => pen.heading(ui, &fill("user.heading", &[("user", &user)])),
        Err(error) => {
            pen.said(ui, &Some(Err(fill("user.no_user", &[("error", &error)]))));
            return;
        }
    }
    pen.hint(ui, &text("user.hint"));
    // What a row last said stays until another row says something (it is not lost after one frame).
    let said_id = egui::Id::new(("qnc-user-settings-said", root.to_string_lossy().to_string()));
    let mut said = ui.ctx().data_mut(|data| data.get_temp::<Option<Result<String, String>>>(said_id)).flatten();
    language(ui, root, pen, &mut said);
    keyboard(ui, root, pen, lists, &mut said);
    wheel(ui, root, pen, &mut said);
    microphone(ui, root, pen, &mut said);
    pen.said(ui, &said);
    ui.ctx().data_mut(|data| data.insert_temp(said_id, said));
}

fn language(ui: &mut egui::Ui, root: &Path, pen: &Pen, said: &mut Option<Result<String, String>>) {
    let chosen = qnc_ui_text::chosen_language(root);
    if let Some(language) = pen.choice(ui, &text("user.language"), "qnc_user_language", &chosen, &qnc_ui_text::languages()) {
        if let Err(error) = qnc_ui_text::choose_language(root, &language) {
            *said = Some(Err(error));
        }
    }
    let running = qnc_ui_text::host().map(|texts| texts.language.clone()).unwrap_or_default();
    if qnc_ui_text::chosen_language(root) != running {
        pen.hint(ui, &text("user.after_restart"));
    }
}

fn keyboard(ui: &mut egui::Ui, root: &Path, pen: &Pen, lists: &Lists<'_>, said: &mut Option<Result<String, String>>) {
    let current = qnc_key_preset::user_preset(root).unwrap_or_else(|| lists.keyboard_default.to_string());
    if let Some(preset) = pen.choice(ui, &text("user.keyboard"), "qnc_user_keyboard", &current, lists.keyboard_presets) {
        if let Err(error) = qnc_key_preset::choose_preset(root, &preset) {
            *said = Some(Err(error));
        }
    }
}

fn wheel(ui: &mut egui::Ui, root: &Path, pen: &Pen, said: &mut Option<Result<String, String>>) {
    let forward = qnc_user_settings::read_from(root, qnc_edit_step::SETTING_FILE)
        .and_then(|path| std::fs::read_to_string(path).ok())
        .and_then(|setting| qnc_edit_step::parse_setting(&setting).ok())
        .unwrap_or(true);
    let current = if forward { "forward" } else { "back" };
    let choices = [("forward".to_string(), text("user.wheel_forward")), ("back".to_string(), text("user.wheel_back"))];
    if let Some(chosen) = pen.choice(ui, &text("user.wheel"), "qnc_user_wheel", current, &choices) {
        let setting = serde_json::json!({ "playhead_up": chosen }).to_string();
        match qnc_user_settings::write(root, qnc_edit_step::SETTING_FILE, &setting) {
            Ok(()) => *said = Some(Ok(text("user.after_restart"))),
            Err(error) => *said = Some(Err(error)),
        }
    }
}

fn microphone(ui: &mut egui::Ui, root: &Path, pen: &Pen, said: &mut Option<Result<String, String>>) {
    // The inputs are listed once when the tab is first shown and again on Refresh (listing asks
    // the sound system and takes a moment).
    let id = egui::Id::new(("qnc-user-settings-microphones", root.to_string_lossy().to_string()));
    let mut devices = ui.ctx().data_mut(|data| data.get_temp::<Vec<String>>(id));
    let current = match qnc_voice_record::chosen_input(root) {
        Ok(name) => name.unwrap_or_default(),
        Err(error) => {
            *said = Some(Err(error));
            String::new()
        }
    };
    let mut choices = vec![(String::new(), text("user.microphone_default"))];
    choices.extend(devices.iter().flatten().map(|name| (name.clone(), name.clone())));
    if !current.is_empty() && !choices.iter().any(|(name, _)| *name == current) {
        choices.push((current.clone(), fill("user.microphone_absent", &[("name", &current)])));
    }
    let (mut picked, mut refresh) = (None, devices.is_none());
    pen.row(
        ui,
        &text("user.microphone"),
        |ui, row| picked = row.choice(ui, "qnc_user_microphone", row.width, &current, &choices),
        |ui, buttons| refresh |= buttons.action(ui, &text("user.refresh")),
    );
    if refresh {
        devices = Some(qnc_voice_record::input_devices());
        ui.ctx().data_mut(|data| data.insert_temp(id, devices.clone().unwrap_or_default()));
    }
    if let Some(name) = picked {
        if let Err(error) = qnc_voice_record::choose_input(root, Some(name.as_str()).filter(|name| !name.is_empty())) {
            *said = Some(Err(error));
        }
    }
}
