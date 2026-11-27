//! The projects listed in a QNC root, read only: `list_projects QNC_ROOT` prints each project's id,
//! name and whether it is the active one.

fn main() -> Result<(), String> {
    let root = std::env::args().nth(1).ok_or("QNC_ROOT")?;
    let store = qnc_project_store::ProjectStore::open(root)?;
    for project in store.list_projects()? {
        let folder = store.project_folder(&project.project_id).map(|path| path.display().to_string()).unwrap_or_default();
        println!("{} {} | {} | {folder}", if project.active { "*" } else { " " }, project.project_id, project.name);
    }
    Ok(())
}
