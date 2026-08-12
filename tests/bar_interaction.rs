//! Clicks and holds the real app's buttons and checks the state that comes out.

use chrono::{DateTime, Local, TimeZone as _};
use egui_kittest::Harness;
use egui_kittest::kittest::Queryable as _;
use wiptracker::app::{WipTracker, prefers_decorations};
use wiptracker::domain::task::PAUSE_NAME;
use wiptracker::domain::tracker::Tracker;
use wiptracker::theme;
use wiptracker::ui::bar::{HOLD_FINISH, HOLD_QUICK};

/// How far each frame moves the clock on. Coarse on purpose: a two-second hold is then
/// five frames rather than a two-second sleep.
const STEP: f32 = 0.5;

fn at(hour: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(2026, 8, 12, hour, 0, 0)
        .single()
        .expect("valid local time")
}

fn bar_size() -> egui::Vec2 {
    theme::bar_size(prefers_decorations())
}

fn harness(tracker: Tracker) -> Harness<'static, WipTracker> {
    Harness::builder()
        .with_size(bar_size())
        .with_step_dt(STEP)
        .wgpu()
        .build_eframe(|cc| WipTracker::with_tracker(cc, tracker))
}

/// The centre of one of the three buttons, counted from the right edge: the burger is 0,
/// the plus 1, the fork 2.
fn button_center(from_right: f32) -> egui::Pos2 {
    let spacing = 2.0;
    egui::pos2(
        bar_size().x
            - theme::BAR_MARGIN
            - from_right * (theme::BUTTON_SIZE.x + spacing)
            - theme::BUTTON_SIZE.x / 2.0,
        theme::BAR_HEIGHT / 2.0,
    )
}

fn name_center() -> egui::Pos2 {
    egui::pos2(
        theme::BAR_MARGIN + theme::grip_width(prefers_decorations()) + 30.0,
        theme::BAR_HEIGHT / 2.0,
    )
}

fn pointer(harness: &mut Harness<'_, WipTracker>, pos: egui::Pos2, pressed: bool) {
    let events = &mut harness.input_mut().events;
    events.push(egui::Event::PointerMoved(pos));
    events.push(egui::Event::PointerButton {
        pos,
        button: egui::PointerButton::Primary,
        pressed,
        modifiers: egui::Modifiers::NONE,
    });
}

/// Holds the primary button down over `pos` for `seconds`, then lets go.
///
/// Frames are stepped rather than run, because a live press asks for a repaint every
/// frame and `run` would keep going until it gave up.
fn hold(harness: &mut Harness<'_, WipTracker>, pos: egui::Pos2, seconds: f32) {
    pointer(harness, pos, true);
    harness.step();
    let frames = (seconds / STEP).ceil() as usize;
    for _ in 0..frames {
        harness.step();
    }
    pointer(harness, pos, false);
    harness.step();
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
fn the_burger_closes_the_menu_again() {
    let mut harness = harness(Tracker::new(at(9)));
    harness.get_by_label("menu").click();
    harness.run();
    assert!(harness.state().is_menu_open());

    // Through the accessibility tree, not a pointer: the harness draws the menu window
    // over the bar instead of beside it, so a click at the burger's position would land
    // on the menu.
    harness.get_by_label("menu").click_accesskit();
    harness.run();
    assert!(
        !harness.state().is_menu_open(),
        "the burger toggles the menu, it does not only open it"
    );
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
        "new task",
        "rename",
        "finish",
        "pause",
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

#[test]
fn clicking_the_fork_opens_the_stack_window() {
    let mut harness = harness(Tracker::new(at(9)));
    harness.run();
    assert!(!harness.state().windows().stack);

    harness.get_by_label("task stack").click();
    harness.run();
    assert!(harness.state().windows().stack);
}

#[test]
fn holding_the_fork_takes_a_break() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "write the report").expect("rename");

    let mut harness = harness(tracker);
    harness.run();
    assert_eq!(harness.state().tracker().focused_name(), "write the report");

    hold(&mut harness, button_center(2.0), HOLD_QUICK);
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
    assert!(
        !harness.state().windows().stack,
        "the hold replaces the click, it does not also fire it"
    );
}

#[test]
fn holding_the_name_finishes_the_task() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "write the report").expect("rename");

    let mut harness = harness(tracker);
    harness.run();

    hold(&mut harness, name_center(), HOLD_FINISH);
    assert_eq!(harness.state().tracker().focused_name(), PAUSE_NAME);
    assert!(
        !harness.state().is_renaming(),
        "the hold replaces the click, so no rename is started"
    );
}

#[test]
fn letting_go_of_the_name_early_renames_instead() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "write the report").expect("rename");

    let mut harness = harness(tracker);
    harness.run();

    // Well past egui's own click limit, but short of the finish hold: still a click.
    hold(&mut harness, name_center(), HOLD_FINISH - STEP * 2.0);
    assert_eq!(harness.state().tracker().focused_name(), "write the report");
    assert!(
        harness.state().is_renaming(),
        "a slow click is still a click"
    );
}

/// The whole name column is the target, not just the pixels the text happens to cover.
#[test]
fn the_name_can_be_held_beside_a_short_name() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "mail").expect("rename");

    let mut harness = harness(tracker);
    harness.run();

    let beside = egui::pos2(
        theme::BAR_MARGIN + theme::grip_width(prefers_decorations()) + 120.0,
        theme::BAR_HEIGHT / 2.0,
    );
    hold(&mut harness, beside, HOLD_FINISH);
    assert_eq!(harness.state().tracker().focused_name(), PAUSE_NAME);
}

/// The rename editor shares the pointer machinery that the holds changed, so it is worth
/// driving end to end rather than only rendering it.
#[test]
fn a_rename_typed_into_the_bar_is_committed() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "write the report").expect("rename");

    let mut harness = harness(tracker);
    harness.run();

    hold(&mut harness, name_center(), 0.0);
    harness.run();
    assert!(harness.state().is_renaming());

    harness
        .input_mut()
        .events
        .push(egui::Event::Text("book the trip".to_owned()));
    harness.step();
    harness.input_mut().events.push(egui::Event::Key {
        key: egui::Key::Enter,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: egui::Modifiers::NONE,
    });
    harness.run();

    assert!(!harness.state().is_renaming());
    assert_eq!(harness.state().tracker().focused_name(), "book the trip");
}

#[test]
fn holding_the_burger_opens_the_timer() {
    let mut harness = harness(Tracker::new(at(9)));
    harness.run();

    hold(&mut harness, button_center(0.0), HOLD_QUICK);
    assert!(harness.state().windows().timer);
    assert!(
        !harness.state().is_menu_open(),
        "the hold replaces the click, so the menu stays shut"
    );
}

#[test]
fn holding_plus_opens_revive_only_when_there_is_something_to_revive() {
    let mut empty = harness(Tracker::new(at(9)));
    empty.run();

    hold(&mut empty, button_center(1.0), HOLD_QUICK);
    assert!(
        !empty.state().windows().revive,
        "nothing has been finished, so the hold leads nowhere"
    );

    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "write the report").expect("rename");
    tracker.finish_focused(at(10));

    let mut revivable = harness(tracker);
    revivable.run();
    hold(&mut revivable, button_center(1.0), HOLD_QUICK);
    assert!(revivable.state().windows().revive);
}
