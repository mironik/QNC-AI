//! An isolated root to try the wizard on (AGENTS.md QNC 0.4: never write into a real project):
//! `iso_root <QNC root> <new root>` copies the project registry and the active project's database
//! of the QNC root (read only, VACUUM INTO) into the new root and points the copy's project
//! location at the copied project. Media is not copied: the copy's project reads the same cards.

use std::path::{Path, PathBuf};

fn main() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    let (qnc, out) = match args.as_slice() {
        [_, qnc, out] => (PathBuf::from(qnc), PathBuf::from(out)),
        _ => return Err("iso_root <QNC root> <new root>".into()),
    };
    if out.exists() {
        return Err(format!("{} exists: it must be new", out.display()));
    }
    std::fs::create_dir_all(out.join("data")).map_err(|e| e.to_string())?;
    std::fs::create_dir_all(out.join("contracts")).map_err(|e| e.to_string())?;
    std::fs::write(out.join("AGENTS.md"), "Isolated root of the QNC AI wizard.\n").map_err(|e| e.to_string())?;
    std::fs::write(out.join("Cargo.toml"), "# isolated root\n").map_err(|e| e.to_string())?;
    let registry = out.join("data").join("qnc-projects.db");
    copy(&qnc.join("data").join("qnc-projects.db"), &registry)?;
    let db = rusqlite::Connection::open(&registry).map_err(|e| e.to_string())?;
    let active: String = db
        .query_row("SELECT value FROM app_settings WHERE key = 'active_project_id'", [], |row| row.get(0))
        .map_err(|e| format!("active project: {e}"))?;
    let from: String = db
        .query_row("SELECT local_path FROM project_storage_locations WHERE project_id = ?1", [&active], |row| row.get(0))
        .map_err(|e| format!("project location: {e}"))?;
    let folder = out.join("projects").join(&active);
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    copy(&Path::new(&from).join("project.db"), &folder.join("project.db"))?;
    db.execute("UPDATE project_storage_locations SET local_path = ?1 WHERE project_id = ?2", rusqlite::params![folder.to_string_lossy(), active])
        .map_err(|e| e.to_string())?;
    println!("project {active}: {from} -> {}", folder.display());
    Ok(())
}

fn copy(from: &Path, to: &Path) -> Result<(), String> {
    let source = rusqlite::Connection::open_with_flags(from, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(|e| format!("{}: {e}", from.display()))?;
    source.execute("VACUUM INTO ?1", [to.to_string_lossy()]).map_err(|e| format!("{}: {e}", from.display()))?;
    Ok(())
}
