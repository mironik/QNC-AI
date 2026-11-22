//! Breaking news (user 2026-11-16: "brejking news nije trajni projekt, to je brejking news; njemu
//! ne treba pisati naziv, on je brejking news-datum, vrijeme"; "automatski kreiraš brejking news
//! projekt, bez da novinar bilo što piše"). A card chosen in the Ingest form (not a base) is the
//! second way to the clips: the project links the source card itself, no copy. The project is made
//! at once from the template chosen in Settings, named by the word and the time, and made active;
//! Ingest then Selects the card into it as QNC does, and every AI process writes into its own
//! database (`project.db`, `ai.db`), never into a base.

use std::path::Path;

use qnc_application_catalog::SelectionSnapshot;
use qnc_project_store::ProjectStore;
use serde_json::Value;

/// How a breaking news project takes the card's clips: a link to the source card.
const LINKS: &str = "link";

/// The AI of a breaking news project: the news template with all its features, when this computer
/// has it; otherwise the template's own choice stays.
const NEWS_AI: &str = "news_full";

/// The name of a breaking news project made at `time`.
pub fn name_at(time: chrono::DateTime<chrono::Local>) -> String {
    format!("{} {}", qnc_wizard_text::text("breaking.name"), time.format("%Y-%m-%d %H:%M"))
}

/// The settings of a breaking news project made from `template` (the settings of the template
/// chosen in Settings): the card is linked, the news AI is on.
pub fn settings_of(root: &Path, mut template: Value) -> Value {
    if let Some(storage) = template.get_mut("storage").and_then(Value::as_object_mut) {
        storage.insert("ingest_media".into(), Value::String(LINKS.into()));
    }
    if let Ok(features) = qnc_ai_features::Features::load(root) {
        let (templates, _) = qnc_ai_features::templates(root, &features);
        if let Some(news) = templates.iter().find(|template| template.id == NEWS_AI) {
            qnc_ai_features::set_choice(&mut template, &news.id, &news.features);
        }
    }
    template
}

/// Makes a breaking news project and makes it active: gives its id and name.
pub fn make(root: &Path) -> Result<(String, String), String> {
    qnc_ingest_base::keep_projects_apart(root)?;
    let store = ProjectStore::open(root)?;
    let template = store.selected_template_settings()?.settings;
    let selection: SelectionSnapshot = match template.get("workspace").and_then(|workspace| workspace.get("application_selection")) {
        Some(value) => serde_json::from_value(value.clone()).map_err(|_| "the chosen template's applications are not readable")?,
        // A system template names no applications (audit 2026-11-22: breaking news then never
        // started): every application installed on this computer, one per priority group.
        None => installed_applications(root)?,
    };
    let settings = settings_of(root, template);
    let made = store.create_project(&name_at(chrono::Local::now()), "", Some(&settings), &selection)?;
    Ok((made.project_id, made.name))
}

/// The applications installed on this computer (its application catalog), the first of each
/// priority group.
fn installed_applications(root: &Path) -> Result<SelectionSnapshot, String> {
    let catalog = qnc_application_catalog::read_catalog(&root.join("data").join("application-catalog.json"))
        .map_err(|error| format!("the chosen template names no applications and the application catalog is not readable: {error}"))?;
    first_of_each_group(&catalog)
}

fn first_of_each_group(catalog: &qnc_application_catalog::Catalog) -> Result<SelectionSnapshot, String> {
    let mut groups = std::collections::BTreeMap::new();
    for app in &catalog.applications {
        groups.entry(app.priority_group.clone()).or_insert_with(|| app.application_id.clone());
    }
    let ids: Vec<String> = groups.into_values().collect();
    SelectionSnapshot::from_catalog(catalog, &ids).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_breaking_news_project_is_named_by_its_time_and_links_the_card() {
        let time = chrono::TimeZone::with_ymd_and_hms(&chrono::Local, 2026, 11, 16, 14, 32, 5).unwrap();
        let name = name_at(time);
        assert!(name.ends_with(" 2026-11-16 14:32"), "{name}");
        let root = std::env::temp_dir().join(format!("qnc_breaking_news_{}", std::process::id()));
        let settings = settings_of(&root, serde_json::json!({ "storage": { "ingest_media": "original" } }));
        assert_eq!(settings["storage"]["ingest_media"], LINKS);
    }
}

#[cfg(test)]
mod selection_tests {
    use super::*;
    use qnc_application_catalog::{Application, Catalog, HostMode};

    #[test]
    fn a_template_without_applications_takes_the_first_installed_of_each_group() {
        let app = |id: &str, group: &str| Application {
            application_id: format!("qnc.{id}"),
            tab_id: id.into(),
            label: id.into(),
            priority_group: group.into(),
            system: false,
            removable: true,
            host_mode: HostMode::ExternalComponent,
            desktop_entry: id.into(),
            standalone_executable: id.into(),
        };
        let catalog = Catalog {
            catalog_id: qnc_application_catalog::CATALOG_ID.into(),
            schema_version: 1,
            catalog_uri: qnc_application_catalog::DEFAULT_URI.into(),
            selection_policy: "one_per_priority_group".into(),
            observed_at_unix_ms: 1,
            target_os: "windows".into(),
            target_cpu: "x86_64".into(),
            applications: vec![app("project", "a"), app("ai", "e"), app("story", "o"), app("story-lite", "o")],
            unavailable: vec![],
        };
        let selection = first_of_each_group(&catalog).unwrap();
        let ids: Vec<_> = selection.applications.iter().map(|app| app.application_id.as_str()).collect();
        assert_eq!(ids, ["qnc.project", "qnc.ai", "qnc.story"]);
        selection.validate().unwrap();
    }
}
