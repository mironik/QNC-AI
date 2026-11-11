//! The next program of the wizard opened (user 2026-11-12: Uvezi closes Ingest and opens the new
//! project, Create opens AI Media Assist). The program is the one beside the program running,
//! started in the same QNC-AI root (`QNC_ROOT`); nothing waits for it. Forms only ask for it.

use std::path::{Path, PathBuf};

/// The program `name` beside the program running (with this system's suffix).
pub fn beside(name: &str) -> Result<PathBuf, String> {
    let exe = std::env::current_exe().map_err(|error| error.to_string())?;
    let folder = exe.parent().ok_or("the program running has no folder")?;
    Ok(folder.join(format!("{name}{}", std::env::consts::EXE_SUFFIX)))
}

/// Opens the program `name` beside the one running, in `root`; gives the program opened.
pub fn open(root: &Path, name: &str) -> Result<PathBuf, String> {
    let program = beside(name)?;
    std::process::Command::new(&program)
        .current_dir(root)
        .env("QNC_ROOT", root)
        .spawn()
        .map(|_| program.clone())
        .map_err(|error| format!("{}: {error}", program.display()))
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_program_is_looked_for_beside_the_one_running() {
        let program = super::beside("qnc-next").unwrap();
        assert_eq!(program.parent(), std::env::current_exe().unwrap().parent());
        assert!(program.file_name().unwrap().to_string_lossy().starts_with("qnc-next"));
    }
}
