//! The QNC AI tray program (docs/01-wizard-plan.md, step 1-2): one copy, an icon in the tray, a
//! look at the attached disks every two seconds. A card of a camera QNC knows starts the wizard
//! with that card; any other new disk is only a notice. It loads no AI model, opens no project and
//! writes nothing.

#![windows_subsystem = "windows"]

use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use qnc_card_watch::{CardWatch, Found};
use qnc_dev_diagnostics::{log_line, DiagnosticsStream};
use qnc_tray::{MenuItem, OneCopy, Tray, TrayEvent};
use qnc_wizard_text::{fill, text};

const OPEN_WIZARD: u32 = 1;
const QUIT: u32 = 2;
/// How often the disks are looked at.
const LOOK: Duration = Duration::from_secs(2);
/// How often a missing camera catalog is read again.
const CATALOG_AGAIN: Duration = Duration::from_secs(60);
/// How many looks a card that cannot be read yet gets (a card waking up).
const READ_TRIES: u32 = 5;

fn log(line: String) {
    log_line(DiagnosticsStream::Player, format!("ai-tray {line}"));
}

fn main() {
    qnc_dev_diagnostics::log_panics("qnc-ai-tray");
    let _one = match OneCopy::take("QNC-AI-tray") {
        Ok(Some(one)) => one,
        Ok(None) => return,
        Err(error) => {
            log(format!("not started: {error}"));
            return;
        }
    };
    let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let menu = vec![MenuItem { id: OPEN_WIZARD, text: text("tray.open_wizard") }, MenuItem { id: QUIT, text: text("tray.quit") }];
    let tray = match Tray::start(&text("tray.tip"), menu) {
        Ok(tray) => tray,
        Err(error) => {
            log(format!("not started: {error}"));
            return;
        }
    };
    log("started".into());
    let mut catalog = None;
    let mut catalog_read: Option<Instant> = None;
    let mut watch = CardWatch::new();
    let mut tries: std::collections::BTreeMap<String, u32> = Default::default();
    let mut looked: Option<Instant> = None;
    loop {
        while let Some(event) = tray.try_event() {
            match event {
                TrayEvent::Menu(QUIT) => {
                    log("quit".into());
                    return;
                }
                TrayEvent::Menu(OPEN_WIZARD) | TrayEvent::Clicked => start_wizard(&tray, None, &[]),
                TrayEvent::Menu(_) => {}
            }
        }
        if catalog.is_none() && catalog_read.is_none_or(|at| at.elapsed() >= CATALOG_AGAIN) {
            catalog_read = Some(Instant::now());
            match qnc_card_watch::catalog(&root) {
                Ok(read) => catalog = Some(read),
                Err(error) => {
                    log(format!("no camera catalog: {error}"));
                    tray.notify(&text("tray.title"), &fill("tray.no_catalog", &[("error", &error)]));
                }
            }
        }
        if let Some(catalog) = &catalog {
            if looked.is_none_or(|at| at.elapsed() >= LOOK) {
                looked = Some(Instant::now());
                for found in watch.look(catalog) {
                    match found {
                        Found::Card { volume, cameras, patterns } => {
                            let disk = volume.root.display().to_string();
                            log(format!("card disk={disk} serial={} patterns={patterns:?}", volume.serial_number));
                            tray.notify(&text("tray.title"), &fill("tray.card", &[("disk", &disk), ("camera", &cameras.join(", "))]));
                            start_wizard(&tray, Some(&volume.root), &cameras);
                        }
                        Found::Unknown { volume } => {
                            let disk = volume.root.display().to_string();
                            log(format!("unknown disk={disk} serial={}", volume.serial_number));
                            tray.notify(&text("tray.title"), &fill("tray.unknown", &[("disk", &disk)]));
                        }
                        Found::Unreadable { volume, error } => {
                            let disk = volume.root.display().to_string();
                            let count = tries.entry(format!("{disk}|{}", volume.serial_number)).or_default();
                            *count += 1;
                            log(format!("unreadable disk={disk} try={count} error={error}"));
                            if *count < READ_TRIES {
                                watch.forget(&volume);
                            } else {
                                tray.notify(&text("tray.title"), &fill("tray.unreadable", &[("disk", &disk), ("error", &error)]));
                            }
                        }
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

/// The wizard beside this program, with the card when there is one.
fn start_wizard(tray: &Tray, card: Option<&Path>, cameras: &[String]) {
    let program = beside("qnc-wizard");
    let mut command = Command::new(&program);
    if let Some(card) = card {
        command.arg("--card").arg(card);
        for camera in cameras {
            command.arg("--camera").arg(camera);
        }
    }
    if let Err(error) = command.spawn() {
        log(format!("wizard not started: {} {error}", program.display()));
        tray.notify(&text("tray.title"), &fill("tray.no_wizard", &[("error", &error.to_string())]));
    }
}

fn beside(program: &str) -> PathBuf {
    let name = format!("{program}{}", std::env::consts::EXE_SUFFIX);
    std::env::current_exe().ok().and_then(|exe| exe.parent().map(|folder| folder.join(&name))).unwrap_or_else(|| PathBuf::from(name))
}
