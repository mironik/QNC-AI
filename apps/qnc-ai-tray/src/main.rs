//! The QNC AI tray (user 2026-11-16: "traj ima samo postavke i automatsko pokretanje ingesta po
//! poznatim karticama i uvoz iz direktorija, sve drugo više ne treba biti tamo"; the administration
//! of the bases is an administrator's form, as in real ingests). It waits in the tray, one copy,
//! and looks at the attached disks every two seconds: a card of a camera QNC knows starts its
//! ingest window at once. Where a card's or a folder's ingest writes is the tray's orchestrator's
//! (`qnc-ingest-orchestrator`, chosen in the settings); the order of the steps is the template's.
//! It loads no AI model and writes nothing itself.

#![windows_subsystem = "windows"]

use std::time::{Duration, Instant};

use qnc_card_watch::{CardWatch, Found};
use qnc_dev_diagnostics::{log_line, DiagnosticsStream};
use qnc_tray::{MenuItem, OneCopy, Tray, TrayEvent};
use qnc_wizard_text::{fill, text};

const STATUS: u32 = 1;
const QUIT: u32 = 3;
const ADMIN: u32 = 4;
const INGEST_FOLDER: u32 = 8;
/// How often the disks are looked at.
const LOOK: Duration = Duration::from_secs(2);
/// How often a missing camera catalog is read again.
const CATALOG_AGAIN: Duration = Duration::from_secs(60);
/// How many looks a card that cannot be read yet gets (a card waking up).
const READ_TRIES: u32 = 5;

fn log(line: String) {
    log_line(DiagnosticsStream::Player, format!("ai-tray {line}"));
}

/// The menu: what runs (a line that only tells), Import from a folder, Settings, Quit.
fn menu(running: usize) -> Vec<MenuItem> {
    let line = if running == 0 { text("tray.waiting") } else { fill("tray.ingesting", &[("count", &running.to_string())]) };
    vec![
        MenuItem { id: STATUS, text: line, enabled: false },
        // A source that is no card (a folder on a disk, the LAN): the user picks it in Ingest's
        // browser (user 2026-11-13).
        MenuItem { id: INGEST_FOLDER, text: text("tray.ingest_folder"), enabled: true },
        // The administrator's way in: the bases and, from there, the settings (user 2026-11-16).
        MenuItem { id: ADMIN, text: text("tray.admin"), enabled: true },
        MenuItem { id: QUIT, text: text("tray.quit"), enabled: true },
    ]
}

fn main() {
    qnc_dev_diagnostics::log_panics("qnc-ai-tray");
    let _one = match OneCopy::take("QNC-AI-tray") {
        Ok(Some(one)) => one,
        Ok(None) => {
            log("not started: another copy runs".into());
            return;
        }
        Err(error) => {
            log(format!("not started: {error}"));
            return;
        }
    };
    let root = qnc_dev_diagnostics::resolve_qnc_root().unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let tray = match Tray::start(&format!("{}: {}", text("tray.title"), text("tray.waiting")), menu(0), Some(qnc_app_mark::qnc_mark)) {
        Ok(tray) => tray,
        Err(error) => {
            log(format!("not started: {error}"));
            return;
        }
    };
    log("started".into());
    // A card of a camera the tray knows: the system does not open File Explorer for it, the
    // tray's ingest opens (user 2026-11-14). Any other disk: AutoPlay as the user set it.
    let _autoplay = {
        let root = root.clone();
        qnc_autoplay_guard::guard(Box::new(move |disk, serial| {
            let volume = qnc_source_volumes::Volume { root: disk.to_path_buf(), serial_number: format!("{serial:08x}"), volume_name: None };
            let card = qnc_card_watch::catalog(&root).is_ok_and(|catalog| matches!(qnc_card_watch::what_is(&catalog, volume), Found::Card { .. }));
            log(format!("autoplay disk={} card={card}", disk.display()));
            card
        }))
        .map_err(|error| log(format!("autoplay not taken: {error}")))
        .ok()
    };
    // The camera profiles, to name a card's camera by its model.
    let (profiles, _) = qnc_camera_profiles::read_profiles(&root.join(qnc_camera_profiles::PROFILE_DIR));
    log(format!("root {}", root.display()));
    let mut ingest = Windows::default();
    let mut shown: Option<usize> = None;
    let mut catalog = None;
    let mut catalog_read: Option<Instant> = None;
    let mut watch = CardWatch::new();
    let mut tries: std::collections::BTreeMap<String, u32> = Default::default();
    let mut looked: Option<Instant> = None;
    // The question where a folder goes, and the base it would go into besides a new one.
    let mut folder_asked: Option<(std::sync::mpsc::Receiver<qnc_tray::Answer>, String)> = None;
    loop {
        let running = ingest.running();
        if shown != Some(running) {
            tray.set_menu(menu(running));
            let line = if running == 0 { text("tray.waiting") } else { fill("tray.ingesting", &[("count", &running.to_string())]) };
            tray.set_tip(&format!("{}: {line}", text("tray.title")));
            shown = Some(running);
        }
        while let Some(event) = tray.try_event() {
            match event {
                TrayEvent::Menu(QUIT) => {
                    log("quit".into());
                    return;
                }
                TrayEvent::Menu(ADMIN) => open_admin(&tray, &root),
                // A folder goes where the tray's orchestrator says, with its own window; the user is
                // asked whether into a new base or the one Ingest works in (user 2026-11-16).
                TrayEvent::Menu(INGEST_FOLDER) if folder_asked.is_none() => match orchestrator(&tray, &root).map(|orchestrator| orchestrator.folder_choice(&root)) {
                    Some(Ok(Some(base))) => {
                        let ask = fill("tray.folder_ask", &[("base", &base.shown())]);
                        folder_asked = Some((tray.ask(&text("tray.title"), &ask), base.id));
                    }
                    Some(Ok(None)) => open_folder(&tray, &root, &mut ingest, None),
                    Some(Err(error)) => fail(&tray, error),
                    None => {}
                },
                TrayEvent::Menu(_) => {}
            }
        }
        if let Some(answer) = folder_asked.as_ref().and_then(|(answer, _)| answer.try_recv().ok()) {
            let (_, base) = folder_asked.take().expect("asked");
            log(format!("folder asked: {answer:?}"));
            match answer {
                qnc_tray::Answer::Yes => open_folder(&tray, &root, &mut ingest, None),
                qnc_tray::Answer::No => open_folder(&tray, &root, &mut ingest, Some(&base)),
                qnc_tray::Answer::Cancel => {}
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
                // The first look takes the disks in as new: a card starts its ingest, the rest only logs.
                let first = looked.is_none();
                looked = Some(Instant::now());
                for found in watch.look(catalog) {
                    match found {
                        Found::Card { volume, cameras, patterns } => {
                            let disk = volume.root.display().to_string();
                            // The model the camera profiles recognise ("Sony ILME-FX3A"), else the
                            // camera families of the catalog (user 2026-11-13).
                            let camera = qnc_card_watch::camera_of(catalog, &profiles, &volume).unwrap_or_else(|| cameras.join(", "));
                            log(format!("card disk={disk} serial={} camera={camera} patterns={patterns:?}", volume.serial_number));
                            if ingest.runs_card(&volume.serial_number) {
                                continue; // its window takes the card back
                            }
                            let card = volume.volume_name.clone().unwrap_or_else(|| disk.clone());
                            // A card an earlier base already has whole is only told, no base is
                            // made (user 2026-11-17: "tray neka samo javi da je kartica već
                            // uvezena"); not read is ingested as before.
                            let known = qnc_card_watch::card_originals(catalog, &volume).and_then(|originals| qnc_ingest_base::known_card(&root, &volume.source_uri(), &originals));
                            match known {
                                // Its ingest was stopped before the end: it goes on in that base.
                                Ok(Some(known)) if known.new.is_empty() && known.unfinished > 0 => {
                                    log(format!("card serial={} known base={} unfinished={}: goes on in that base", volume.serial_number, known.base.label(), known.unfinished));
                                    tray.notify(&text("tray.title"), &fill("tray.card_goes_on", &[("card", &card), ("camera", &camera), ("base", &known.base.shown()), ("count", &known.unfinished.to_string())]));
                                    open_window(&tray, &root, &mut ingest, known.base, Some(&volume.serial_number), false, true);
                                    continue;
                                }
                                Ok(Some(known)) if known.new.is_empty() => {
                                    log(format!("card serial={} known base={} had={}: nothing new, no base made", volume.serial_number, known.base.label(), known.had));
                                    tray.notify(&text("tray.title"), &fill("tray.card_known", &[("card", &card), ("camera", &camera), ("base", &known.base.shown()), ("count", &known.had.to_string())]));
                                    continue;
                                }
                                Ok(_) => {}
                                Err(error) => log(format!("card serial={} not compared with the bases: {error}", volume.serial_number)),
                            }
                            let Some(orchestrator) = orchestrator(&tray, &root) else { continue };
                            match orchestrator.card_base(&root, &camera) {
                                Ok(base) => {
                                    tray.notify(&text("tray.title"), &fill("tray.card_ingest", &[("card", &card), ("camera", &camera), ("base", base.label())]));
                                    open_window(&tray, &root, &mut ingest, base, Some(&volume.serial_number), false, false);
                                }
                                Err(error) => fail(&tray, error),
                            }
                        }
                        Found::Unknown { volume } => {
                            let disk = volume.root.display().to_string();
                            log(format!("unknown disk={disk} serial={}", volume.serial_number));
                            if !first {
                                tray.notify(&text("tray.title"), &fill("tray.unknown", &[("disk", &disk)]));
                            }
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
                        Found::Gone { volume } => log(format!("gone disk={} serial={}", volume.root.display(), volume.serial_number)),
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

/// A folder's ingest window: in a new base, or in the base `into` (the user's answer).
fn open_folder(tray: &Tray, root: &std::path::Path, ingest: &mut Windows, into: Option<&str>) {
    let Some(orchestrator) = orchestrator(tray, root) else { return };
    match orchestrator.folder_base(root, &text("tray.folder_base"), into) {
        Ok(base) => open_window(tray, root, ingest, base, None, true, false),
        Err(error) => fail(tray, error),
    }
}

/// The orchestrator chosen for the tray on this computer; not chosen is told, nothing is ingested.
fn orchestrator(tray: &Tray, root: &std::path::Path) -> Option<qnc_ingest_orchestrator::Orchestrator> {
    match qnc_ingest_orchestrator::chosen(root, qnc_ingest_orchestrator::TRAY) {
        Ok(orchestrator) => Some(orchestrator),
        Err(error) => {
            log(format!("no orchestrator: {error}"));
            tray.alert("orchestrator", &text("tray.title"), &fill("tray.no_orchestrator", &[("error", &error)]), &text("tray.ok"));
            None
        }
    }
}

fn fail(tray: &Tray, error: String) {
    log(format!("ingest failed: {error}"));
    tray.alert("ingest", &text("tray.title"), &fill("tray.ingest_failed", &[("error", &error)]), &text("tray.ok"));
}

/// The ingest windows this tray started: one per base, each with the card it ingests (user
/// 2026-11-14: each base its own window and background worker, at the same time).
#[derive(Default)]
struct Windows {
    open: Vec<(String, Option<String>, std::process::Child)>,
}

impl Windows {
    /// The windows that ended are let go.
    fn prune(&mut self) {
        self.open.retain_mut(|(_, _, child)| matches!(child.try_wait(), Ok(None)));
    }

    fn running(&mut self) -> usize {
        self.prune();
        self.open.len()
    }

    /// Whether a window ingests the card of volume serial `serial`.
    fn runs_card(&mut self, serial: &str) -> bool {
        self.prune();
        self.open.iter().any(|(_, card, _)| card.as_deref().is_some_and(|card| card.eq_ignore_ascii_case(serial)))
    }

    /// Whether a window works in the base `id`.
    fn runs_base(&mut self, id: &str) -> bool {
        self.prune();
        self.open.iter().any(|(base, _, _)| base == id)
    }
}

/// The ingest window of a base (beside this program), pinned to the base the orchestrator gave
/// (`QNC_WORK_PROJECT`); with `folder` it first shows Ingest's browser. `card`: the one card it
/// ingests. Its helper programs share one hidden console.
fn open_window(tray: &Tray, root: &std::path::Path, ingest: &mut Windows, base: qnc_ingest_base::IngestBase, card: Option<&str>, folder: bool, goes_on: bool) {
    log(format!("ingest base {} {} card={card:?} folder={folder}", base.name, base.uri));
    if ingest.runs_base(&base.id) {
        return;
    }
    let program = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|folder| folder.join(format!("qnc-ai-card-ingest{}", std::env::consts::EXE_SUFFIX))))
        .unwrap_or_default();
    let mut command = std::process::Command::new(&program);
    command.current_dir(root).env("QNC_ROOT", root).env(qnc_work_settings::PINNED_PROJECT_ENV, &base.id);
    if folder {
        command.arg("--folder");
    }
    if goes_on {
        command.arg("--goes-on");
    }
    if let Some(card) = card {
        command.arg("--card").arg(card);
    }
    // Where it opens: after the windows open now, not behind them (user 2026-11-14).
    command.arg("--place").arg(ingest.running().to_string());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    match command.spawn() {
        Ok(child) => {
            log(format!("ingest opened {} base={}", program.display(), base.id));
            ingest.open.push((base.id.clone(), card.map(str::to_string), child));
        }
        Err(error) => fail(tray, error.to_string()),
    }
}

/// The administrator's form, beside this program (it keeps one copy itself).
fn open_admin(tray: &Tray, root: &std::path::Path) {
    let program = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|folder| folder.join(format!("qnc-ai-admin{}", std::env::consts::EXE_SUFFIX))))
        .unwrap_or_default();
    match std::process::Command::new(&program).current_dir(root).env("QNC_ROOT", root).spawn() {
        Ok(_) => log("administration opened".into()),
        Err(error) => {
            log(format!("administration failed {}: {error}", program.display()));
            tray.alert("admin", &text("tray.title"), &fill("tray.admin_failed", &[("error", &error.to_string())]), &text("tray.ok"));
        }
    }
}
