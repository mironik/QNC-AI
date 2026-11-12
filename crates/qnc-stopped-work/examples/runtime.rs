//! Prints the runtime records of a project database, read only (diagnostic).
fn main() -> Result<(), String> {
    let file = std::env::args().nth(1).ok_or("usage: runtime <project.db>")?;
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())?;
    let mut statement = conn
        .prepare("SELECT key, value, CAST(strftime('%s','now') AS INTEGER) - updated_at FROM ingest_runtime ORDER BY key")
        .map_err(|e| e.to_string())?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, i64>(2)?)))
        .map_err(|e| e.to_string())?;
    for row in rows {
        let (key, value, age) = row.map_err(|e| e.to_string())?;
        println!("{key} = {value} ({age} s)");
    }
    Ok(())
}
