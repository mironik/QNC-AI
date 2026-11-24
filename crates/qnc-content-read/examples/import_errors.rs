//! Prints the clips whose import failed and why (read only; nothing is written).
//! `cargo run -p qnc-content-read --example import_errors -- <path to project.db>`

use rusqlite::{Connection, OpenFlags};

fn main() -> Result<(), String> {
    let path = std::env::args().nth(1).ok_or("give the path of a project.db")?;
    let db = Connection::open_with_flags(&path, OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX).map_err(|error| error.to_string())?;
    let mut statement = db
        .prepare("SELECT name, import_status, COALESCE(import_error, ''), original_uri FROM clips WHERE import_status NOT IN ('imported', 'done') ORDER BY name")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?)))
        .map_err(|error| error.to_string())?;
    for row in rows {
        let (name, status, error, uri) = row.map_err(|error| error.to_string())?;
        println!("{name} {status} uri={uri} error={error}");
    }
    Ok(())
}
