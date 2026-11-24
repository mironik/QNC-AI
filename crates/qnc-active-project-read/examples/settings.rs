//! The active project of a QNC root and its work settings as the blocks read them (read only):
//! `settings <root>`.

fn main() -> Result<(), String> {
    let root = std::path::PathBuf::from(std::env::args().nth(1).ok_or("usage: settings <root>")?);
    let snapshot = qnc_active_project_read::read_from_root(&root).map_err(|error| error.to_string())?;
    println!("{snapshot:#?}");
    Ok(())
}
