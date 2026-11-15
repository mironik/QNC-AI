//! The first clips of a base's database (read only): `base_clips <project.db>`.
fn main() {
    let file = std::env::args().nth(1).expect("a database file");
    let conn = rusqlite::Connection::open_with_flags(&file, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).expect("open");
    let mut statement = conn.prepare("SELECT clip_id, name, import_status, final, imported_media_uri, thumbnail_uri FROM clips LIMIT 3").unwrap();
    let rows = statement.query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, i64>(3)?, row.get::<_, Option<String>>(4)?, row.get::<_, Option<String>>(5)?))).unwrap();
    for row in rows {
        println!("{:?}", row.unwrap());
    }
}
