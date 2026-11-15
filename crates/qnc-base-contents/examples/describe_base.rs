//! What the shot analysis says a base holds, read only (nothing is written):
//! `describe_base <qnc root> [base id]` (the newest base without an id).
fn main() -> Result<(), String> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("a QNC root")?);
    let bases = qnc_ingest_base::all(&root)?;
    let id = std::env::args().nth(2).or_else(|| bases.first().map(|base| base.id.clone())).ok_or("no base")?;
    let store = qnc_project_store::ProjectStore::open(&root)?;
    let (file, workspace) = store.project_db_binding(&id)?;
    let target = qnc_db_broker::ProjectDbTarget::from_owner_binding(&file, &qnc_db_broker::project_db_uri(&workspace)?)?;
    println!("{id}\n{}", qnc_base_contents::describe(&root, &target, target.uri())?);
    Ok(())
}
