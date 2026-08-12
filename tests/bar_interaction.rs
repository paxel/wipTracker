//! Clicks the real app's buttons and checks the state that comes out.

use chrono::{DateTime, Local, TimeZone as _};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use wiptracker::app::WipTracker;
use wiptracker::domain::task::PAUSE_NAME;
use wiptracker::domain::tracker::Tracker;
use wiptracker::theme;

fn at(hour: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(2026, 8, 12, hour, 0, 0)
        .single()
        .expect("valid local time")
}

fn harness(tracker: Tracker) -> Harness<'static, WipTracker> {
    Harness::builder()
        .with_size(theme::BAR_SIZE)
        .wgpu()
        .build_eframe(|cc| WipTracker::with_tracker(cc, tracker))
}

#[test]
fn clicking_plus_creates_and_focuses_a_task() {
    let mut harness = harness(Tracker::new(at(9)));
    assert_eq!(harness.state().tracker().focused_name(), PAUSE_NAME);

    harness.get_by_label("new task").click();
    harness.run();
    assert_eq!(harness.state().tracker().focused_name(), "new task 1");

    harness.get_by_label("new task").click();
    harness.run();
    assert_eq!(harness.state().tracker().focused_name(), "new task 2");
}

#[test]
fn clicking_the_burger_opens_the_menu() {
    let mut harness = harness(Tracker::new(at(9)));
    assert!(!harness.state().is_menu_open());

    harness.get_by_label("menu").click();
    harness.run();
    assert!(harness.state().is_menu_open());
}

#[test]
fn the_menu_lists_its_entries_and_the_open_tasks() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "write the report").expect("rename");

    let mut harness = harness(tracker);
    harness.state_mut().set_menu_open(true);
    harness.run();

    for label in [
        "write the report",
        PAUSE_NAME,
        "groom",
        "end day",
        "week",
        "revive",
    ] {
        assert!(
            harness.query_all_by_label_contains(label).count() > 0,
            "menu is missing an entry for {label}"
        );
    }
}

#[test]
fn picking_a_task_from_the_menu_focuses_it() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "write the report").expect("rename");
    tracker.push_new_task(at(10));

    let mut harness = harness(tracker);
    harness.state_mut().set_menu_open(true);
    harness.run();
    assert_eq!(harness.state().tracker().focused_name(), "new task 2");

    harness
        .get_by_label_contains("write the report")
        .click_accesskit();
    harness.run();
    assert_eq!(harness.state().tracker().focused_name(), "write the report");
    assert!(!harness.state().is_menu_open());
}
