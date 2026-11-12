//! The Ingest bases of a QNC-AI root and the project that took each (read only): `list_bases <root>`.

fn main() -> Result<(), String> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("usage: list_bases <root>")?);
    for base in qnc_ingest_base::all(&root)? {
        println!("{} active={} project={:?} {}", base.name, base.active, base.project_name, base.id);
        let store = qnc_project_store::ProjectStore::open(&root)?;
        println!("    db: {:?}", store.project_db_binding(&base.id).map(|(file, _)| file));
    }
    println!("active is base: {}", qnc_ingest_base::active_is_base(&root));
    Ok(())
}
