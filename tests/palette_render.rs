//! Renders the light and shuffled palettes, and the stack window's emphasis fade.
//!
//! Its own test binary on purpose: the palette is process-global, and the renders in
//! `bar_render.rs` assert the stock dark colours. One test function, not several — tests
//! in one binary run in parallel and would race the global palette.

use chrono::{DateTime, Local, TimeZone as _};
use egui_kittest::Harness;
use wiptracker::app::{WipTracker, prefers_decorations};
use wiptracker::domain::tracker::Tracker;
use wiptracker::theme;

fn at(hour: u32) -> DateTime<Local> {
    Local
        .with_ymd_and_hms(2026, 8, 12, hour, 0, 0)
        .single()
        .expect("valid local time")
}

fn bar_harness(tracker: Tracker) -> Harness<'static, WipTracker> {
    Harness::builder()
        .with_size(theme::bar_size(prefers_decorations()))
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

fn stacked_tracker() -> Tracker {
    let mut tracker = Tracker::new(at(9));
    for name in [
        "write the report",
        "review the patch",
        "answer mail",
        "tidy the desk",
    ] {
        let id = tracker.push_new_task(at(9));
        tracker.rename(id, name).expect("rename");
    }
    tracker.accrue(at(10));
    tracker
}

#[test]
fn the_palettes_render_readable() {
    // The stack window under the stock dark palette: the top row is bigger and the names
    // fade towards the dim tone.
    theme::set_current(theme::DARK);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(460.0, 420.0))
        .wgpu()
        .build_eframe(|cc| WipTracker::with_tracker(cc, stacked_tracker()));
    harness.state_mut().windows_mut().stack = true;
    harness.run();
    save(&mut harness, "stack_gradient_dark");
    drop(harness);

    // The light palette: a bright bar carrying dark text.
    theme::set_current(theme::LIGHT);
    let mut harness = bar_harness(stacked_tracker());
    harness.run();
    save(&mut harness, "bar_light");
    let image = harness.render().expect("wgpu render");
    let total = (image.width() * image.height()) as f32;
    let bright = image
        .pixels()
        .filter(|pixel| {
            let [r, g, b, _] = pixel.0;
            r > 0xD8 && g > 0xD8 && b > 0xD8
        })
        .count() as f32;
    let dark = image
        .pixels()
        .filter(|pixel| {
            let [r, g, b, _] = pixel.0;
            r < 0x50 && g < 0x50 && b < 0x50
        })
        .count();
    assert!(
        bright / total > 0.3,
        "the light bar should be mostly bright, found {:.0}%",
        100.0 * bright / total
    );
    assert!(
        dark > 50,
        "dark text should sit on the light bar, found {dark} dark pixels"
    );
    drop(harness);

    // A shuffled dark palette: still dark, still carrying light text — the rotation only
    // moves the hue.
    theme::set_current(theme::DARK.rotated(140.0));
    let mut harness = bar_harness(stacked_tracker());
    harness.run();
    save(&mut harness, "bar_shuffled");
    let image = harness.render().expect("wgpu render");
    let light_text = image
        .pixels()
        .filter(|pixel| {
            let [r, g, b, _] = pixel.0;
            r > 0xC8 && g > 0xC8 && b > 0xC8
        })
        .count();
    assert!(
        light_text > 50,
        "light text should survive the shuffle, found {light_text} bright pixels"
    );

    theme::set_current(theme::DARK);
}
