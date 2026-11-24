//! Read-only: the newest undo steps of a copy of a project database (the blocks as they were
//! before each change). Usage: undo_raw PROJECT_DB_COPY [STEPS]
fn main() -> Result<(), String> {
    let db = std::env::args().nth(1).ok_or("usage: undo_raw PROJECT_DB_COPY [STEPS]")?;
    let steps: i64 = std::env::args().nth(2).and_then(|n| n.parse().ok()).unwrap_or(3);
    let conn = rusqlite::Connection::open_with_flags(&db, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| e.to_string())?;
    let mut statement = conn.prepare("SELECT step, stack, rows_json FROM script_undo ORDER BY step DESC LIMIT ?1").map_err(|e| e.to_string())?;
    let rows = statement.query_map([steps], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?))).map_err(|e| e.to_string())?;
    for row in rows {
        let (step, stack, json) = row.map_err(|e| e.to_string())?;
        println!("== step {step} {stack}");
        let blocks: Vec<serde_json::Value> = serde_json::from_str(&json).map_err(|e| e.to_string())?;
        for block in blocks {
            println!("{}", serde_json::to_string(&block).unwrap_or_default());
        }
    }
    Ok(())
}
