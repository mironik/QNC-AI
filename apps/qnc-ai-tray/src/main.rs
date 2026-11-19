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
const STOP_ALL: u32 = 9;
/// Opens again the form the mode opens after a copy (user 2026-11-19: "nemam šortkat za povratak
/// u aplikaciju ako iz nje izađem").
const OPEN_FORM: u32 = 10;
/// The first of the tray's modes in the menu (`qnc_tray_mode::ALL` in order).
const MODE_FIRST: u32 = 20;
/// How often the disks are looked at.
const LOOK: Duration = Duration::from_secs(2);
/// How often a missing camera catalog is read again.
const CATALOG_AGAIN: Duration = Duration::from_secs(60);
/// How long a card put in settles before its ingest starts.
const SETTLE: Duration = Duration::from_secs(5);
/// How many looks a card that cannot be read yet gets (a card waking up).
const READ_TRIES: u32 = 5;

fn log(line: String) {
    log_line(DiagnosticsStream::Player, format!("ai-tray {line}"));
}

/// The menu: what runs (a line that only tells), Stop all the work (while any runs), the tray's
/// mode, Import from a folder, Admin, Quit.
fn menu(running: usize, active: usize, mode: Option<qnc_tray_mode::Mode>) -> Vec<MenuItem> {
    let mut items = vec![
        MenuItem { id: STATUS, text: status(running, active), enabled: false, check: None },
        // Every process the tray started, with the windows and the background work (user
        // 2026-11-17: "treba indikacija u tray-u i dugme za prekid").
        MenuItem { id: STOP_ALL, text: text("tray.stop_all"), enabled: active > 0, check: None },
    ];
    // How the tray works on this computer (user 2026-11-18): Local with Breaking news or Story, or
    // Net; the one chosen has its box ticked (kept on the computer, read at every start).
    for (i, choice) in qnc_tray_mode::ALL.into_iter().enumerate() {
        items.push(MenuItem { id: MODE_FIRST + i as u32, text: text(choice.label()), enabled: true, check: Some(mode == Some(choice)) });
    }
    items.extend([
        MenuItem { id: OPEN_FORM, text: text("tray.open_form"), enabled: mode.and_then(qnc_tray_mode::Mode::opens).is_some(), check: None },
        // A source that is no card (a folder on a disk, the LAN): the user picks it in Ingest's
        // browser (user 2026-11-13).
        MenuItem { id: INGEST_FOLDER, text: text("tray.ingest_folder"), enabled: true, check: None },
        // The administrator's way in: the bases and, from there, the settings (user 2026-11-16).
        MenuItem { id: ADMIN, text: text("tray.admin"), enabled: true, check: None },
        MenuItem { id: QUIT, text: text("tray.quit"), enabled: true, check: None },
    ]);
    items
}

/// What runs, in one line: the ingest windows, else the work left in the background (a window
/// closed, its worker on), else waiting for a card.
fn status(running: usize, active: usize) -> String {
    if running > 0 {
        fill("tray.ingesting", &[("count", &running.to_string())])
    } else if active > 0 {
        fill("tray.background", &[("count", &active.to_string())])
    } else {
        text("tray.waiting")
    }
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
    // Everything the tray starts and all that starts in turn ends with the tray: quitting, or a
    // crash (user 2026-11-17: the user has no sign of work left in the background).
    let job = match qnc_process_job::Job::new() {
        Ok(job) => job,
        Err(error) => {
            log(format!("not started: {error}"));
            return;
        }
    };
    // The tray's mode on this computer; none chosen yet: the tray only ingests, as before.
    let mut mode = qnc_tray_mode::read(&root).unwrap_or_else(|error| {
        log(format!("mode not read: {error}"));
        None
    });
    log(format!("mode {mode:?}"));
    let tray = match Tray::start(&format!("{}: {}", text("tray.title"), text("tray.waiting")), menu(0, 0, mode), Some(qnc_app_mark::qnc_mark)) {
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
    let mut ingest = Windows { open: Vec::new(), job, admin: None, bases: Vec::new() };
    let mut shown: Option<(usize, usize, Option<qnc_tray_mode::Mode>)> = None;
    // The question before all the work is stopped: true when the user quits.
    let mut stop_asked: Option<(std::sync::mpsc::Receiver<Option<String>>, bool)> = None;
    // The bases to delete once their work has stopped (and whether the tray quits after), and the
    // deletes running.
    let mut to_delete: Option<(Vec<qnc_ingest_base::IngestBase>, bool, Instant)> = None;
    let mut deleting: Vec<(qnc_base_admin::Delete, qnc_ingest_base::IngestBase)> = Vec::new();
    let mut quit_when_deleted = false;
    let mut catalog = None;
    let mut catalog_read: Option<Instant> = None;
    let mut watch = CardWatch::new();
    let mut tries: std::collections::BTreeMap<String, u32> = Default::default();
    // The cards put in that have not settled yet, by volume serial: since when, and when last seen.
    let mut settling: std::collections::BTreeMap<String, (Instant, Instant)> = Default::default();
    let mut looked: Option<Instant> = None;
    // The question where a folder goes, and the base it would go into besides a new one.
    let mut folder_asked: Option<(std::sync::mpsc::Receiver<qnc_tray::Answer>, String)> = None;
    loop {
        let running = ingest.running();
        let active = ingest.work();
        if shown != Some((running, active, mode)) {
            tray.set_menu(menu(running, active, mode));
            tray.set_tip(&format!("{}: {}", text("tray.title"), status(running, active)));
            if shown.is_some_and(|(_, was, _)| (was > 0) != (active > 0)) {
                log(format!("work running={running} processes={active}"));
            }
            shown = Some((running, active, mode));
        }
        // Stopping is asked what becomes of the partial ingest: kept (it goes on when the card is
        // put in again) or deleted (user 2026-11-17: "ako zaustavljam proces onda ne treba nuditi
        // spremanje napravljenog, brisanje ili spremanje").
        if let Some(answer) = stop_asked.as_ref().and_then(|(answer, _)| answer.try_recv().ok()) {
            let (_, quit) = stop_asked.take().expect("asked");
            log(format!("stop asked quit={quit}: {answer:?}"));
            match answer.as_deref() {
                Some(KEEP) if quit => {
                    log("quit: the work stops, the partial ingest kept".into());
                    return;
                }
                Some(KEEP) => {
                    match ingest.job.stop() {
                        Ok(()) => tray.notify(&text("tray.title"), &text("tray.stopped")),
                        Err(error) => fail(&tray, error),
                    }
                    ingest.bases.clear();
                }
                Some(DELETE) => {
                    if let Err(error) = ingest.job.stop() {
                        fail(&tray, error);
                    }
                    // Deleted once nothing of the job holds the base's files any more.
                    to_delete = Some((std::mem::take(&mut ingest.bases), quit, Instant::now()));
                }
                _ => {}
            }
        }
        if let Some((bases, quit, since)) = to_delete.take_if(|(_, _, since)| ingest.job.active() == 0 || since.elapsed() >= Duration::from_secs(10)) {
            log(format!("deleting {} bases quit={quit} after {} ms", bases.len(), since.elapsed().as_millis()));
            for base in bases {
                tray.notify(&text("tray.title"), &fill("tray.base_deleting", &[("base", &base.shown())]));
                deleting.push((qnc_base_admin::Delete::start(&root, &base), base));
            }
            quit_when_deleted = quit;
        }
        deleting.retain(|(delete, base)| match delete.poll() {
            None => true,
            Some(result) => {
                log(format!("base {} deleted: {result:?}", base.id));
                match result {
                    Ok(()) => tray.notify(&text("tray.title"), &fill("tray.base_deleted", &[("base", &base.shown())])),
                    Err(error) => fail(&tray, fill("tray.base_delete_failed", &[("base", &base.shown()), ("error", &error)])),
                }
                false
            }
        });
        if quit_when_deleted && to_delete.is_none() && deleting.is_empty() {
            log("quit: the partial ingest deleted".into());
            return;
        }
        while let Some(event) = tray.try_event() {
            match event {
                // Work running is asked about first: quitting stops it.
                TrayEvent::Menu(QUIT) if ingest.work() > 0 => {
                    if stop_asked.is_none() {
                        stop_asked = Some((ask_stop(&tray, "tray.quit_ask"), true));
                    }
                }
                TrayEvent::Menu(QUIT) => {
                    log("quit".into());
                    return;
                }
                TrayEvent::Menu(STOP_ALL) if stop_asked.is_none() && ingest.work() > 0 => {
                    stop_asked = Some((ask_stop(&tray, "tray.stop_ask"), false));
                }
                TrayEvent::Menu(ADMIN) => open_admin(&tray, &root, &mut ingest),
                TrayEvent::Menu(OPEN_FORM) => open_form(&tray, &root, mode),
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
                // A mode chosen in the menu is this computer's from now on.
                TrayEvent::Menu(id) if (MODE_FIRST..MODE_FIRST + qnc_tray_mode::ALL.len() as u32).contains(&id) => {
                    let chosen = qnc_tray_mode::ALL[(id - MODE_FIRST) as usize];
                    match qnc_tray_mode::write(&root, chosen) {
                        Ok(()) => {
                            log(format!("mode chosen {chosen:?}"));
                            mode = Some(chosen);
                        }
                        Err(error) => fail(&tray, error),
                    }
                }
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
                            // A card put in settles for a while before its ingest starts (user
                            // 2026-11-17: "kod ubacivanja kartice napravi pauzu 5 sekundi prije
                            // pokretanja"; a reader still waking gave half a card to Select). It is
                            // looked at again until it has been there that long; gone, it starts over.
                            let (since, seen) = settling.entry(volume.serial_number.clone()).or_insert_with(|| (Instant::now(), Instant::now()));
                            // Not seen at the last looks: it was out meanwhile, it settles anew.
                            if seen.elapsed() > LOOK * 2 {
                                *since = Instant::now();
                            }
                            *seen = Instant::now();
                            if since.elapsed() < SETTLE {
                                watch.forget(&volume);
                                continue;
                            }
                            settling.remove(&volume.serial_number);
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
                                // The base it is in is being deleted (the user's Delete): the card
                                // is looked at again once the base is gone, then ingested as new
                                // (2026-11-17 log: the tray went on in a base being deleted).
                                Ok(Some(known)) if qnc_ingest_base::closing(&root, &known.base.id) => {
                                    log(format!("card serial={} known base={} is being deleted: looked at again later", volume.serial_number, known.base.label()));
                                    watch.forget(&volume);
                                    continue;
                                }
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
                        Found::Gone { volume } => {
                            log(format!("gone disk={} serial={}", volume.root.display(), volume.serial_number));
                            settling.remove(&volume.serial_number);
                        }
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
/// 2026-11-14: each base its own window and background worker, at the same time); `job` holds
/// them and every process they start, which end with the tray.
struct Windows {
    open: Vec<(String, Option<String>, std::process::Child)>,
    job: qnc_process_job::Job,
    /// The bases the work since the last stop went into: a stop that deletes deletes these.
    bases: Vec<qnc_ingest_base::IngestBase>,
    /// The administrator's form, held too but no work.
    admin: Option<std::process::Child>,
}

impl Windows {
    /// How many processes work: the ingest windows, their background workers and theirs (the
    /// administrator's form is no work).
    fn work(&mut self) -> usize {
        let admin = self.admin.as_mut().is_some_and(|child| matches!(child.try_wait(), Ok(None)));
        self.job.active().saturating_sub(usize::from(admin))
    }

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
            if let Err(error) = ingest.job.take(&child) {
                log(format!("ingest window not held by the tray: {error}"));
            }
            ingest.open.push((base.id.clone(), card.map(str::to_string), child));
            if !ingest.bases.iter().any(|known| known.id == base.id) {
                ingest.bases.push(base.clone());
            }
        }
        Err(error) => fail(tray, error.to_string()),
    }
}

/// The administrator's form, beside this program (it keeps one copy itself).
fn open_admin(tray: &Tray, root: &std::path::Path, ingest: &mut Windows) {
    let program = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|folder| folder.join(format!("qnc-ai-admin{}", std::env::consts::EXE_SUFFIX))))
        .unwrap_or_default();
    match std::process::Command::new(&program).current_dir(root).env("QNC_ROOT", root).spawn() {
        Ok(child) => {
            log("administration opened".into());
            if let Err(error) = ingest.job.take(&child) {
                log(format!("administration not held by the tray: {error}"));
            }
            ingest.admin = Some(child);
        }
        Err(error) => {
            log(format!("administration failed {}: {error}", program.display()));
            tray.alert("admin", &text("tray.title"), &fill("tray.admin_failed", &[("error", &error.to_string())]), &text("tray.ok"));
        }
    }
}

/// The answers of the question before the work stops.
const KEEP: &str = "keep";
const DELETE: &str = "delete";

/// The question before the work stops: keep the partial ingest, delete it, or not stop (Enter).
fn ask_stop(tray: &Tray, question: &str) -> std::sync::mpsc::Receiver<Option<String>> {
    let buttons = [(KEEP, text("card_ingest.partial_keep")), (DELETE, text("card_ingest.partial_delete")), ("cancel", text("ask.cancel"))];
    tray.choose(&text("tray.title"), &text(question), &buttons, "cancel")
}

/// The form the tray's mode opens after a copy, on the active project of the database (user
/// 2026-11-19): nothing when it already runs, the user is told so.
fn open_form(tray: &Tray, root: &std::path::Path, mode: Option<qnc_tray_mode::Mode>) {
    let Some((program, args)) = mode.and_then(qnc_tray_mode::Mode::opens) else { return };
    match qnc_wizard_open::open(root, program, args) {
        Ok(qnc_wizard_open::Opened::Started) => log(format!("form opened {program}")),
        Ok(qnc_wizard_open::Opened::Running) => tray.notify(&text("tray.title"), &text("tray.form_open")),
        Err(error) => fail(tray, fill("tray.form_failed", &[("error", &error)])),
    }
}
