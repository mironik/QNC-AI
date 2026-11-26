//! The projects listed in a QNC root whose folder is under a given folder: listed, or forgotten
//! by the registry with `--forget` (the folders stay). `forget_under QNC_ROOT FOLDER [--forget]`.

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (root, folder) = (args.first().ok_or("QNC_ROOT")?, std::path::PathBuf::from(args.get(1).ok_or("FOLDER")?));
    let forget = args.iter().any(|arg| arg == "--forget");
    let store = qnc_project_store::ProjectStore::open(root)?;
    for project in store.list_projects()? {
        let dir = store.project_folder(&project.project_id)?;
        if !dir.starts_with(&folder) {
            continue;
        }
        if forget {
            store.forget_project(&project.project_id)?;
            println!("forgotten {} {} {}", project.project_id, project.name, dir.display());
        } else {
            println!("listed {} {} {}", project.project_id, project.name, dir.display());
        }
    }
    Ok(())
}
