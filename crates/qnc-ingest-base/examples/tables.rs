//! The tables of a database and their row counts, read only: `tables FILE...`.

fn main() -> Result<(), String> {
    for file in std::env::args().skip(1) {
        println!("== {file}");
        let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|error| error.to_string())?;
        let mut statement = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name").map_err(|error| error.to_string())?;
        let names: Vec<String> = statement.query_map([], |row| row.get(0)).map_err(|error| error.to_string())?.collect::<Result<_, _>>().map_err(|error| error.to_string())?;
        for name in names {
            let count: i64 = conn.query_row(&format!("SELECT count(*) FROM \"{name}\""), [], |row| row.get(0)).unwrap_or(-1);
            println!("{count:>8} {name}");
        }
    }
    Ok(())
}
