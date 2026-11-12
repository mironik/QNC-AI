//! Who made each Ingest base and whether this user may delete it (read only).
fn main() -> Result<(), String> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("usage: owner <root>")?);
    println!("administrator: {}", qnc_workstation_identity::is_administrator());
    for base in qnc_ingest_base::all(&root)? {
        println!("{}: owner {:?}, may delete {:?}", base.label(), qnc_ingest_base::owner(&root, &base.id)?, qnc_ingest_base::may_delete(&root, &base.id));
    }
    Ok(())
}
