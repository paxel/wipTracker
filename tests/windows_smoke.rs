//! Drives the report windows headlessly to prove they build their contents without
//! panicking. The window contents are drawn in their own viewport, which the test harness
//! renders inline.

use chrono::{DateTime, Local, TimeZone as _};
use egui_kittest::Harness;
use wiptracker::app::WipTracker;
use wiptracker::domain::tracker::Tracker;
use wiptracker::theme;

fn at(day: u32, hour: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(2026, 8, day, hour, 0, 0)
        .single()
        .expect("valid local time")
}

fn populated() -> Tracker {
    let mut tracker = Tracker::new(at(10, 9));
    let first = tracker.push_new_task(at(10, 9));
    tracker.rename(first, "write the report").expect("rename");
    let second = tracker.push_new_task(at(10, 11));
    tracker
        .rename(second, "review the pull request")
        .expect("rename");
    tracker.finish_focused(at(10, 12));
    tracker.accrue(at(10, 15));
    tracker
}

fn harness(tracker: Tracker) -> Harness<'static, WipTracker> {
    Harness::builder()
        .with_size(theme::BAR_SIZE)
        .wgpu()
        .build_eframe(|cc| WipTracker::with_tracker(cc, tracker))
}

#[test]
fn every_report_window_can_be_opened() {
    let mut harness = harness(populated());
    {
        let windows = harness.state_mut().windows_mut();
        windows.groom = true;
        windows.end_day = true;
        windows.week = true;
        windows.revive = true;
    }
    harness.run();
    harness.run();

    // Prove the window contents really were built, not silently skipped.
    use egui_kittest::kittest::Queryable as _;
    assert!(harness.query_all_by_label_contains("Open tasks").count() > 0);
    assert!(
        harness
            .query_all_by_label_contains("Finished tasks")
            .count()
            > 0
    );
    assert!(
        harness
            .query_all_by_label_contains("write the report")
            .count()
            > 0
    );
}

#[test]
fn the_windows_survive_an_empty_tracker() {
    let mut harness = harness(Tracker::new(at(10, 9)));
    {
        let windows = harness.state_mut().windows_mut();
        windows.groom = true;
        windows.end_day = true;
        windows.week = true;
        windows.revive = true;
    }
    harness.run();
}

#[test]
fn groom_finishes_the_selected_tasks() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = harness(populated());
    harness.state_mut().windows_mut().groom = true;
    harness.run();

    // Tick the one open task, then finish it.
    harness
        .get_by_role_and_label(egui::accesskit::Role::CheckBox, "write the report")
        .click_accesskit();
    harness.run();
    harness
        .get_by_label_contains("Finish selected")
        .click_accesskit();
    harness.run();

    assert_eq!(
        harness.state().tracker().focused_name(),
        wiptracker::domain::task::PAUSE_NAME
    );
}

#[test]
fn closing_the_day_marks_it_closed() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = harness(populated());
    harness.state_mut().windows_mut().end_day = true;
    harness.run();

    harness.get_by_label_contains("Close day").click_accesskit();
    harness.run();

    let today = Local::now().date_naive();
    assert!(
        harness
            .state()
            .tracker()
            .day(today)
            .is_some_and(|record| record.closed)
    );
}

#[test]
fn the_revive_window_closes_once_nothing_is_left_to_revive() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = harness(populated());
    harness.state_mut().windows_mut().revive = true;
    harness.run();

    harness
        .get_by_label_contains("review the pull request")
        .click_accesskit();
    harness.run();

    assert!(
        !harness.state().windows().revive,
        "the last finished task was revived, so the window should be gone"
    );
}

#[test]
fn reviving_from_the_window_puts_the_task_back_on_top() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = harness(populated());
    harness.state_mut().windows_mut().revive = true;
    harness.run();

    harness
        .get_by_label_contains("review the pull request")
        .click_accesskit();
    harness.run();

    assert_eq!(
        harness.state().tracker().focused_name(),
        "review the pull request"
    );
}

#[test]
fn renaming_the_focused_task_commits_on_enter() {
    let mut harness = harness(populated());
    harness.state_mut().start_rename();
    harness.run();
    assert!(harness.state().is_renaming());

    harness.key_press(egui::Key::Enter);
    harness.run();
    assert!(!harness.state().is_renaming());
}
