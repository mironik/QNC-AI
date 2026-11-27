//! Opens a database once for writing so a journal left behind is rolled back where it may be
//! deleted (a copy in a temporary folder): `settle FILE`.

fn main() -> Result<(), String> {
    let file = std::env::args().nth(1).ok_or("FILE")?;
    let conn = rusqlite::Connection::open(&file).map_err(|error| error.to_string())?;
    let check: String = conn.query_row("PRAGMA integrity_check", [], |row| row.get(0)).map_err(|error| error.to_string())?;
    println!("integrity {check}");
    Ok(())
}
