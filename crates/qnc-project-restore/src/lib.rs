//! Takes the whole project back to a moment of its journal (user 2026-11-25: "a quasi git that
//! remembers every action from the start of the edit to its end"; the project belongs to its
//! user). One thing only: each piece with commits after the moment is taken back to it through
//! the project database intermediary (`qnc_db_broker::restore_payload`), each as one commit of
//! its own (`restore`), so the going back can itself be gone back from and nothing is lost. All
//! of them are tried first: one that would refuse (a row changed since) leaves all as they are.
//! Who may go back is not decided here (piece `qnc-project-history`).

use std::sync::Arc;

use qnc_db_broker::{Access, ProjectDbTarget, TableModuleFactory};

pub type Result<T> = std::result::Result<T, String>;

/// The pieces whose writes the project's journal keeps.
fn journaled() -> Vec<Arc<dyn TableModuleFactory>> {
    vec![
        qnc_program_db::StoryModule::factory(),
        qnc_script_db::ScriptModule::factory(),
        qnc_virtual_shots::VirtualShotsModule::factory(),
        qnc_clip_sound::ClipSoundModule::factory(),
        qnc_skeleton_db::SkeletonModule::factory(),
        qnc_newsroom_sheet_db::StorySheetModule::factory(),
        qnc_clip_marks::ClipMarksModule::factory(),
    ]
}

/// Takes the pieces `modules` (those with commits after `to`) back to the moment right after the
/// commit `to`; how many commits were taken back. A piece this computer does not know refuses it.
pub fn restore(target: &ProjectDbTarget, to: i64, modules: &[String]) -> Result<u64> {
    let known = journaled();
    let factories: Vec<Arc<dyn TableModuleFactory>> = modules
        .iter()
        .map(|module| known.iter().find(|factory| factory.id() == module).cloned().ok_or_else(|| qnc_ui_text::fill("journal.piece_unknown", &[("piece", module)])))
        .collect::<Result<_>>()?;
    let mut client = target.open(Access::ReadWrite, factories.clone())?;
    for check in [true, false] {
        let mut taken = 0;
        for factory in &factories {
            let answer = client.execute(factory.id(), qnc_db_broker::restore_payload(to, check))?;
            taken += answer.get("taken").and_then(|taken| taken.as_u64()).unwrap_or(0);
        }
        if !check {
            return Ok(taken);
        }
    }
    Ok(0)
}
