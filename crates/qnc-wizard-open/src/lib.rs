//! Opens the QNC Wizard's desktop (user 2026-11-18, the tray's Breaking news: "čim je kopija
//! gotova, otvara se aplikacija"). Which forms it shows is the project's template: the desktop
//! reads it from the active project, this block only starts it.
//!
//! The desktop is `qnc-app` beside this program; classic QNC has a program of the same name in
//! another folder, so a running one counts only when it is this very file.

use std::path::{Path, PathBuf};

/// The program of the Wizard's desktop.
pub const DESKTOP: &str = "qnc-app";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    /// It already ran: nothing was started.
    Running,
    Started,
}

/// Starts the Wizard's desktop for the QNC root `root` unless it already runs.
pub fn open(root: &Path) -> Result<Opened, String> {
    let desktop = qnc_plugin_process::beside_application(DESKTOP)?;
    if runs(&desktop) {
        return Ok(Opened::Running);
    }
    let mut command = std::process::Command::new(&desktop);
    command.current_dir(root).env("QNC_ROOT", root);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0000_0008); // detached: it outlives the window that started it
    }
    command.spawn().map(|_| Opened::Started).map_err(|error| format!("{}: {error}", desktop.display()))
}

/// Whether a process of this very program runs now.
fn runs(program: &Path) -> bool {
    let wanted = normal(program);
    running_paths().iter().any(|path| normal(path) == wanted)
}

fn normal(path: &Path) -> String {
    let path = std::fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    path.to_string_lossy().trim_start_matches(r"\?\").to_lowercase()
}

/// The full paths of the running programs (those this user may ask about).
#[cfg(windows)]
fn running_paths() -> Vec<PathBuf> {
    use windows_sys::Win32::Foundation::{CloseHandle, INVALID_HANDLE_VALUE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    use windows_sys::Win32::System::Threading::{OpenProcess, QueryFullProcessImageNameW, PROCESS_QUERY_LIMITED_INFORMATION};
    let mut paths = Vec::new();
    let name = format!("{DESKTOP}.exe");
    // SAFETY: a snapshot this function owns and closes; the entry and the buffer are sized; each
    // handle is closed.
    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
        if snapshot == INVALID_HANDLE_VALUE {
            return paths;
        }
        let mut entry: PROCESSENTRY32W = std::mem::zeroed();
        entry.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
        let mut more = Process32FirstW(snapshot, &mut entry) != 0;
        while more {
            let length = entry.szExeFile.iter().position(|c| *c == 0).unwrap_or(entry.szExeFile.len());
            if String::from_utf16_lossy(&entry.szExeFile[..length]).eq_ignore_ascii_case(&name) {
                let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, entry.th32ProcessID);
                if !process.is_null() {
                    let mut buffer = [0u16; 1024];
                    let mut size = buffer.len() as u32;
                    if QueryFullProcessImageNameW(process, 0, buffer.as_mut_ptr(), &mut size) != 0 {
                        paths.push(PathBuf::from(String::from_utf16_lossy(&buffer[..size as usize])));
                    }
                    CloseHandle(process);
                }
            }
            more = Process32NextW(snapshot, &mut entry) != 0;
        }
        CloseHandle(snapshot);
    }
    paths
}

#[cfg(not(windows))]
fn running_paths() -> Vec<PathBuf> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_program_of_the_same_name_elsewhere_is_not_this_one() {
        // This test's own program runs; a file of its name in another folder does not.
        let me = std::env::current_exe().unwrap();
        assert_eq!(normal(&me), normal(&me));
        assert_ne!(normal(&me), normal(&std::env::temp_dir().join(me.file_name().unwrap())));
        assert!(!runs(&std::env::temp_dir().join("qnc-app.exe")));
    }
}
