//! Undo of the selected (user 2026-10-29: "two kinds of undo: undo in time, back along the order
//! of the edits, and undo selected: what is selected is undone or redone, whenever it was").
//! v5 undid a selected cover only, one level (`story/object_history.rs`); here every segment, M
//! marker and cover. The commits of the project's journal are the history (user 2026-11-25):
//! undo selected finds the newest commit that changed the object and puts back only that
//! object's row as it was before it; redo selected puts back what that undo took. An undo
//! selected is a story edit itself, so the undo in time takes it back too. Per object, in
//! `story_object_history`: the commit it went back to and the rows it took (its redo), cleared
//! when another edit changes it.

use qnc_change_capture::RowChange;
use rusqlite::{params, Connection, OptionalExtension};
use serde_json::{json, Value};

use crate::store::{err, story_now};
use crate::Result;

/// An object that can be undone alone: its table and its key column.
fn table_of(object_type: &str) -> Result<(&'static str, &'static str)> {
    match object_type.trim() {
        "segment" | "cover" => Ok(("story_items", "item_id")),
        "marker" => Ok(("story_markers", "marker_id")),
        other => Err(format!("undo selected: unknown object type {other}")),
    }
}

/// The columns that say nothing about the object itself.
const STAMPS: [&str; 2] = ["created_at", "updated_at"];

/// The row without its stamps, to tell whether the object changed (`Null`: not there).
fn content(row: &Value) -> Value {
    match row {
        Value::Object(map) => Value::Object(map.iter().filter(|(column, _)| !STAMPS.contains(&column.as_str())).map(|(column, value)| (column.clone(), value.clone())).collect()),
        _ => Value::Null,
    }
}

/// The object's history: the commit it went back to (`None` none yet) and what each undo took,
/// newest last.
#[derive(Default)]
struct History {
    from: Option<i64>,
    taken: Vec<(Option<i64>, Value)>,
}

fn history(conn: &Connection, object_type: &str, id: &str) -> Result<History> {
    let raw: Option<String> = conn
        .query_row("SELECT snapshot_json FROM story_object_history WHERE object_type = ?1 AND object_id = ?2", params![object_type, id], |row| row.get(0))
        .optional()
        .map_err(err)?;
    let Some(raw) = raw else {
        return Ok(History::default());
    };
    let json: Value = serde_json::from_str(&raw).map_err(err)?;
    if json.get("from_seq").is_some() {
        return Ok(History::default()); // kept by the undo before the journal: its steps are gone
    }
    let taken = json["taken"].as_array().cloned().unwrap_or_default();
    Ok(History { from: json["from_commit"].as_i64(), taken: taken.into_iter().map(|entry| (entry["from_commit"].as_i64(), entry["row"].clone())).collect() })
}

fn keep_history(conn: &Connection, object_type: &str, id: &str, history: &History) -> Result<()> {
    let taken: Vec<Value> = history.taken.iter().map(|(from, row)| json!({ "from_commit": from, "row": row })).collect();
    let snapshot = json!({ "from_commit": history.from, "taken": taken }).to_string();
    conn.execute(
        "INSERT INTO story_object_history (object_type, object_id, state, snapshot_json, updated_at)
         VALUES (?1, ?2, 'undone', ?3, ?4)
         ON CONFLICT(object_type, object_id) DO UPDATE SET
            state = excluded.state, snapshot_json = excluded.snapshot_json, updated_at = excluded.updated_at",
        params![object_type, id, snapshot, story_now()],
    )
    .map_err(err)?;
    Ok(())
}

/// Undo selected: the newest change of the object before the commit it last went back to is
/// undone, the object alone. Nothing to undo is a controlled error.
pub(crate) fn undo(conn: &Connection, object_type: &str, id: &str) -> Result<()> {
    let (table, key) = table_of(object_type)?;
    let id = id.trim();
    let keyed = json!({ key: id });
    let mut history = history(conn, object_type, id)?;
    let now = qnc_journal_revert::current(conn, table, &keyed)?.unwrap_or(Value::Null);
    let commits = qnc_project_journal::row_history(conn, table, key, id, usize::MAX >> 1)?;
    let found = commits.iter().rev().filter(|commit| commit.module == crate::MODULE_ID && history.from.is_none_or(|from| commit.id < from)).find_map(|commit| {
        let (first, last) = (commit.changes.first()?, commit.changes.last()?);
        (content(&first.before) != content(&last.after)).then(|| (commit.id, first.before.clone()))
    });
    let Some((commit, before)) = found else {
        return Err(qnc_ui_text::text("story.nothing_undo_selected").into());
    };
    qnc_journal_revert::put(conn, table, &keyed, Some(&before).filter(|row| !row.is_null()))?;
    history.taken.push((history.from, now));
    history.from = Some(commit);
    keep_history(conn, object_type, id, &history)
}

/// Redo selected: what the last undo selected of the object took comes back.
pub(crate) fn redo(conn: &Connection, object_type: &str, id: &str) -> Result<()> {
    let (table, key) = table_of(object_type)?;
    let id = id.trim();
    let mut history = history(conn, object_type, id)?;
    let Some((from, row)) = history.taken.pop() else {
        return Err(qnc_ui_text::text("story.nothing_redo_selected").into());
    };
    qnc_journal_revert::put(conn, table, &json!({ key: id }), Some(&row).filter(|row| !row.is_null()))?;
    history.from = from;
    keep_history(conn, object_type, id, &history)
}

/// After any other edit: the objects it changed (the rows caught so far) lose their own history
/// (a new change starts it again), the others keep theirs.
pub(crate) fn forget_changed(conn: &Connection, changes: &[RowChange]) -> Result<()> {
    let mut kept = conn.prepare("SELECT object_type, object_id FROM story_object_history").map_err(err)?;
    let kept = kept
        .query_map([], |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)))
        .map_err(err)?
        .collect::<rusqlite::Result<Vec<_>>>()
        .map_err(err)?;
    for (object_type, id) in kept {
        let Ok((table, key)) = table_of(&object_type) else { continue };
        let is_it = |row: &Value| row.get(key).and_then(Value::as_str) == Some(id.as_str());
        let changed = changes.iter().any(|change| change.table == table && (is_it(&change.before) || is_it(&change.after)) && content(&change.before) != content(&change.after));
        if changed {
            conn.execute("DELETE FROM story_object_history WHERE object_type = ?1 AND object_id = ?2", params![object_type, id]).map_err(err)?;
        }
    }
    Ok(())
}

/// After an undo or redo in time: every object's own history goes (its steps moved).
pub(crate) fn forget_all(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM story_object_history", []).map_err(err)?;
    Ok(())
}
