use super::*;
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

fn wait(history: &mut ClipHistory, target: &ProjectDbTarget, clip: &str, count: usize) -> Vec<Entry> {
    for _ in 0..1000 {
        if let Some(entries) = history.for_clip(Some(target), Some(clip)) {
            if entries.len() == count && entries.iter().all(|entry| entry.id > 0) {
                return entries;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
    panic!("not read")
}

/// User 2026-11-25: the history is kept per clip and read from the project's journal; a clip's
/// first Save keeps where IN and OUT were before it, a going back is a position like any other.
#[test]
fn the_history_of_a_clip_is_its_rows_writes_in_the_journal() {
    let (_dir, target) = project();
    assert!(read(&target, "a").unwrap().is_empty(), "a project without the table has no history");
    let mut history = ClipHistory::new();
    history.keep(&target, "a", (0, 100), (10, 100));
    history.keep(&target, "b", (0, 50), (5, 50));
    history.keep(&target, "a", (10, 100), (12, 90));
    history.back(&target, "a", (0, 100));
    let a = wait(&mut history, &target, "a", 4);
    assert_eq!(a.iter().map(|entry| (entry.in_frame, entry.out_frame, entry.back)).collect::<Vec<_>>(), [(0, 100, false), (10, 100, false), (12, 90, false), (0, 100, true)]);
    assert!(a.iter().all(|entry| !entry.author.is_empty() && !entry.made_at_utc.is_empty()), "who and when, from the journal");
    assert_eq!(read(&target, "b").unwrap().len(), 2, "another clip has its own history");
}

#[test]
fn marks_without_a_clip_or_with_out_before_in_are_refused() {
    let (_dir, target) = project();
    let mut client = target.open(Access::ReadWrite, vec![ClipMarksModule::factory()]).unwrap();
    assert!(client.execute(MODULE_ID, encode(&Operation::Back { clip_id: String::new(), to: (0, 10) }).unwrap()).is_err());
    assert!(client.execute(MODULE_ID, encode(&Operation::Back { clip_id: "a".into(), to: (10, 10) }).unwrap()).is_err());
}
