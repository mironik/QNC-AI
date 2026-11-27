//! The owners make their tables in a story's database (as Create project does before the copy of a
//! base): `prepare_story QNC_ROOT STORY_ID [STORY_DB]`; a STORY_DB given is used instead of the
//! story's own file (for copies of databases).
fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let (root, story) = (args.next().ok_or("QNC_ROOT")?, args.next().ok_or("STORY_ID")?);
    let (file, uri) = qnc_project_store::ProjectStore::open(&root)?.project_db_binding(&story)?;
    let file = args.next().map_or(file, std::path::PathBuf::from);
    qnc_ingest_base::prepare_story(&file, &uri)?;
    println!("prepared {}", file.display());
    Ok(())
}
