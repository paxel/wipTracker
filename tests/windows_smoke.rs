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

/// An alarm that only records what went off: task sounds count single, day sounds count
/// by the hundred, so a test can tell them apart in one number.
#[derive(Clone, Default)]
struct SilentAlarm(std::sync::Arc<std::sync::atomic::AtomicUsize>);

const DAY_SOUND: usize = 100;

impl wiptracker::domain::ports::Alarm for SilentAlarm {
    fn sound(&self, _task: &str) {
        self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    }

    fn sound_day_over(&self) {
        self.0
            .fetch_add(DAY_SOUND, std::sync::atomic::Ordering::Relaxed);
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
fn the_timer_window_sets_the_day_timer() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = harness(populated());
    harness.state_mut().windows_mut().timer = true;
    harness.run();

    harness
        .get_by_label_contains("the whole day")
        .click_accesskit();
    harness.run();
    harness.get_by_label("8h").click_accesskit();
    harness.run();

    assert_eq!(
        harness.state().tracker().day_timer(),
        std::time::Duration::from_secs(8 * 3600)
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

/// The launcher offer: installing writes the entry and icons into the given directory
/// and never asks again; declining never asks again either.
#[test]
fn the_launcher_offer_installs_and_is_not_asked_again() {
    use egui_kittest::kittest::Queryable as _;

    let scratch = tempfile::tempdir().expect("tempdir");
    let data_home = scratch.path().to_path_buf();
    let into = data_home.clone();

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(640.0, 480.0))
        .wgpu()
        .build_eframe(move |cc| {
            let mut app = WipTracker::with_tracker(cc, Tracker::new(at(1, 9)));
            app.set_launcher_data_home(into);
            app.windows_mut().launcher_offer = true;
            app
        });
    harness.run();

    harness.get_by_label("Add to the menu").click_accesskit();
    harness.run();

    let entry = data_home.join("applications/wiptracker.desktop");
    assert!(entry.exists(), "the entry was written");
    let written = std::fs::read_to_string(&entry).expect("read entry");
    assert!(
        written.contains("TryExec="),
        "the entry hides itself once the binary is uninstalled"
    );
    assert!(
        data_home
            .join("icons/hicolor/256x256/apps/wiptracker.png")
            .exists(),
        "the icons came with it"
    );
    assert!(
        !harness.state().windows().launcher_offer,
        "the offer closes after installing"
    );
}

#[test]
fn declining_the_launcher_offer_is_remembered() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(640.0, 480.0))
        .wgpu()
        .build_eframe(|cc| {
            let mut app = WipTracker::with_tracker(cc, Tracker::new(at(1, 9)));
            app.windows_mut().launcher_offer = true;
            app
        });
    harness.run();

    harness.get_by_label("Don't ask again").click_accesskit();
    harness.run();
    assert!(!harness.state().windows().launcher_offer);
}

/// A fixed answer instead of the operating system's idle clock.
struct AlwaysIdle(std::time::Duration);

impl wiptracker::domain::ports::IdleProbe for AlwaysIdle {
    fn idle(&self) -> Option<std::time::Duration> {
        Some(self.0)
    }
}

/// With auto-pause configured, a long-idle user is moved to the break by the app loop
/// itself; without the setting nothing watches at all.
#[test]
fn a_long_idle_starts_the_break_when_asked_to() {
    use wiptracker::domain::task::PAUSE_NAME;

    let mut tracker = Tracker::new(at(1, 9));
    let id = tracker.push_new_task(at(1, 9));
    tracker.rename(id, "write the report").expect("rename");
    tracker.set_idle_pause(std::time::Duration::from_secs(300));

    let mut harness = egui_kittest::Harness::builder()
        .with_size(theme::bar_size(wiptracker::app::prefers_decorations()))
        .wgpu()
        .build_eframe(|cc| {
            let mut app = WipTracker::with_tracker(cc, tracker);
            app.set_idle_probe(Box::new(AlwaysIdle(std::time::Duration::from_secs(600))));
            app
        });
    harness.run();

    assert_eq!(harness.state().tracker().focused_name(), PAUSE_NAME);
}

/// The menu toggle writes and removes the autostart entry, against a scratch directory.
#[test]
fn the_menu_toggles_starting_with_the_session() {
    use egui_kittest::kittest::Queryable as _;

    let scratch = tempfile::tempdir().expect("tempdir");
    let config = scratch.path().to_path_buf();
    let into = config.clone();

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(640.0, 480.0))
        .wgpu()
        .build_eframe(move |cc| {
            let mut app = WipTracker::with_tracker(cc, Tracker::new(at(1, 9)));
            app.set_autostart_config_home(into);
            app.set_menu_open(true);
            app
        });
    harness.run();

    harness
        .get_by_label_contains("start with my session")
        .click_accesskit();
    harness.run();
    assert!(
        config.join("autostart/wiptracker.desktop").exists(),
        "the toggle wrote the autostart entry"
    );

    harness.state_mut().set_menu_open(true);
    harness.run();
    harness
        .get_by_label_contains("stop starting with my session")
        .click_accesskit();
    harness.run();
    assert!(
        !config.join("autostart/wiptracker.desktop").exists(),
        "the toggle removed it again"
    );
}

/// A menu entry under test: its label, whether its window is open, and how to close it.
type MenuEntryCheck = (&'static str, fn(&WipTracker) -> bool, fn(&mut WipTracker));

/// Every menu entry that opens a window really opens it.
#[test]
fn the_menu_opens_each_window() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(640.0, 480.0))
        .wgpu()
        .build_eframe(|cc| {
            let mut app = WipTracker::with_tracker(cc, populated());
            app.set_menu_open(true);
            app
        });
    harness.run();

    // Each entry closes the menu, and the opened window is closed again before the next
    // round so its contents cannot shadow the next entry's label.
    let entries: &[MenuEntryCheck] = &[
        (
            "select",
            |app| app.windows().stack,
            |app| {
                app.windows_mut().stack = false;
            },
        ),
        (
            "timer",
            |app| app.windows().timer,
            |app| {
                app.windows_mut().timer = false;
            },
        ),
        (
            "groom",
            |app| app.windows().groom,
            |app| {
                app.windows_mut().groom = false;
            },
        ),
        (
            "end day",
            |app| app.windows().end_day,
            |app| {
                app.windows_mut().end_day = false;
            },
        ),
        (
            "week",
            |app| app.windows().week,
            |app| {
                app.windows_mut().week = false;
            },
        ),
        (
            "revive",
            |app| app.windows().revive,
            |app| {
                app.windows_mut().revive = false;
            },
        ),
    ];
    for (label, is_open, close) in entries {
        harness.get_by_label(label).click_accesskit();
        harness.run();
        assert!(is_open(harness.state()), "{label} did not open its window");
        close(harness.state_mut());
        harness.state_mut().set_menu_open(true);
        harness.run();
    }
}

/// The task entries in the menu do what their bar gestures do.
#[test]
fn the_menu_drives_the_task_actions() {
    use egui_kittest::kittest::Queryable as _;
    use wiptracker::domain::task::PAUSE_NAME;

    let mut tracker = populated();
    tracker.set_day_timer(std::time::Duration::from_secs(8 * 3600));
    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(640.0, 480.0))
        .wgpu()
        .build_eframe(|cc| {
            let mut app = WipTracker::with_tracker(cc, tracker);
            app.set_menu_open(true);
            app
        });
    harness.run();

    let before = harness.state().tracker().focused_name().to_owned();
    // The bar's plus button carries the same accessible label; the menu's entry is the
    // later node, since the menu is drawn after the bar.
    harness
        .query_all_by_label("new task")
        .last()
        .expect("the menu shows a new task entry")
        .click_accesskit();
    harness.run();
    assert_ne!(harness.state().tracker().focused_name(), before);

    harness.state_mut().set_menu_open(true);
    harness.run();
    harness.get_by_label("pause").click_accesskit();
    harness.run();
    assert_eq!(harness.state().tracker().focused_name(), PAUSE_NAME);

    harness.state_mut().set_menu_open(true);
    harness.run();
    harness.get_by_label("end break").click_accesskit();
    harness.run();
    assert_ne!(harness.state().tracker().focused_name(), PAUSE_NAME);

    harness.state_mut().set_menu_open(true);
    harness.run();
    harness.get_by_label("rename").click_accesskit();
    harness.run();
    assert!(harness.state().is_renaming());

    harness.state_mut().set_menu_open(true);
    harness.run();
    harness.get_by_label("mute day reminder").click_accesskit();
    harness.run();
    let today = Local::now().date_naive();
    assert!(harness.state().tracker().nag_muted(today));
}

/// The menu toggle flips the taskbar preference both ways. It is a stored preference —
/// the window itself only picks it up on the next start.
#[test]
fn the_menu_toggles_the_taskbar_entry() {
    use egui_kittest::kittest::Queryable as _;

    let mut harness = egui_kittest::Harness::builder()
        .with_size(egui::vec2(640.0, 480.0))
        .wgpu()
        .build_eframe(move |cc| {
            let mut app = WipTracker::with_tracker(cc, Tracker::new(at(1, 9)));
            app.set_menu_open(true);
            app
        });
    harness.run();
    assert!(
        harness.state().shows_in_taskbar(),
        "the bar starts out in the taskbar"
    );

    harness
        .get_by_label_contains("leave the taskbar")
        .click_accesskit();
    harness.run();
    assert!(!harness.state().shows_in_taskbar());

    harness.state_mut().set_menu_open(true);
    harness.run();
    harness
        .get_by_label_contains("show up in the taskbar")
        .click_accesskit();
    harness.run();
    assert!(harness.state().shows_in_taskbar());
}
