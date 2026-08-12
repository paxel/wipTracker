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
        "select",
        "timer",
        "groom",
        "end day",
        "week",
        "revive",
        "window frame",
    ] {
        assert!(
            harness.query_all_by_label_contains(label).count() > 0,
            "menu is missing an entry for {label}"
        );
    }
}

#[test]
fn the_window_frame_can_be_toggled_from_the_menu() {
    let mut harness = harness(Tracker::new(at(9)));
    let before = harness.state().is_decorated();
    harness.state_mut().set_menu_open(true);
    harness.run();

    harness
        .get_by_label_contains("window frame")
        .click_accesskit();
    harness.run();

    assert_eq!(harness.state().is_decorated(), !before);
    assert!(
        harness.state().is_menu_open(),
        "the menu stays open to explain that a restart is needed"
    );
}

#[test]
fn picking_a_task_from_the_stack_window_focuses_it() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "write the report").expect("rename");
    tracker.push_new_task(at(10));

    let mut harness = harness(tracker);
    harness.state_mut().windows_mut().stack = true;
    harness.run();
    assert_eq!(harness.state().tracker().focused_name(), "new task 2");

    harness
        .get_by_label_contains("write the report")
        .click_accesskit();
    harness.run();
    assert_eq!(harness.state().tracker().focused_name(), "write the report");
    assert!(
        !harness.state().windows().stack,
        "picking a task closes the stack window"
    );
}

#[test]
fn the_select_entry_opens_the_stack_window() {
    let mut harness = harness(Tracker::new(at(9)));
    harness.state_mut().set_menu_open(true);
    harness.run();

    harness.get_by_label_contains("select").click_accesskit();
    harness.run();
    assert!(harness.state().windows().stack);
}

#[test]
fn a_new_task_opens_its_name_for_editing() {
    let mut harness = harness(Tracker::new(at(9)));
    harness.run();
    assert!(!harness.state().is_renaming());

    harness.get_by_label("new task").click();
    harness.run();
    assert!(
        harness.state().is_renaming(),
        "the placeholder name should be ready to be typed over"
    );
}

/// kittest clicks with the primary button, so the middle button is sent by hand.
fn middle_click(harness: &mut Harness<'_, WipTracker>, pos: egui::Pos2) {
    let events = &mut harness.input_mut().events;
    events.push(egui::Event::PointerMoved(pos));
    events.push(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Middle,
        pressed: true,
        modifiers: egui::Modifiers::NONE,
    });
    events.push(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Middle,
        pressed: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();
}

fn plus_center() -> egui::Pos2 {
    egui::pos2(
        theme::BAR_SIZE.x - theme::BAR_MARGIN - 1.5 * theme::BUTTON_SIZE.x,
        theme::BAR_SIZE.y / 2.0,
    )
}

fn name_center() -> egui::Pos2 {
    egui::pos2(
        theme::BAR_MARGIN + theme::GRIP_WIDTH + 30.0,
        theme::BAR_SIZE.y / 2.0,
    )
}

#[test]
fn middle_clicking_plus_takes_a_break() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "write the report").expect("rename");

    let mut harness = harness(tracker);
    harness.run();
    assert_eq!(harness.state().tracker().focused_name(), "write the report");

    middle_click(&mut harness, plus_center());
    assert_eq!(harness.state().tracker().focused_name(), PAUSE_NAME);

    // The task is still open underneath, so it is only a break, not a finish.
    assert!(
        harness
            .state()
            .tracker()
            .open_tasks_top_first()
            .iter()
            .any(|task| task.name == "write the report")
    );
}

#[test]
fn middle_clicking_the_name_opens_the_stack_window() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "write the report").expect("rename");

    let mut harness = harness(tracker);
    harness.run();
    assert!(!harness.state().windows().stack);

    middle_click(&mut harness, name_center());
    assert!(harness.state().windows().stack);
}
