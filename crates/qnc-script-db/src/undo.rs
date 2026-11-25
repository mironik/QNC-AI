//! Undo and redo of the script (user 2026-11-06: "Backspace undo does not work"; words cut out of
//! a statement must come back; 2026-11-25: from the journal of the project). Every change of the
//! script's blocks is a commit of the project's journal that Undo may take back; Undo takes the
//! newest back, Redo makes the last taken one again (piece `qnc-journal-revert`), each itself a
//! commit. The voice of a block (its recording) is never undone: it stays as recorded.

use qnc_project_journal::Step;
use rusqlite::Connection;

use crate::Result;

/// What a step neither checks nor puts back: the recording of a block and its stamp.
pub(crate) const LOOSE: qnc_journal_revert::Loose<'static> = &[("script_blocks", &["voice_uri", "voice_seconds", "voice_tc", "updated_at_utc"])];

/// One step back (`undo`) or forward; `None`: there is none. A block changed since by another
/// edit refuses the step.
pub(crate) fn step(conn: &Connection, undo: bool) -> Result<Option<Step>> {
    let stacks = qnc_project_journal::stacks(conn, crate::MODULE_ID)?;
    let target = if undo { stacks.undo.last() } else { stacks.redo.last() };
    let Some(&target) = target else { return Ok(None) };
    let Some(commit) = qnc_project_journal::commit(conn, target)? else { return Ok(None) };
    if undo {
        qnc_journal_revert::revert(conn, &commit.changes, LOOSE)?;
        Ok(Some(Step::Undo(target)))
    } else {
        qnc_journal_revert::reapply(conn, &commit.changes, LOOSE)?;
        Ok(Some(Step::Redo(target)))
    }
}
