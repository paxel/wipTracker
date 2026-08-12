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

/// The bar's clock is today's time, and it turns amber once the daily timer is reached.
#[test]
fn the_clock_marks_a_task_that_is_over_its_timer() {
    let start = Local::now();
    let mut tracker = Tracker::new(start);
    let id = tracker.push_new_task(start);
    tracker.rename(id, "write the report").expect("rename");
    tracker
        .set_timer(id, std::time::Duration::from_secs(1800))
        .expect("set timer");
    // An hour today, against a half-hour limit.
    tracker.accrue(start + chrono::TimeDelta::hours(1));

    let mut harness = harness(tracker);
    harness.run();
    save(&mut harness, "bar_over_limit");

    let image = harness.render().expect("wgpu render");
    let amber = image.pixels().any(|pixel| {
        let [r, g, b, _] = pixel.0;
        r > 0xC0 && (0x90..=0xD0).contains(&g) && b < 0x80
    });
    assert!(amber, "the clock should be amber once the timer is reached");
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

/// Regenerates the screenshot the README shows. Run with:
/// `cargo test --test bar_render docs_screenshot -- --ignored`
#[test]
#[ignore = "writes into docs/, run on purpose"]
fn docs_screenshot() {
    // Accruing into the future pins the clock: the app's own accrual then adds nothing,
    // so the screenshot reads the same every time it is regenerated.
    let start = Local::now();
    let mut tracker = Tracker::new(start);
    let id = tracker.push_new_task(start);
    tracker
        .rename(id, "write the release notes")
        .expect("rename");
    tracker.accrue(start + chrono::TimeDelta::seconds(5025));

    let mut harness = Harness::builder()
        .with_size(theme::BAR_SIZE)
        .with_pixels_per_point(3.0)
        .wgpu()
        .build_eframe(|cc| WipTracker::with_tracker(cc, tracker));
    harness.run();

    let image = harness.render().expect("wgpu render");
    std::fs::create_dir_all("docs").expect("create docs dir");
    image.save("docs/bar.png").expect("save png");
}

/// The desktop's own light theme must not leak into the bar.
///
/// A mac user's recording showed exactly that: white menu buttons with near-white labels,
/// and a white rename field with a white caret, because egui follows the system theme by
/// default and re-applies it every frame.
#[test]
fn the_bar_stays_dark_on_a_light_desktop() {
    let mut tracker = Tracker::new(at(9));
    let id = tracker.push_new_task(at(9));
    tracker.rename(id, "arbeit").expect("rename");

    let mut harness = harness(tracker);
    // Whatever the desktop asks for, the app pins its own theme — and, belt and braces,
    // installs the same dark palette under the light theme as well.
    harness.ctx.set_theme(egui::ThemePreference::Light);
    harness.run();
    assert_eq!(
        harness.ctx.options(|options| options.theme_preference),
        egui::ThemePreference::Dark,
        "the app pins the dark theme regardless of the desktop"
    );
    harness.state_mut().start_rename();
    harness.run();
    harness.run();
    save(&mut harness, "bar_rename_dark");

    let image = harness.render().expect("wgpu render");
    let width = image.width();
    // The rename field spans the left of the bar; sample its middle rows.
    let bright = image
        .enumerate_pixels()
        .filter(|(x, y, _)| *x > 20 && *x < width / 2 && *y > 8 && *y < 24)
        .filter(|(_, _, pixel)| {
            let [r, g, b, _] = pixel.0;
            r > 0xC8 && g > 0xC8 && b > 0xC8
        })
        .count();
    assert!(
        bright < 40,
        "the rename field should be dark, found {bright} near-white pixels"
    );
}
