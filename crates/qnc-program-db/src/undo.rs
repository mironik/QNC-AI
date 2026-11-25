//! Story undo / redo (user request 2026-09-25: an UNDO button; 2026-11-25: the journal of the
//! project is "a quasi git that remembers every action"). Every story edit is a commit of the
//! project's journal (who, when, the rows before and after); Undo takes the newest of them back,
//! Redo makes the last taken one again (piece `qnc-journal-revert`). Undo and Redo are commits
//! too, so nothing is lost and any form or process sees the same steps. A new edit clears Redo.
//! Selection changes are not steps: they are no commits and are neither checked nor put back.

use qnc_project_journal::Step;
use rusqlite::Connection;

use crate::Result;

/// What a step neither checks nor puts back: the selection and the stamps of the story state.
pub(crate) const LOOSE: qnc_journal_revert::Loose<'static> = &[(
    "story_state",
    &["selected_part_id", "selected_shot_id", "selected_slot_id", "selected_cover_id", "draft_updated_at", "committed_at", "updated_at"],
)];

/// Undo (or redo): the newest commit taken back (or the last taken one made again). Nothing to
/// take is a controlled error; a row changed since by another edit refuses the step. It runs in
/// the write's transaction; the step it was goes to the journal.
pub(crate) fn step(conn: &Connection, undo: bool) -> Result<Step> {
    let stacks = qnc_project_journal::stacks(conn, crate::MODULE_ID)?;
    let target = if undo { stacks.undo.last() } else { stacks.redo.last() };
    let Some(&target) = target else {
        return Err(if undo { qnc_ui_text::text("story.nothing_undo").into() } else { qnc_ui_text::text("story.nothing_redo").into() });
    };
    let commit = qnc_project_journal::commit(conn, target)?.ok_or_else(|| qnc_ui_text::text("story.nothing_undo"))?;
    if undo {
        qnc_journal_revert::revert(conn, &commit.changes, LOOSE)?;
        Ok(Step::Undo(target))
    } else {
        qnc_journal_revert::reapply(conn, &commit.changes, LOOSE)?;
        Ok(Step::Redo(target))
    }
}

/// How many undo and redo steps there are.
pub(crate) fn depth(conn: &Connection) -> Result<(u64, u64)> {
    let stacks = qnc_project_journal::stacks(conn, crate::MODULE_ID)?;
    Ok((stacks.undo.len() as u64, stacks.redo.len() as u64))
}
