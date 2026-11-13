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
const PROJECTS: u32 = 6;
const LAST_PROJECT: u32 = 7;
const INGEST_FOLDER: u32 = 8;
/// How often the last project is read again for the menu.
const LAST_AGAIN: Duration = Duration::from_secs(10);
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
fn menu(status: Option<&Status>, last: Option<&str>, has_bases: bool) -> Vec<MenuItem> {
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
    // The project worked on last, and the user's projects, to go on with one that was closed
    // (user 2026-11-12).
    if let Some(name) = last {
        items.push(MenuItem { id: LAST_PROJECT, text: fill("tray.last_project", &[("name", name)]), enabled: true });
    }
    items.push(MenuItem { id: PROJECTS, text: text("tray.projects"), enabled: true });
    // A source that is no card (a folder on a disk, the LAN): the user picks it in Ingest's browser
    // (user 2026-11-13: "otvaranje izbornika direktorija, ako izvor nije kartica").
    items.push(MenuItem { id: INGEST_FOLDER, text: text("tray.ingest_folder"), enabled: true });
    // Settings are rarely used and apart from the wizard (user 2026-11-11).
    items.push(MenuItem { id: SETTINGS, text: text("tray.settings"), enabled: true });
    // The Ingest base and its folder, after a question (user 2026-11-12); greyed when there is no
    // base (user 2026-11-13).
    items.push(MenuItem { id: DELETE_BASE, text: text("tray.delete_base"), enabled: has_bases });
    items.push(MenuItem { id: QUIT, text: text("tray.quit"), enabled: true });
    items
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
    let tray = match Tray::start(&format!("{}: {}", text("tray.title"), text("tray.waiting")), menu(None, None, false), Some(qnc_app_mark::qnc_mark)) {
        Ok(tray) => tray,
        Err(error) => {
            log(format!("not started: {error}"));
            return;
        }
    };
    log("started".into());
    // The camera profiles, to name a card's camera by its model.
    let (profiles, _) = qnc_camera_profiles::read_profiles(&root.join(qnc_camera_profiles::PROFILE_DIR));
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
    // A base is being deleted: no card opens the ingest meanwhile.
    let mut removing = false;
    // Quit was chosen while a base is deleted: the tray ends once the delete is done (live
    // 2026-11-13: Quit 8 s into a delete left the base; the card was then "already imported").
    let mut quitting = false;
    // A card a base already has, with new clips: the question where they go (user 2026-11-12).
    let mut known: Option<(qnc_ingest_base::KnownCard, std::sync::mpsc::Receiver<qnc_tray::Answer>)> = None;
    // A card already in a base, nothing new: "open your projects?" (user 2026-11-12).
    let mut offer: Option<std::sync::mpsc::Receiver<bool>> = None;
    // The project worked on last (id, name), read again now and then for the menu.
    let mut last: Option<(String, String)> = None;
    let mut last_read: Option<Instant> = None;
    // Whether there is an Ingest base to delete, read with the last project.
    let mut has_bases = false;
    loop {
        flow.poll();
        let status = flow.status();
        if last_read.is_none_or(|at| at.elapsed() >= LAST_AGAIN) {
            last_read = Some(Instant::now());
            last = qnc_ingest_base::last_story(&root, "").unwrap_or_else(|error| {
                log(format!("last project not read: {error}"));
                None
            });
            // Not read is not greyed: the delete then says why.
            has_bases = qnc_ingest_base::all(&root).map_or(true, |bases| !bases.is_empty());
        }
        // Without a card and without a project there is no story yet: the tray waits.
        let story = (!flow.cards().is_empty() || status.project.is_some()).then_some(&status);
        let line = story.map_or_else(|| text("tray.waiting"), |status| format!("{} {}", status.line(), status.cards.join(", ")));
        let last_name = last.as_ref().map(|(_, name)| name.as_str());
        let key = format!("{line}|{}|{has_bases}", last_name.unwrap_or(""));
        if shown.as_ref() != Some(&key) {
            tray.set_menu(menu(story, last_name, has_bases));
            tray.set_tip(&format!("{}: {line}", text("tray.title")));
            shown = Some(key);
        }
        while let Some(event) = tray.try_event() {
            match event {
                TrayEvent::Menu(QUIT) if removing => {
                    log("quit: waits for the base being deleted".into());
                    tray.notify(&text("tray.title"), &text("tray.quit_after_delete"));
                    quitting = true;
                }
                TrayEvent::Menu(QUIT) => {
                    log("quit".into());
                    return;
                }
                TrayEvent::Menu(SETTINGS) => open_settings(&tray, &root),
                TrayEvent::Menu(PROJECTS) => open_projects(&tray, &root),
                TrayEvent::Menu(LAST_PROJECT) => {
                    if let Some((id, name)) = last.clone() {
                        open_last(&tray, &root, &id, &name);
                    }
                }
                // The active base, or, when none is active (a delete that failed closed it), the
                // newest base (live 2026-11-12).
                TrayEvent::Menu(DELETE_BASE) => match qnc_ingest_base::active(&root).and_then(|base| match base {
                    Some(base) => Ok(Some(base)),
                    None => qnc_ingest_base::all(&root).map(|bases| bases.into_iter().next()),
                }) {
                    Ok(Some(base)) => match qnc_ingest_base::may_delete(&root, &base.id) {
                        Ok(()) => {
                            let ask = fill("tray.delete_base_ask", &[("base", base.label())]);
                            log(format!("delete base asked {}", base.label()));
                            deleting = Some((base, tray.confirm(&text("tray.title"), &ask)));
                        }
                        // The base is public: only who made it, or the administrator, deletes it
                        // (user 2026-11-12).
                        Err(refusal) => {
                            log(format!("base not deleted {}: {refusal:?}", base.label()));
                            let why = match refusal {
                                qnc_ingest_base::Refusal::NotOwner { user, computer } => fill("tray.delete_base_not_owner", &[("base", base.label()), ("user", &user), ("computer", &computer)]),
                                qnc_ingest_base::Refusal::NoOwner => fill("tray.delete_base_no_owner", &[("base", base.label())]),
                                qnc_ingest_base::Refusal::Unread(error) => fill("tray.base_delete_failed", &[("base", base.label()), ("error", &error)]),
                            };
                            tray.alert("delete-base", &text("tray.title"), &why, &text("tray.ok"));
                        }
                    },
                    Ok(None) => {
                        log("delete base: there is none".into());
                        tray.info("delete-base", &text("tray.title"), &text("tray.delete_base_none"), &text("tray.ok"), Duration::from_secs(5));
                    }
                    Err(error) => {
                        log(format!("delete base: bases not read: {error}"));
                        tray.alert("delete-base", &text("tray.title"), &error, &text("tray.ok"));
                    }
                },
                // The form of the step: the card step is the import form; the forms of the steps
                // after it come with W3 on (docs/01-wizard-plan.md).
                TrayEvent::Menu(GO_ON) if status.code == qnc_wizard_flow::CARD_STEP => open_ingest(&tray, &root, &mut ingest, qnc_ingest_base::current),
                TrayEvent::Menu(INGEST_FOLDER) => open_ingest_of(&tray, &root, &mut ingest, qnc_ingest_base::current, true),
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
                log(format!("delete base answered {}: {yes}", base.label()));
                if yes {
                    if let Some(child) = ingest.as_mut() {
                        let _ = child.kill();
                    }
                    ingest = None;
                    removing = true;
                    // It can take some seconds (the worker ends its step first): the user is told.
                    tray.notify(&text("tray.title"), &fill("tray.base_deleting", &[("base", base.label())]));
                    let (root, base, send) = (root.clone(), base.clone(), deleted.0.clone());
                    std::thread::spawn(move || {
                        // The base is closed as the active project first: its worker stops within
                        // a second once its base is no longer active and lets the database go
                        // (live 2026-11-12: "the process cannot access the file").
                        if base.active {
                            if let Err(error) = qnc_project_close::CloseProjectComponent::from_root(&root).close_active_project() {
                                log(format!("base not closed {}: {error}", base.label()));
                            }
                        }
                        let mut result = Err(String::new());
                        // A worker in the middle of an AI step ends it first (seconds).
                        for _ in 0..60 {
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
            removing = false;
            // The menu says at once whether a base is left to delete.
            last_read = None;
            match result {
                Ok(()) => {
                    log(format!("base deleted {label}"));
                    // An overlay in the middle of the screen that closes itself (user 2026-11-12).
                    tray.info("base-deleted", &text("tray.title"), &fill("tray.base_deleted", &[("base", &label)]), &text("tray.ok"), Duration::from_secs(5));
                }
                Err(error) => {
                    log(format!("base not deleted {label}: {error}"));
                    tray.alert("delete-base", &text("tray.title"), &fill("tray.base_delete_failed", &[("base", &label), ("error", &error)]), &text("tray.ok"));
                }
            }
        }
        if quitting && !removing {
            log("quit".into());
            return;
        }
        // Nothing new on the card: the user's projects, to go on with one of them.
        if let Some(yes) = offer.as_ref().and_then(|answer| answer.try_recv().ok()) {
            offer = None;
            log(format!("card known, projects offered: {yes}"));
            if yes {
                open_projects(&tray, &root);
            }
        }
        // Where the new clips of a card a base has go: into that base, or into a new base that
        // takes only them; what the base has is never imported again.
        if let Some(answer) = known.as_ref().and_then(|(_, answer)| answer.try_recv().ok()) {
            let (card_known, _) = known.take().expect("asked");
            log(format!("card known answered base={}: {answer:?}", card_known.base.label()));
            match answer {
                qnc_tray::Answer::Yes => {
                    let id = card_known.base.id.clone();
                    open_ingest(&tray, &root, &mut ingest, |root| qnc_ingest_base::open(root, &id));
                }
                qnc_tray::Answer::No => {
                    let from = card_known.base.clone();
                    open_ingest(&tray, &root, &mut ingest, |root| qnc_ingest_base::new_with_only_new(root, &from));
                }
                qnc_tray::Answer::Cancel => {}
            }
        }
        while let Ok((card, result)) = ejected.1.try_recv() {
            let name = card.name_with_camera();
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
                            // The model the camera profiles recognise ("Sony ILME-FX3A"), else the
                            // camera families of the catalog (user 2026-11-13: numbers alone say little).
                            let cameras = match qnc_card_watch::camera_of(catalog, &profiles, &volume) {
                                Some(model) => vec![model],
                                None => cameras,
                            };
                            log(format!("card disk={disk} serial={} camera={} patterns={patterns:?}", volume.serial_number, cameras.join(", ")));
                            let card = Card { disk: volume.root.clone(), serial: volume.serial_number.clone(), label: volume.volume_name.clone(), cameras: cameras.clone(), out: false };
                            let name = card.name();
                            let named = card.name_with_camera();
                            // The card is in: its "put the card back" is done, whatever happened before.
                            tray.close_alert(&format!("put-back:{}", volume.serial_number));
                            if flow.add_card(card) {
                                log(format!("card back disk={disk}"));
                                tray.notify(&text("tray.title"), &fill("tray.card_back", &[("card", &named)]));
                            } else {
                                let next = flow.status().name;
                                tray.notify(&text("tray.title"), &fill("tray.card", &[("card", &name), ("camera", &cameras.join(", ")), ("step", &next)]));
                            }
                            // The ingest of the card, started when it does not run: a card back
                            // is Selected again, so a clip whose import failed while it was out
                            // goes back to the queue (user 2026-11-12).
                            // Not while a base is deleted: it would be made again at once.
                            // A card a base already has is not imported again (user 2026-11-12):
                            // nothing new is said; new clips are asked about.
                            if !removing && known.is_none() && !runs(&mut ingest) {
                                match card_check(&root, catalog, &volume) {
                                    Ok(None) => open_ingest(&tray, &root, &mut ingest, qnc_ingest_base::current),
                                    Ok(Some(card_known)) if card_known.new.is_empty() => {
                                        log(format!("card known disk={disk} base={} had={}", card_known.base.label(), card_known.had));
                                        let said = fill("tray.card_known", &[("card", &named), ("base", card_known.base.label()), ("had", &card_known.had.to_string())]);
                                        if offer.is_none() {
                                            offer = Some(tray.confirm(&text("tray.title"), &said));
                                        }
                                    }
                                    Ok(Some(card_known)) => {
                                        log(format!("card known disk={disk} base={} had={} new={}", card_known.base.label(), card_known.had, card_known.new.len()));
                                        let base = card_known.base.label().to_string();
                                        let ask = fill("tray.card_known_ask", &[("card", &named), ("base", &base), ("had", &card_known.had.to_string()), ("new", &card_known.new.len().to_string())]);
                                        known = Some((card_known, tray.ask(&text("tray.title"), &ask)));
                                    }
                                    Err(error) => {
                                        // Not known for sure: Ingest goes on as before (Select reconciles).
                                        log(format!("card check failed disk={disk}: {error}"));
                                        open_ingest(&tray, &root, &mut ingest, qnc_ingest_base::current);
                                    }
                                }
                            } else if !removing && known.is_none() {
                                open_ingest(&tray, &root, &mut ingest, qnc_ingest_base::current);
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
                            let disk = volume.root.display().to_string();
                            log(format!("gone disk={disk} serial={}", volume.serial_number));
                            // A card of the story taken out without ejecting it: the story still uses it.
                            if let Some(card) = flow.card_out(&volume.serial_number) {
                                tray.alert(&format!("put-back:{}", card.serial), &text("tray.title"), &fill("tray.put_back", &[("card", &card.name_with_camera())]), &text("tray.ok"));
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
fn open_ingest(tray: &Tray, root: &std::path::Path, ingest: &mut Option<std::process::Child>, base: impl FnOnce(&std::path::Path) -> Result<qnc_ingest_base::IngestBase, String>) {
    open_ingest_of(tray, root, ingest, base, false);
}

/// As `open_ingest`; with `folder` its window first shows Ingest's browser, the user picks the
/// folder to import (a source that is no card), and the cards attached are left alone.
fn open_ingest_of(tray: &Tray, root: &std::path::Path, ingest: &mut Option<std::process::Child>, base: impl FnOnce(&std::path::Path) -> Result<qnc_ingest_base::IngestBase, String>, folder: bool) {
    if runs(ingest) {
        // One window at a time: asked for a folder, the window that runs shows the browser beside
        // the card it ingests (a copy started with `--folder` tells it so and ends), in its base.
        if folder {
            let program = std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|folder| folder.join(format!("qnc-ai-card-ingest{}", std::env::consts::EXE_SUFFIX))))
                .unwrap_or_default();
            let mut command = std::process::Command::new(&program);
            command.current_dir(root).env("QNC_ROOT", root).arg("--folder");
            match command.spawn() {
                Ok(_) => log("ingest runs: asked it for a folder".into()),
                Err(error) => {
                    log(format!("ingest runs: folder not asked: {error}"));
                    tray.notify(&text("tray.title"), &text("tray.ingest_runs"));
                }
            }
        }
        return;
    }
    // Ingest works in the base `base` makes active (the current one, the one a card is in, or a
    // new one that takes only the new clips).
    match base(root) {
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
    if folder {
        command.arg("--folder");
    }
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

/// The Projects form, beside this program (it keeps one copy itself): the user's projects.
fn open_projects(tray: &Tray, root: &std::path::Path) {
    match qnc_ai_open::open(root, "qnc-ai-projects") {
        Ok(program) => log(format!("projects opened {}", program.display())),
        Err(error) => {
            log(format!("projects failed: {error}"));
            tray.alert("projects", &text("tray.title"), &fill("tray.projects_failed", &[("error", &error)]), &text("tray.ok"));
        }
    }
}

/// The project worked on last goes on: its newest base is made the one Ingest works in and AI
/// Media Assist opens, as after Uvezi.
fn open_last(tray: &Tray, root: &std::path::Path, id: &str, name: &str) {
    let opened = qnc_ingest_base::open_story(root, id).and_then(|_| qnc_ai_open::open(root, "qnc-media-assist-audio-ai"));
    match opened {
        Ok(program) => log(format!("last project {name} opened in {}", program.display())),
        Err(error) => {
            log(format!("last project {name} not opened: {error}"));
            tray.alert("last-project", &text("tray.title"), &fill("tray.last_project_failed", &[("name", name), ("error", &error)]), &text("tray.ok"));
        }
    }
}

/// Whether the ingest this tray started still runs.
fn runs(ingest: &mut Option<std::process::Child>) -> bool {
    ingest.as_mut().is_some_and(|child| matches!(child.try_wait(), Ok(None)))
}

/// The base that already has the card at `volume` and what is new on it (user 2026-11-12: the
/// camera operator did not erase the card before the next shoot). `None` when no base has its
/// clips, or the base Ingest works in now has them and no story took it (a card back goes on).
fn card_check(root: &std::path::Path, catalog: &qnc_camera_patterns::Catalog, volume: &qnc_source_volumes::Volume) -> Result<Option<qnc_ingest_base::KnownCard>, String> {
    let card = qnc_card_watch::card_originals(catalog, volume)?;
    let Some(known) = qnc_ingest_base::known_card(root, &volume.source_uri(), &card)? else { return Ok(None) };
    if known.base.active && known.base.project_name.is_none() {
        return Ok(None);
    }
    Ok(Some(known))
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
