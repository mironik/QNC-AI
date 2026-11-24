//! The clips of the active project of a QNC root with their import status and whether each has
//! its filmstrip and wave (read only): `catalog <root>`.

fn main() -> Result<(), String> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("usage: catalog <root>")?);
    let active = qnc_active_project_read::ActiveProjectReader::from_root(&root).map_err(|error| error.to_string())?;
    let snapshot = active.read().map_err(|error| error.to_string())?;
    let content = qnc_content_read::ContentReader::for_project(active.settings_reader(), &snapshot.settings)?;
    let mut statuses = std::collections::BTreeMap::<String, usize>::new();
    let (mut filmstrips, mut waves) = (0, 0);
    for (clip, _) in content.clip_posters()? {
        if let Some(head) = content.summaries()?.into_iter().find(|summary| summary.clip_id == clip) {
            *statuses.entry(head.import_status).or_default() += 1;
        } else {
            *statuses.entry("not listed".into()).or_default() += 1;
        }
        filmstrips += usize::from(content.filmstrip(&clip)?.is_some());
        waves += usize::from(content.wave(&clip)?.is_some());
    }
    println!("{} statuses={statuses:?} filmstrips={filmstrips} waves={waves}", snapshot.settings.project_id);
    println!("waiting records: {}", content.records_waiting()?.len());
    // The reasons the import gave, read only from the public view.
    let conn = rusqlite::Connection::open_with_flags(content.database_file(), rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
    let mut statement = conn.prepare("SELECT import_error, count(*) FROM public_clips WHERE import_error IS NOT NULL GROUP BY import_error").map_err(|error| error.to_string())?;
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, i64>(1)?))).map_err(|error| error.to_string())?;
    for row in rows {
        let (error, count) = row.map_err(|error| error.to_string())?;
        println!("import error x{count}: {error}");
    }
    Ok(())
}
