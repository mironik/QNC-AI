use super::*;
use qnc_db_broker::Access;
use rusqlite::Connection;

const PROJECT_DB: &str = "qnc://local/db/project_db/p1";

fn project() -> (tempfile::TempDir, ProjectDbTarget) {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("project.db");
    Connection::open(&file)
        .unwrap()
        .execute_batch(
            "CREATE TABLE project_settings (project_id TEXT);
             INSERT INTO project_settings (project_id) VALUES ('p1');
             CREATE VIEW public_project_settings AS SELECT project_id FROM project_settings;",
        )
        .unwrap();
    (dir, ProjectDbTarget::from_owner_binding(&file, PROJECT_DB).unwrap())
}

fn keep(target: &ProjectDbTarget, to: (u64, u64)) {
    let mut client = target.open(Access::ReadWrite, vec![qnc_clip_marks::ClipMarksModule::factory()]).unwrap();
    let operation = qnc_clip_marks::Operation::Keep { clip_id: "a".into(), from: (0, 100), to };
    client.execute(qnc_clip_marks::MODULE_ID, serde_json::to_value(operation).unwrap()).unwrap();
}

fn marks(dir: &tempfile::TempDir) -> (i64, i64) {
    let file = dir.path().join("project.db");
    Connection::open(file).unwrap().query_row("SELECT in_frame, out_frame FROM clip_marks WHERE clip_id = 'a'", [], |row| Ok((row.get(0)?, row.get(1)?))).unwrap()
}

fn settle(history: &mut ProjectHistory) {
    for _ in 0..2000 {
        history.poll();
        if !history.busy() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("not done")
}

/// User 2026-11-25: the whole project goes back to a moment of its journal; the going back is a
/// moment itself, and the moments after the one gone back to stay in the list.
#[test]
fn the_project_goes_back_to_a_moment_and_the_moments_after_it_stay() {
    let (dir, target) = project();
    keep(&target, (10, 100));
    keep(&target, (20, 100));
    keep(&target, (30, 100));
    let mut history = ProjectHistory::of(target.clone());
    assert!(history.key(OPEN, None));
    settle(&mut history);
    let view = history.view().unwrap();
    assert_eq!(view.rows.len(), 4, "three moments and the start");
    assert!(view.may, "a project without an owner kept: anyone");
    // The oldest moment (the first Keep) is the row before the start.
    assert!(history.key(RESTORE, Some(2)));
    settle(&mut history);
    assert_eq!(marks(&dir), (10, 100), "back to the moment right after the first Save");
    let view = history.view().unwrap();
    assert_eq!(view.rows.len(), 5, "the going back is a moment, the others stay");
    assert!(view.rows[0].starts_with("#4"), "{}", view.rows[0]);
    assert!(history.key("clear_focus", None));
    assert!(history.view().is_none());
}

/// Only the QNC user who made the project goes back.
#[test]
fn only_the_owner_goes_back() {
    let (dir, target) = project();
    keep(&target, (10, 100));
    keep(&target, (20, 100));
    Connection::open(dir.path().join("project.db"))
        .unwrap()
        .execute_batch("CREATE TABLE project_owner (project_id TEXT PRIMARY KEY, qnc_user TEXT NOT NULL); INSERT INTO project_owner VALUES ('p1', 'netko-drugi'); CREATE VIEW public_project_owner AS SELECT project_id, qnc_user FROM project_owner;")
        .unwrap();
    let mut history = ProjectHistory::of(target.clone());
    history.key(OPEN, None);
    settle(&mut history);
    assert!(!history.view().unwrap().may);
    history.key(RESTORE, Some(1));
    settle(&mut history);
    assert_eq!(marks(&dir), (20, 100), "nothing gone back");
    assert!(history.view().unwrap().message.contains("netko-drugi"));
}
