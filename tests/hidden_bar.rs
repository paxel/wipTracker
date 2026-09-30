//! The bar keeps counting while its window is out of sight. eframe runs `App::ui` only
//! for a visible window and `App::logic` whenever a repaint was asked for, so the clock
//! has to live in `logic`: on macOS every other Space, every fullscreen app, a sleeping
//! display and the lock screen occlude the bar, and each such span used to be dropped as
//! if the machine had slept.

use std::time::Duration;

use chrono::{Local, TimeDelta};
use egui_kittest::Harness;
use wiptracker::app::WipTracker;
use wiptracker::domain::tracker::Tracker;

/// The bar with its drawing switched off: exactly what eframe runs for an occluded window.
struct Hidden(WipTracker);

impl eframe::App for Hidden {
    fn logic(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.0.logic(ctx, frame);
    }

    fn ui(&mut self, _ui: &mut egui::Ui, _frame: &mut eframe::Frame) {}
}

#[test]
fn time_is_credited_without_a_single_painted_frame() {
    let start = Local::now() - TimeDelta::hours(3);
    let mut tracker = Tracker::new(start);
    let task = tracker.push_new_task(start);

    let mut harness =
        Harness::builder().build_eframe(|cc| Hidden(WipTracker::with_tracker(cc, tracker)));
    harness.run();

    let total = harness
        .state()
        .0
        .tracker()
        .task(task)
        .expect("the task is still there")
        .total;
    assert!(
        total >= Duration::from_secs(3 * 60 * 60),
        "three hours on the task, none of them painted: got {total:?}"
    );
}
