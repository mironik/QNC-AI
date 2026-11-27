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

/// The action of a form's button that opens MA Script (user 2026-11-22: "Idi na Script (OFF)"
/// after Ažuriraj in AI Media Assist): the statements made are in the project's database,
/// MA Script reads them from there.
pub const GO_TO_SCRIPT: &str = "go_to_script";

/// The form of the story header, opened before MA Script when the user has no story yet.
pub const HEADER_FORM: &str = "qnc-story-header-form";

/// Where "Idi na Script (OFF)" goes (user 2026-11-27: the header's form is not tied to the Script,
/// its confirm would make a project and break the links of the one there): a story's project
/// active, MA Script; an Ingest base active, the story this user goes on with on that base (made
/// the active one) and MA Script, or the header's form when there is none (one base may give
/// several projects, so the user and the project's state decide, not the base). Decided off the
/// caller's thread; what it opened goes to the player log. The form asking stays open.
pub fn go_to_script() -> Result<(), String> {
    let root = qnc_dev_diagnostics::resolve_qnc_root().ok_or("no QNC root for MA Script")?;
    std::thread::Builder::new()
        .name("qnc-go-to-script".into())
        .spawn(move || {
            let line = match script_or_header(&root) {
                Ok(said) => said,
                Err(error) => format!("go-to-script failed: {error}"),
            };
            qnc_dev_diagnostics::log_line(qnc_dev_diagnostics::DiagnosticsStream::Player, line);
        })
        .map(drop)
        .map_err(|error| error.to_string())
}

fn script_or_header(root: &Path) -> Result<String, String> {
    if !qnc_ingest_base::active_is_base(root) {
        return open(root, "qnc-ma-script").map(|program| format!("go-to-script a story's project is active: {}", program.display()));
    }
    let base = qnc_ingest_base::active(root)?.ok_or("no active Ingest base")?;
    match qnc_ingest_base::my_story_of_base(root, &base.id)? {
        Some((story, name)) => {
            qnc_ingest_base::open_story(root, &story)?;
            open(root, "qnc-ma-script").map(|program| format!("go-to-script the user's story {name} on base {} opened: {}", base.id, program.display()))
        }
        None => open(root, HEADER_FORM).map(|program| format!("go-to-script no story of this user on base {}: the header's form {}", base.id, program.display())),
    }
}

/// The environment variable a program opened for a new story carries (user 2026-11-15: "Novi
/// projekt" of the Projects form: Uvezi of Ingest makes a new project, offering none to go on).
pub const NEW_STORY: &str = "QNC_NEW_STORY";

/// Opens `name` as [`open`] does, for a new story ([`NEW_STORY`]).
pub fn open_for_new_story(root: &Path, name: &str) -> Result<PathBuf, String> {
    let program = beside(name)?;
    std::process::Command::new(&program)
        .current_dir(root)
        .env("QNC_ROOT", root)
        .env(NEW_STORY, "1")
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


/// Opens the tray's ingest window for the folder or card `uri`, working in the base `base_id`, as
/// the tray's "Import from a folder" does once the folder is picked (user 2026-11-16).
pub fn open_card_ingest(root: &Path, base_id: &str, uri: &str) -> Result<PathBuf, String> {
    let program = beside("qnc-ai-card-ingest")?;
    std::process::Command::new(&program)
        .current_dir(root)
        .env("QNC_ROOT", root)
        .env(qnc_work_settings::PINNED_PROJECT_ENV, base_id)
        .arg("--folder-uri")
        .arg(uri)
        .spawn()
        .map(|_| program.clone())
        .map_err(|error| format!("{}: {error}", program.display()))
}
