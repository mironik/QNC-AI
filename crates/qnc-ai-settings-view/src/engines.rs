//! The AI engines of this computer (`qnc-ai-catalog-edit`): which transcription engine is the best,
//! the fast and the live one, the threads and the clips at once of each, and the model of every
//! other AI job; Save writes the catalog unless it would break an engine that worked.

use std::path::Path;

use eframe::egui::{self, RichText};
use qnc_ai_catalog_edit::{Catalog, JOBS, TRANSCRIBE_SLOTS};
use qnc_desktop_theme::Theme;
use qnc_settings_form::{FieldKind, FieldSpec, OptionSets};
use qnc_settings_window::Style;
use qnc_wizard_text::text;
use serde_json::{Map, Value};

fn spec(kind: FieldKind, label: String, path: String, options: Option<&str>) -> FieldSpec {
    FieldSpec {
        kind,
        id: format!("qnc-ai-engines-{path}"),
        label,
        path,
        options: options.map(str::to_string),
        default: String::new(),
        hint: String::new(),
    }
}

fn as_text(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Number(number)) => number.to_string(),
        _ => String::new(),
    }
}

/// Draws the engines of `catalog` and keeps what is typed in it; gives what Save said.
pub fn show(
    ui: &mut egui::Ui,
    root: &Path,
    width: f32,
    style: &Style,
    theme: &Theme,
    catalog: &mut Catalog,
) -> Option<Result<String, String>> {
    let engines = catalog.engines();
    let mut options = OptionSets::new();
    let ids: Vec<(String, String)> = engines
        .iter()
        .map(|engine| (engine.id.clone(), engine.id.clone()))
        .collect();
    options.insert("engines".into(), ids.clone());
    options.insert(
        "engines_or_none".into(),
        std::iter::once((String::new(), text("ai_engines.none")))
            .chain(ids)
            .collect(),
    );
    options.insert(
        "backends".into(),
        ["cpu", "directml", "vulkan"]
            .iter()
            .map(|b| (b.to_string(), b.to_string()))
            .collect(),
    );
    options.insert(
        "pictures".into(),
        (1..=3).map(|n| (n.to_string(), n.to_string())).collect(),
    );
    let models = qnc_ai_catalog_edit::shared_models(root);

    let mut settings = Map::new();
    let mut slot_fields = Vec::new();
    for (slot, code) in TRANSCRIBE_SLOTS {
        settings.insert(
            slot.into(),
            Value::from(catalog.slot(slot).unwrap_or_default()),
        );
        slot_fields.push(spec(
            FieldKind::Choice,
            text(code),
            slot.into(),
            Some(if slot == "selected" {
                "engines"
            } else {
                "engines_or_none"
            }),
        ));
    }
    let mut engine_fields = Vec::new();
    for (index, engine) in engines.iter().enumerate() {
        settings.insert(format!("t{index}"), Value::from(engine.threads));
        settings.insert(
            format!("p{index}"),
            engine.parallel.map(Value::from).unwrap_or(Value::from("")),
        );
        engine_fields.push(spec(
            FieldKind::Integer,
            text("ai_engines.threads").replace("{engine}", &engine.id),
            format!("t{index}"),
            None,
        ));
        engine_fields.push(spec(
            FieldKind::Integer,
            text("ai_engines.parallel").replace("{engine}", &engine.id),
            format!("p{index}"),
            None,
        ));
    }
    let mut job_fields = Vec::new();
    for (job, code) in JOBS {
        if !catalog.has_job(job) {
            continue;
        }
        let model = catalog.job(job, "model").unwrap_or_default();
        let mut choices: Vec<(String, String)> = models
            .iter()
            .map(|m| (m.clone(), m.trim_start_matches("share:models/").to_string()))
            .collect();
        if !model.is_empty() && !models.contains(&model) {
            choices.insert(0, (model.clone(), model.clone()));
        }
        options.insert(format!("models_{job}"), choices);
        settings.insert(format!("{job}_model"), Value::from(model));
        job_fields.push(spec(
            FieldKind::Choice,
            text(code),
            format!("{job}_model"),
            Some(&format!("models_{job}")),
        ));
        if job == "shot_caption" {
            settings.insert(
                "shot_caption_backend".into(),
                Value::from(catalog.job(job, "backend").unwrap_or_else(|| "cpu".into())),
            );
            job_fields.push(spec(
                FieldKind::Choice,
                text("ai_engines.caption_backend"),
                "shot_caption_backend".into(),
                Some("backends"),
            ));
        }
        if job == "shot_content" {
            settings.insert(
                "shot_content_pictures".into(),
                Value::from(catalog.job(job, "pictures").unwrap_or_else(|| "2".into())),
            );
            job_fields.push(spec(
                FieldKind::Choice,
                text("ai_engines.pictures"),
                "shot_content_pictures".into(),
                Some("pictures"),
            ));
        }
    }

    let mut settings = Value::Object(settings);
    for (title, fields) in [
        ("ai_engines.transcribe", &slot_fields),
        ("ai_engines.engine_list", &engine_fields),
        ("ai_engines.jobs", &job_fields),
    ] {
        qnc_settings_rows::section(ui, width, &text(title), &style.metrics, theme, |ui| {
            qnc_settings_form::field_grid(ui, width, &mut settings, fields, &options, &style.grid);
        });
    }

    // What was typed goes into the catalog as a draft; a value it refuses is said.
    let mut refused = Vec::new();
    for (slot, _) in TRANSCRIBE_SLOTS {
        let id = as_text(settings.get(slot));
        if catalog.slot(slot).unwrap_or_default() != id {
            if let Err(error) = catalog.set_slot(slot, (!id.is_empty()).then_some(id.as_str())) {
                refused.push(error);
            }
        }
    }
    for (index, engine) in engines.iter().enumerate() {
        let threads = as_text(settings.get(format!("t{index}").as_str()))
            .parse::<u32>()
            .unwrap_or(engine.threads);
        let parallel = as_text(settings.get(format!("p{index}").as_str()))
            .parse::<u32>()
            .ok();
        if (threads, parallel) != (engine.threads, engine.parallel) {
            if let Err(error) = catalog.set_engine(&engine.id, threads, parallel) {
                refused.push(error);
            }
        }
    }
    for (job, _) in JOBS {
        for field in ["model", "backend", "pictures"] {
            if let Some(value) = settings
                .get(format!("{job}_{field}").as_str())
                .map(|value| as_text(Some(value)))
            {
                if catalog.job(job, field).unwrap_or_default() != value && !value.is_empty() {
                    if let Err(error) = catalog.set_job(job, field, &value) {
                        refused.push(error);
                    }
                }
            }
        }
    }
    ui.label(
        RichText::new(text("ai_engines.hint"))
            .size(style.grid.label_size)
            .color(theme.text_muted),
    );
    let mut said = (!refused.is_empty()).then(|| Err(refused.join("; ")));
    if qnc_settings_rows::primary_btn(ui, &text("machine.save"), true, theme).clicked() {
        said = Some(catalog.save().map(|()| text("machine.saved")));
    }
    said
}
