//! Where a QNC-AI root makes its Ingest bases and its projects (read only): `locations <root>`.

fn main() -> Result<(), String> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("usage: locations <root>")?);
    println!("bases: {:?}", qnc_ingest_base::folder(&root));
    println!("projects: {}", qnc_project_store::ProjectStore::open(&root)?.projects_root_display());
    Ok(())
}
