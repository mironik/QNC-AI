//! The QNC AI wizard (docs/01-wizard-plan.md). It has no window of its own (user 2026-11-11:
//! "ne zelim formu vizarda, vizard moze biti sastavni dio traya"): it waits in the tray, one
//! copy, and looks at the attached disks every two seconds. A card of a camera QNC knows starts a
//! story; where the story is shows in the tray's menu (a click on the icon) and tooltip, and "Go on"
//! opens the form of that step. Any other new disk is only a notice. It loads no AI model and
//! writes nothing itself.

#![windows_subsystem = "windows"]

use std::time::{Duration, Instant};

use qnc_card_watch::{CardWatch, Found};
use qnc_dev_diagnostics::{log_line, DiagnosticsStream};
use qnc_tray::{MenuItem, OneCopy, Tray, TrayEvent};
use qnc_wizard_flow::{Card, Flow, Status};
use qnc_wizard_text::{fill, text};

const STATUS: u32 = 1;
const GO_ON: u32 = 2;
const QUIT: u32 = 3;
const SETTINGS: u32 = 4;
const DELETE_BASE: u32 = 5;
/// "Eject" of the n-th card of the story is `EJECT + n`.
const EJECT: u32 = 100;
/// How often the disks are looked at.
const LOOK: Duration = Duration::from_secs(2);
/// How often a missing camera catalog is read again.
const CATALOG_AGAIN: Duration = Duration::from_secs(60);
/// How many looks a card that cannot be read yet gets (a card waking up).
const READ_TRIES: u32 = 5;

fn log(line: String) {
    log_line(DiagnosticsStream::Player, format!("ai-tray {line}"));
}

/// The menu for where the story is: the status line (it only tells), "Eject" for each card, Go on, Quit.
fn menu(status: Option<&Status>) -> Vec<MenuItem> {
    let line = status.map_or_else(|| text("tray.waiting"), Status::line);
    let mut items = vec![MenuItem { id: STATUS, text: line, enabled: false }];
    if let Some(status) = status {
        // A card taken out is only a line (it cannot be ejected); a card in can be ejected.
        items.extend(status.cards.iter().zip(&status.cards_out).enumerate().map(|(index, (card, out))| match out {
            true => MenuItem { id: STATUS, text: format!("    {card}"), enabled: false },
            false => MenuItem { id: EJECT + index as u32, text: fill("tray.eject", &[("card", card)]), enabled: true },
        }));
        items.push(MenuItem { id: GO_ON, text: fill("tray.continue", &[("step", &status.name)]), enabled: true });
    }
    // Settings are rarely used and apart from the wizard (user 2026-11-11).
    items.push(MenuItem { id: SETTINGS, text: text("tray.settings"), enabled: true });
    // The Ingest base and its folder, after a question (user 2026-11-12).
    items.push(MenuItem { id: DELETE_BASE, text: text("tray.delete_base"), enabled: true });
    items.push(MenuItem { id: QUIT, text: text("tray.quit"), enabled: true });
    items
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
    let tray = match Tray::start(&format!("{}: {}", text("tray.title"), text("tray.waiting")), menu(None)) {
        Ok(tray) => tray,
        Err(error) => {
            log(format!("not started: {error}"));
            return;
        }
    };
    log("started".into());
    log(format!("root {}", root.display()));
    let mut flow = Flow::new(root.clone());
    let mut ingest: Option<std::process::Child> = None;
    let mut shown: Option<String> = None;
    let mut catalog = None;
    let mut catalog_read: Option<Instant> = None;
    let mut watch = CardWatch::new();
    let mut tries: std::collections::BTreeMap<String, u32> = Default::default();
    let mut looked: Option<Instant> = None;
    // A card is ejected on its own thread (the lock may wait); its answer comes back here.
    let ejected = std::sync::mpsc::channel::<(Card, Result<(), qnc_card_eject::EjectError>)>();
    // A base being asked about, and the bases deleted (their label and the result).
    let mut deleting: Option<(qnc_ingest_base::IngestBase, std::sync::mpsc::Receiver<bool>)> = None;
    let deleted = std::sync::mpsc::channel::<(String, Result<(), String>)>();
    loop {
        flow.poll();
        let status = flow.status();
        // Without a card and without a project there is no story yet: the tray waits.
        let story = (!flow.cards().is_empty() || status.project.is_some()).then_some(&status);
        let line = story.map_or_else(|| text("tray.waiting"), |status| format!("{} {}", status.line(), status.cards.join(", ")));
        if shown.as_ref() != Some(&line) {
            tray.set_menu(menu(story));
            tray.set_tip(&format!("{}: {line}", text("tray.title")));
            shown = Some(line);
        }
        while let Some(event) = tray.try_event() {
            match event {
                TrayEvent::Menu(QUIT) => {
                    log("quit".into());
                    return;
                }
                TrayEvent::Menu(SETTINGS) => open_settings(&tray, &root),
                TrayEvent::Menu(DELETE_BASE) => match qnc_ingest_base::active(&root) {
                    Ok(Some(base)) => {
                        let ask = fill("tray.delete_base_ask", &[("base", base.label())]);
                        deleting = Some((base, tray.confirm(&text("tray.title"), &ask)));
                    }
                    Ok(None) => tray.notify(&text("tray.title"), &text("tray.delete_base_none")),
                    Err(error) => tray.notify(&text("tray.title"), &error),
                },
                // The form of the step: the card step is the import form; the forms of the steps
                // after it come with W3 on (docs/01-wizard-plan.md).
                TrayEvent::Menu(GO_ON) if status.code == qnc_wizard_flow::CARD_STEP => open_ingest(&tray, &root, &mut ingest),
                TrayEvent::Menu(GO_ON) => tray.notify(&text("tray.title"), &fill("tray.no_form", &[("step", &status.name)])),
                TrayEvent::Menu(id) if id >= EJECT => {
                    if let Some(card) = flow.cards().get((id - EJECT) as usize).filter(|card| !card.out) {
                        let card = card.clone();
                        log(format!("eject disk={} serial={}", card.disk.display(), card.serial));
                        let answer = ejected.0.clone();
                        std::thread::spawn(move || {
                            let result = qnc_card_eject::eject(&card.disk);
                            let _ = answer.send((card, result));
                        });
                    }
                }
                TrayEvent::Menu(_) => {}
            }
        }
        // The answer to "delete the base?": the ingest of it stops first (its window and worker
        // let the base go), then the base and its folder are deleted on a thread of their own.
        if let Some((base, answer)) = &deleting {
            if let Ok(yes) = answer.try_recv() {
                if yes {
                    if let Some(child) = ingest.as_mut() {
                        let _ = child.kill();
                    }
                    ingest = None;
                    let (root, base, send) = (root.clone(), base.clone(), deleted.0.clone());
                    std::thread::spawn(move || {
                        // The worker stops within a second once its base is no longer active.
                        let mut result = Err(String::new());
                        for _ in 0..15 {
                            result = qnc_ingest_base::forget(&root, &base.id);
                            if result.is_ok() {
                                break;
                            }
                            std::thread::sleep(Duration::from_secs(1));
                        }
                        let _ = send.send((base.label().to_string(), result));
                    });
                }
                deleting = None;
            }
        }
        while let Ok((label, result)) = deleted.1.try_recv() {
            match result {
                Ok(()) => {
                    log(format!("base deleted {label}"));
                    tray.notify(&text("tray.title"), &fill("tray.base_deleted", &[("base", &label)]));
                }
                Err(error) => {
                    log(format!("base not deleted {label}: {error}"));
                    tray.alert("delete-base", &text("tray.title"), &fill("tray.base_delete_failed", &[("base", &label), ("error", &error)]), &text("tray.ok"));
                }
            }
        }
        while let Ok((card, result)) = ejected.1.try_recv() {
            let name = card.name();
            match result {
                Ok(()) => {
                    // Ejected on purpose: it leaves the story before its disk is gone, so no "put it back".
                    flow.remove_card(&card.serial);
                    log(format!("ejected disk={}", card.disk.display()));
                    tray.notify(&text("tray.title"), &fill("tray.ejected", &[("card", &name)]));
                }
                Err(qnc_card_eject::EjectError::InUse) => {
                    log(format!("eject refused disk={}: in use", card.disk.display()));
                    tray.alert(&format!("eject:{}", card.serial), &text("tray.title"), &fill("tray.eject_in_use", &[("card", &name)]), &text("tray.ok"));
                }
                Err(error) => {
                    log(format!("eject failed disk={}: {error}", card.disk.display()));
                    tray.alert(&format!("eject:{}", card.serial), &text("tray.title"), &fill("tray.eject_failed", &[("card", &name), ("error", &error.to_string())]), &text("tray.ok"));
                }
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
                // The first look takes the disks in as new: a card starts its story, the rest only logs.
                let first = looked.is_none();
                looked = Some(Instant::now());
                for found in watch.look(catalog) {
                    match found {
                        Found::Card { volume, cameras, patterns } => {
                            let disk = volume.root.display().to_string();
                            log(format!("card disk={disk} serial={} patterns={patterns:?}", volume.serial_number));
                            let card = Card { disk: volume.root.clone(), serial: volume.serial_number.clone(), label: volume.volume_name.clone(), cameras: cameras.clone(), out: false };
                            let name = card.name();
                            // The card is in: its "put the card back" is done, whatever happened before.
                            tray.close_alert(&format!("put-back:{}", volume.serial_number));
                            if flow.add_card(card) {
                                log(format!("card back disk={disk}"));
                                tray.notify(&text("tray.title"), &fill("tray.card_back", &[("card", &name)]));
                            } else {
                                let next = flow.status().name;
                                tray.notify(&text("tray.title"), &fill("tray.card", &[("card", &name), ("camera", &cameras.join(", ")), ("step", &next)]));
                            }
                            // The ingest of the card, started when it does not run: a card back
                            // is Selected again, so a clip whose import failed while it was out
                            // goes back to the queue (user 2026-11-12).
                            open_ingest(&tray, &root, &mut ingest);
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
                            let disk = volume.root.display().to_string();
                            log(format!("gone disk={disk} serial={}", volume.serial_number));
                            // A card of the story taken out without ejecting it: the story still uses it.
                            if let Some(card) = flow.card_out(&volume.serial_number) {
                                tray.alert(&format!("put-back:{}", card.serial), &text("tray.title"), &fill("tray.put_back", &[("card", &card.name())]), &text("tray.ok"));
                            }
                        }
                    }
                }
            }
        }
        std::thread::sleep(Duration::from_millis(150));
    }
}

/// The background ingest of the cards into the global Ingest base (user 2026-11-12), beside this
/// program, started when the one this tray started does not run any more; its window says it runs
/// and, when it is done, starts QNC (the Ingest form). Its helper programs (player, worker,
/// decoders) are console programs: they share one hidden console, so no window flashes.
fn open_ingest(tray: &Tray, root: &std::path::Path, ingest: &mut Option<std::process::Child>) {
    if ingest.as_mut().is_some_and(|child| matches!(child.try_wait(), Ok(None))) {
        return;
    }
    // Ingest works in the current Ingest base of the application (data/ingest).
    match qnc_ingest_base::current(root) {
        Ok(base) => log(format!("ingest base {} {}", base.name, base.uri)),
        Err(error) => {
            log(format!("ingest base failed: {error}"));
            tray.alert("ingest", &text("tray.title"), &fill("tray.ingest_failed", &[("error", &error)]), &text("tray.ok"));
            return;
        }
    }
    let program = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|folder| folder.join(format!("qnc-ai-card-ingest{}", std::env::consts::EXE_SUFFIX))))
        .unwrap_or_default();
    let mut command = std::process::Command::new(&program);
    command.current_dir(root).env("QNC_ROOT", root);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    match command.spawn() {
        Ok(child) => {
            log(format!("ingest opened {}", program.display()));
            *ingest = Some(child);
        }
        Err(error) => {
            log(format!("ingest failed {}: {error}", program.display()));
            tray.alert("ingest", &text("tray.title"), &fill("tray.ingest_failed", &[("error", &error.to_string())]), &text("tray.ok"));
        }
    }
}

/// The Settings form, beside this program (it keeps one copy itself).
fn open_settings(tray: &Tray, root: &std::path::Path) {
    let program = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(|folder| folder.join(format!("qnc-ai-settings{}", std::env::consts::EXE_SUFFIX))))
        .unwrap_or_default();
    match std::process::Command::new(&program).current_dir(root).env("QNC_ROOT", root).spawn() {
        Ok(_) => log("settings opened".into()),
        Err(error) => {
            log(format!("settings failed {}: {error}", program.display()));
            tray.alert("settings", &text("tray.title"), &fill("tray.settings_failed", &[("error", &error.to_string())]), &text("tray.ok"));
        }
    }
}
