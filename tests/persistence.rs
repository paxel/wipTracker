//! Starts the real app from a stored database and checks that what it changes lands back
//! on disk.

use chrono::{DateTime, Local, TimeZone as _};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use wiptracker::app::WipTracker;
use wiptracker::domain::ports::Store as _;
use wiptracker::domain::tracker::Tracker;
use wiptracker::infrastructure::redb_store::RedbStore;
use wiptracker::theme;

fn at(hour: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(2026, 8, 12, hour, 0, 0)
        .single()
        .expect("valid local time")
}

#[test]
fn the_app_resumes_the_stored_state_and_writes_its_own_changes_back() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("state.redb");

    // A previous session: one named task on top, the clock hidden, a known position.
    {
        let mut tracker = Tracker::new(at(9));
        let id = tracker.push_new_task(at(9));
        tracker.rename(id, "write the report").expect("rename");
        tracker.accrue(at(11));

        let store = RedbStore::open(&path).expect("open");
        store
            .save(&tracker.snapshot(false, Some(true), Some((120.0, 40.0))))
            .expect("save");
    }

    // This session: the app is started the way the binary starts it.
    let store = RedbStore::open(&path).expect("reopen");
    let snapshot = store.load().expect("load");
    assert!(snapshot.is_some(), "the stored snapshot should be found");

    let mut harness = Harness::builder()
        .with_size(theme::bar_size(wiptracker::app::prefers_decorations()))
        .wgpu()
        .build_eframe(|cc| WipTracker::start(cc, Box::new(store), snapshot));
    harness.run();

    assert_eq!(
        harness.state().tracker().focused_name(),
        "write the report",
        "the task that was focused last time should be focused again"
    );

    // A change made through the UI must reach the database.
    harness.get_by_label("new task").click();
    harness.run();
    assert_eq!(harness.state().tracker().focused_name(), "new task 2");

    // redb keeps an exclusive lock, so the app has to let go before the file can be read
    // back — which is also why a second WipTracker instance refuses to start.
    drop(harness);

    let store = RedbStore::open(&path).expect("reopen after the change");
    let stored = store.load().expect("load").expect("a snapshot");
    let names: Vec<&str> = stored
        .tasks
        .values()
        .map(|task| task.name.as_str())
        .collect();
    assert!(names.contains(&"write the report"));
    assert!(names.contains(&"new task 2"));
    assert!(
        !stored.show_duration,
        "the hidden clock should have survived the restart"
    );
    assert_eq!(
        stored.decorated,
        Some(true),
        "the window frame preference should have survived the restart"
    );
    assert_eq!(stored.next_number, 3);
}

#[test]
fn a_first_run_starts_from_an_empty_database() {
    let dir = tempfile::tempdir().expect("temp dir");
    let store = RedbStore::open(dir.path().join("state.redb")).expect("open");
    let snapshot = store.load().expect("load");
    assert!(snapshot.is_none());

    let mut harness = Harness::builder()
        .with_size(theme::bar_size(wiptracker::app::prefers_decorations()))
        .wgpu()
        .build_eframe(|cc| WipTracker::start(cc, Box::new(store), snapshot));
    harness.run();

    assert_eq!(
        harness.state().tracker().focused_name(),
        wiptracker::domain::task::PAUSE_NAME
    );
}
