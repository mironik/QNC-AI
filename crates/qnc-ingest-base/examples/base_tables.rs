//! The tables of a database (read only) and their row counts: `base_tables <file>`.
fn main() {
    let file = std::env::args().nth(1).expect("a database file");
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).expect("open");
    let mut statement = conn.prepare("SELECT name, type FROM sqlite_master WHERE type IN ('table','view') ORDER BY type, name").unwrap();
    let names: Vec<(String, String)> = statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().map(Result::unwrap).collect();
    for (name, kind) in names {
        let count: i64 = conn.query_row(&format!("SELECT count(*) FROM \"{name}\""), [], |row| row.get(0)).unwrap_or(-1);
        let has_clip: bool = conn.prepare(&format!("SELECT * FROM \"{name}\" LIMIT 0")).map(|s| s.column_names().iter().any(|c| *c == "clip_id")).unwrap_or(false);
        println!("{kind:5} {name:40} {count:>8} {}", if has_clip { "clip_id" } else { "" });
    }
}
