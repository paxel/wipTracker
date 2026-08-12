//! Drives the real app headlessly: renders the bar and clicks its buttons.

use chrono::{DateTime, Local, TimeZone as _};
use egui_kittest::Harness;
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

fn save(harness: &mut Harness<'_, WipTracker>, name: &str) {
    let image = harness.render().expect("wgpu render");
    std::fs::create_dir_all("target/render").expect("create render dir");
    image
        .save(format!("target/render/{name}.png"))
        .expect("save png");
}

#[test]
fn bar_renders_focused_task_and_clock() {
    let mut tracker = Tracker::new(at(9));
    tracker.push_new_task(at(9));
    tracker.accrue(at(10));

    let mut harness = harness(tracker);
    harness.run();
    save(&mut harness, "bar_short");

    assert_eq!(harness.state().tracker().focused_name(), "new task 1");
}

#[test]
fn bar_renders_without_clock() {
    let mut tracker = Tracker::new(at(9));
    tracker.push_new_task(at(9));

    let mut harness = harness(tracker);
    harness.state_mut().set_show_duration(false);
    harness.run();
    save(&mut harness, "bar_no_clock");
}

#[test]
fn bar_truncates_a_long_name() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker
        .rename(id, "a task with a really quite long name that will not fit")
        .expect("rename");
    tracker.accrue(at(10));

    let mut harness = harness(tracker);
    harness.run();
    save(&mut harness, "bar_truncated");
}

#[test]
fn hovering_the_buttons_keeps_them_readable() {
    let mut tracker = Tracker::new(at(9));
    tracker.push_new_task(at(9));

    let mut harness = harness(tracker);
    harness.hover_at(egui::pos2(
        theme::BAR_SIZE.x - 1.5 * theme::BUTTON_SIZE.x,
        theme::BAR_SIZE.y / 2.0,
    ));
    harness.run();
    save(&mut harness, "bar_hover_plus");
}

#[test]
fn the_empty_stack_shows_pause() {
    let mut harness = harness(Tracker::new(at(9)));
    harness.run();
    save(&mut harness, "bar_pause");

    assert_eq!(harness.state().tracker().focused_name(), PAUSE_NAME);
}
