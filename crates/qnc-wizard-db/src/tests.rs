use super::*;
use rusqlite::Connection;

const PROJECT_DB: &str = "qnc://local/db/project_db/p1";

/// A project database of a temporary folder (never a real project).
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

fn wait_for(state: &mut WizardState, target: &ProjectDbTarget, want: &[&str]) {
    let want: BTreeSet<String> = want.iter().map(|step| step.to_string()).collect();
    for _ in 0..1000 {
        if state.poll(Some(target)) == Some(&want) && !state.writing() {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("not {want:?}: {:?} {:?}", state.done(), state.error());
}

#[test]
fn a_project_without_the_wizard_has_no_step_done() {
    let (_dir, target) = project();
    assert!(read(&target).unwrap().is_empty());
}

#[test]
fn a_step_done_is_in_the_database_and_another_window_reads_it() {
    let (_dir, target) = project();
    let mut state = WizardState::new();
    wait_for(&mut state, &target, &[]);
    state.set(&target, "new_story", true);
    state.set(&target, "import", true);
    assert_eq!(state.poll(Some(&target)).unwrap().len(), 2, "shown at once");
    wait_for(&mut state, &target, &["import", "new_story"]);
    assert_eq!(read(&target).unwrap().len(), 2, "in the database");
    state.set(&target, "import", false);
    wait_for(&mut state, &target, &["new_story"]);
    let mut other = WizardState::new();
    wait_for(&mut other, &target, &["new_story"]);
}

#[test]
fn a_step_the_wizard_has_not_is_refused() {
    let (_dir, target) = project();
    let writer = ProjectDbWriter::start(target.clone(), vec![WizardModule::factory()]).unwrap();
    assert!(writer.call(MODULE_ID, encode(&Operation::MarkDone { step: "auto_b_roll".into() }).unwrap()).is_err());
    assert!(read(&target).unwrap().is_empty());
}
