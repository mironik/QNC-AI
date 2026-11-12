//! Deletes every project of a QNC-AI root's registry, Ingest bases and stories alike, with their
//! folders (the copied Project store lifts their protection first): `forget_all <root>`.

fn main() -> Result<(), String> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("usage: forget_all <root>")?);
    let store = qnc_project_store::ProjectStore::open(&root)?;
    for row in store.list_projects()? {
        match store.delete_project(&row.project_id) {
            Ok(_) => println!("deleted {}", row.name),
            Err(error) => println!("not deleted {}: {error}", row.name),
        }
    }
    Ok(())
}
