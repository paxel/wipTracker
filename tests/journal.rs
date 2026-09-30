//! The journal writes down every event that moves time from one task to another, so a
//! day whose numbers look wrong can be explained from the file instead of guessed at.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use chrono::Local;
use egui_kittest::Harness;
use wiptracker::app::WipTracker;
use wiptracker::domain::ports::{IdleProbe, Journal};
use wiptracker::domain::task::PAUSE_ID;
use wiptracker::domain::tracker::Tracker;
use wiptracker::theme;

/// Keeps every event in memory for the test to read.
#[derive(Clone, Default)]
struct Memo(Arc<Mutex<Vec<String>>>);

impl Journal for Memo {
    fn record(&self, event: &str) {
        self.0.lock().expect("journal lock").push(event.to_owned());
    }
}

impl Memo {
    fn lines(&self) -> Vec<String> {
        self.0.lock().expect("journal lock").clone()
    }
}

/// Always this idle, so auto-pause fires on the first run.
struct Idle(Duration);

impl IdleProbe for Idle {
    fn idle(&self) -> Option<Duration> {
        Some(self.0)
    }
}

fn harness(tracker: Tracker, memo: &Memo) -> Harness<'static, WipTracker> {
    let memo = memo.clone();
    Harness::builder()
        .with_size(theme::bar_size(wiptracker::app::prefers_decorations()))
        .build_eframe(move |cc| {
            let mut app = WipTracker::with_tracker(cc, tracker);
            app.set_journal(Box::new(memo));
            app
        })
}

/// A switch made straight on the tracker, as the stack window does it, is journaled with
/// both names.
#[test]
fn a_change_of_focus_is_written_down_with_both_names() {
    let now = Local::now();
    let mut tracker = Tracker::new(now);
    let task = tracker.push_new_task(now);
    tracker.rename(task, "write the report").expect("rename");

    let memo = Memo::default();
    let mut harness = harness(tracker, &memo);
    harness.run();
    assert_eq!(
        memo.lines(),
        vec!["focus 'write the report' (was 'nothing')"],
        "the first run records what is on top"
    );

    let now = Local::now();
    let _ = harness.state_mut().tracker_mut().select(PAUSE_ID, now);
    harness.run();
    assert_eq!(
        memo.lines().last().map(String::as_str),
        Some("focus 'pause' (was 'write the report')")
    );
}

/// An auto-pause names the idle span it took back and the task it took it from.
#[test]
fn an_auto_pause_is_written_down_with_the_idle_span() {
    let now = Local::now();
    let mut tracker = Tracker::new(now);
    let task = tracker.push_new_task(now);
    tracker.rename(task, "write the report").expect("rename");
    tracker.set_idle_pause(Duration::from_secs(300));

    let memo = Memo::default();
    let mut harness = harness(tracker, &memo);
    harness
        .state_mut()
        .set_idle_probe(Box::new(Idle(Duration::from_secs(600))));
    harness.run();

    let lines = memo.lines();
    assert!(
        lines.contains(&"auto-pause after 10:00 idle: took that off 'write the report'".to_owned()),
        "{lines:?}"
    );
    assert_eq!(harness.state().tracker().focused_id(), PAUSE_ID);
}
