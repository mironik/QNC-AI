//! Reads the stored wave of one clip of the active project (read only): whether it is there and
//! coarse. `cargo run --example read_wave -- <QNC root> <clip id>`.

fn main() -> Result<(), String> {
    let mut args = std::env::args().skip(1);
    let root = std::path::PathBuf::from(args.next().ok_or("QNC root")?);
    let clip = args.next().ok_or("clip id")?;
    let active = qnc_active_project_read::ActiveProjectReader::from_root(&root).map_err(|e| e.to_string())?;
    let snapshot = active.read().map_err(|e| e.to_string())?;
    println!("project {} workspace {:?}", snapshot.project_id, active.settings_reader().local_workspace_dir(&snapshot.settings));
    let target = qnc_db_broker::ProjectDbTarget::for_project(active.settings_reader(), &snapshot.settings)?;
    let mut reader = qnc_artifact_db::ArtifactReader::open(&target)?;
    match reader.read_wave(&clip)? {
        Some(record) => println!("wave coarse={} version={} peaks={}", record.coarse, record.render_version, record.peak_count),
        None => println!("no wave"),
    }
    Ok(())
}
