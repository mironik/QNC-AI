//! The orchestrators of an ingest (user 2026-11-16: "sve kocke koje služe za ingest moraju biti
//! univerzalne... gdje će se, kojim redosljedom odvijati, i gdje će se rezultati njihova rada
//! snimati određuju Orkestratori"; "može biti više varijanti orkestratora... pa se orkestratori
//! biraju u postavkama"). The blocks only do their work. An orchestrator says in which project a
//! card's ingest writes; the order of the steps is the template's (`qnc-ingest-plan`). The
//! orchestrators are a catalog (`catalogs/ingest/orchestrators.json`), so another variant is data;
//! which one a form uses is a setting of this computer (`data/ingest-orchestrators.qncingest`, JSON, one per
//! form, chosen in the settings), never a default made up.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Where the ingest of a card writes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Writes {
    /// A project of its own, made for the card (breaking news), unless the form works in a
    /// story the user opened.
    NewProject,
    /// The project active now.
    ActiveProject,
    /// The Ingest base of the card (the tray's wizard).
    CardBase,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Orchestrator {
    pub id: String,
    /// The text code of its name.
    pub label: String,
    pub writes: Writes,
    /// The forms it can serve (`ingest`, `tray`).
    pub forms: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Catalog {
    orchestrators: Vec<Orchestrator>,
}

/// The form of Ingest (QNC-AI's Ingest form).
pub const INGEST: &str = "ingest";
/// The tray's own ingest (the wizard).
pub const TRAY: &str = "tray";

/// The variable of a form opened for a new story ("Novi projekt" of the Projects form).
const NEW_STORY: &str = "QNC_NEW_STORY";

fn catalog_file(root: &Path) -> PathBuf {
    root.join("catalogs").join("ingest").join("orchestrators.json")
}

fn choice_file(root: &Path) -> PathBuf {
    root.join("data").join("ingest-orchestrators.qncingest")
}

/// Every orchestrator of the catalog.
pub fn catalog(root: &Path) -> Result<Vec<Orchestrator>, String> {
    let file = catalog_file(root);
    let text = std::fs::read_to_string(&file).map_err(|error| format!("{}: {error}", file.display()))?;
    let catalog: Catalog = serde_json::from_str(&text).map_err(|error| format!("{}: {error}", file.display()))?;
    let mut seen = std::collections::BTreeSet::new();
    for orchestrator in &catalog.orchestrators {
        if !seen.insert(orchestrator.id.clone()) {
            return Err(format!("{}: {} twice", file.display(), orchestrator.id));
        }
    }
    Ok(catalog.orchestrators)
}

/// The orchestrators a form can use.
pub fn for_form(root: &Path, form: &str) -> Result<Vec<Orchestrator>, String> {
    Ok(catalog(root)?.into_iter().filter(|orchestrator| orchestrator.forms.iter().any(|f| f == form)).collect())
}

fn choices(root: &Path) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    let file = choice_file(root);
    match std::fs::read_to_string(&file) {
        Ok(text) => match serde_json::from_str(&text).map_err(|error| format!("{}: {error}", file.display()))? {
            serde_json::Value::Object(map) => Ok(map),
            _ => Err(format!("{}: not an object", file.display())),
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(Default::default()),
        Err(error) => Err(format!("{}: {error}", file.display())),
    }
}

/// The orchestrator chosen on this computer for `form`; not chosen is an error.
pub fn chosen(root: &Path, form: &str) -> Result<Orchestrator, String> {
    let id = choices(root)?
        .get(form)
        .and_then(|value| value.as_str())
        .map(str::to_string)
        .ok_or_else(|| format!("no orchestrator chosen for {form} ({})", choice_file(root).display()))?;
    for_form(root, form)?.into_iter().find(|orchestrator| orchestrator.id == id).ok_or_else(|| format!("orchestrator {id} is not one for {form}"))
}

/// Chooses `id` for `form` on this computer (the others stay).
pub fn choose(root: &Path, form: &str, id: &str) -> Result<(), String> {
    if !for_form(root, form)?.iter().any(|orchestrator| orchestrator.id == id) {
        return Err(format!("orchestrator {id} is not one for {form}"));
    }
    let mut map = choices(root)?;
    map.insert(form.to_string(), serde_json::Value::String(id.to_string()));
    let file = choice_file(root);
    let text = serde_json::to_string_pretty(&serde_json::Value::Object(map)).map_err(|error| error.to_string())?;
    std::fs::write(&file, text).map_err(|error| format!("{}: {error}", file.display()))
}

/// The project a card's ingest writes in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Destination {
    pub project_id: String,
    /// The name of a project made for the card; `None` for one that was there.
    pub made: Option<String>,
}

impl Orchestrator {
    /// Where the card's ingest writes; a project made for it is made now (and active).
    pub fn destination(&self, root: &Path) -> Result<Destination, String> {
        match self.writes {
            // A form opened for a new story, or no story of the user active: a project of its own.
            Writes::NewProject if std::env::var_os(NEW_STORY).is_some() || !qnc_ingest_base::active_is_story(root) => {
                let (project_id, name) = qnc_breaking_news::make(root)?;
                Ok(Destination { project_id, made: Some(name) })
            }
            Writes::NewProject | Writes::ActiveProject => {
                let snapshot = qnc_active_project_read::read_from_root(root).map_err(|error| error.to_string())?;
                Ok(Destination { project_id: snapshot.project_id, made: None })
            }
            Writes::CardBase => Err(format!("orchestrator {} writes into a card's base: the tray's", self.id)),
        }
    }

    /// The Ingest base a card's ingest writes in (the tray's orchestrator, user 2026-11-16: "traj u
    /// automatskom radu uvijek kreira novu bazu"): a new base named by its camera, every time.
    pub fn card_base(&self, root: &Path, camera: &str) -> Result<qnc_ingest_base::IngestBase, String> {
        self.only(Writes::CardBase)?;
        qnc_ingest_base::new_base(root, camera)
    }

    /// The base a folder could go into besides a new one (the user is asked, user 2026-11-16:
    /// "za uvoz iz direktorija pita"): the base Ingest works in now, if there is one.
    pub fn folder_choice(&self, root: &Path) -> Result<Option<qnc_ingest_base::IngestBase>, String> {
        self.only(Writes::CardBase)?;
        Ok(qnc_ingest_base::all(root)?.into_iter().find(|base| base.active))
    }

    /// The Ingest base a folder's ingest writes in: a new one, or `into` (the answer of the user).
    pub fn folder_base(&self, root: &Path, name: &str, into: Option<&str>) -> Result<qnc_ingest_base::IngestBase, String> {
        self.only(Writes::CardBase)?;
        match into {
            Some(id) => qnc_ingest_base::by_id(root, id),
            None => qnc_ingest_base::new_base(root, name),
        }
    }

    fn only(&self, writes: Writes) -> Result<(), String> {
        if self.writes == writes {
            Ok(())
        } else {
            Err(format!("orchestrator {} does not write into a card's base", self.id))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_with(choice: &str) -> tempfile::TempDir {
        let root = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(root.path().join("catalogs/ingest")).unwrap();
        std::fs::create_dir_all(root.path().join("data")).unwrap();
        std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/../../catalogs/ingest/orchestrators.json"), catalog_file(root.path())).unwrap();
        std::fs::write(choice_file(root.path()), choice).unwrap();
        root
    }

    #[test]
    fn a_form_uses_the_orchestrator_chosen_for_it_and_only_one_that_serves_it() {
        let root = root_with(r#"{"ingest": "breaking_news", "tray": "card_base"}"#);
        assert_eq!(chosen(root.path(), INGEST).unwrap().writes, Writes::NewProject);
        assert_eq!(chosen(root.path(), TRAY).unwrap().writes, Writes::CardBase);
        assert!(choose(root.path(), INGEST, "card_base").is_err(), "the card base is the tray's");
        choose(root.path(), INGEST, "active_project").unwrap();
        assert_eq!(chosen(root.path(), INGEST).unwrap().id, "active_project");
        assert_eq!(chosen(root.path(), TRAY).unwrap().id, "card_base", "the other form's choice stays");
    }

    #[test]
    fn a_form_with_no_orchestrator_chosen_is_an_error_not_a_default() {
        let root = root_with("{}");
        assert!(chosen(root.path(), INGEST).is_err());
    }
}
