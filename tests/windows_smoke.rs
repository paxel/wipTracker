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

/// A tracker with one open and one finished task, dated relative to the real clock: the
/// revive window only lists the last 30 days, so fixed dates would age out and turn the
/// tests red on their own.
fn populated() -> Tracker {
    let start = Local::now() - chrono::TimeDelta::hours(6);
    let mut tracker = Tracker::new(start);
    let first = tracker.push_new_task(start);
    tracker.rename(first, "write the report").expect("rename");
    let second = tracker.push_new_task(start + chrono::TimeDelta::hours(2));
    tracker
        .rename(second, "review the pull request")
        .expect("rename");
    tracker.finish_focused(start + chrono::TimeDelta::hours(3));
    tracker.accrue(start + chrono::TimeDelta::hours(4));
    tracker
}

fn harness(tracker: Tracker) -> Harness<'static, WipTracker> {
    Harness::builder()
        .with_size(theme::bar_size(wiptracker::app::prefers_decorations()))
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

/// An alarm that only records that it went off.
#[derive(Clone, Default)]
struct SilentAlarm(std::sync::Arc<std::sync::atomic::AtomicUsize>);

impl wiptracker::domain::ports::Alarm for SilentAlarm {
    fn sound(&self) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }
}

#[test]
fn the_timer_window_sets_a_task_and_the_default() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = harness(populated());
    harness.state_mut().windows_mut().timer = true;
    harness.run();

    // Open the picker for the default row, then choose one hour.
    harness
        .get_by_label_contains("default for new tasks")
        .click_accesskit();
    harness.run();
    harness.get_by_label("1h").click_accesskit();
    harness.run();

    assert_eq!(
        harness.state().tracker().default_timer(),
        std::time::Duration::from_secs(3600)
    );

    // A task created afterwards inherits it. The click has to go through AccessKit: with
    // the timer window open, a pointer click would land in the wrong viewport.
    harness.get_by_label("new task").click_accesskit();
    harness.run();
    let focused = harness.state().tracker().focused_id();
    assert_eq!(
        harness
            .state()
            .tracker()
            .task(focused)
            .map(|task| task.timer),
        Some(std::time::Duration::from_secs(3600))
    );
}

#[test]
fn a_reached_timer_sounds_the_alarm_once() {
    let counter = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let mut tracker = Tracker::new(at(10, 9));
    let id = tracker.push_new_task(at(10, 9));
    tracker
        .set_timer(id, std::time::Duration::from_secs(1))
        .expect("set timer");
    // Already over the limit, so the very next frame is due.
    tracker.accrue(at(10, 10));

    // The harness draws one frame while it is being built, so the alarm has to be in
    // place before that: an alarm installed afterwards would miss the first firing.
    let alarm = SilentAlarm(counter.clone());
    let mut harness = Harness::builder()
        .with_size(theme::bar_size(wiptracker::app::prefers_decorations()))
        .wgpu()
        .build_eframe(move |cc| {
            let mut app = WipTracker::with_tracker(cc, tracker);
            app.set_alarm(Box::new(alarm));
            app
        });
    harness.run();
    harness.run();
    assert_eq!(counter.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert_eq!(harness.state().alarms_sounded(), [id]);
}

#[test]
fn the_day_window_exports_json_to_the_clipboard() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = harness(populated());
    harness.state_mut().windows_mut().end_day = true;
    harness.run();

    harness.get_by_label("export").click_accesskit();
    // A single frame, not `run()`: `run()` keeps stepping until the ui settles, and the
    // clipboard command belongs to the one frame that handled the click.
    harness.step();

    // The clipboard command really was emitted — the confirmation label alone would still
    // appear if the copy silently went nowhere, which is the one failure that matters for
    // a feature whose whole contract is "it is on your clipboard".
    let copied: Vec<String> = harness
        .output()
        .platform_output
        .commands
        .iter()
        .filter_map(|command| match command {
            egui::OutputCommand::CopyText(text) => Some(text.clone()),
            _ => None,
        })
        .collect();
    assert!(
        copied.iter().any(|text| text.contains("\"seconds\"")),
        "the export button should put the JSON on the clipboard, got {copied:?}"
    );

    // The confirmation appears next to the button.
    assert!(
        harness
            .query_all_by_label_contains("copied to clipboard")
            .count()
            > 0,
        "the export button should confirm that it copied"
    );

    // And the payload is the flat row shape, straight from the domain.
    let today = Local::now().date_naive();
    let json = wiptracker::domain::export::to_json(harness.state().tracker(), &[today]);
    let rows: Vec<serde_json::Value> = serde_json::from_str(&json).expect("valid json");
    assert!(!rows.is_empty(), "today has tracked time");
    for row in &rows {
        assert!(row.get("date").is_some());
        assert!(row.get("task").is_some());
        assert!(row["seconds"].is_u64());
    }
}
