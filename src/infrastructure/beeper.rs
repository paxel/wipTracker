//! What a timer running out sounds and looks like: a beep, and a desktop notification.
//!
//! Both are best-effort: a machine with no output device or no notification daemon
//! should keep tracking time rather than fail, so every error is swallowed — the audio
//! one after being reported once.

use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use rodio::source::{SineWave, Source as _};

use crate::domain::ports::Alarm;

/// Two short tones, played on a scratch thread so the UI never waits for audio.
pub struct Beeper;

/// Whether the "no audio output" complaint has already been printed.
static REPORTED: AtomicBool = AtomicBool::new(false);

fn report(error: &str) {
    if !REPORTED.swap(true, Ordering::Relaxed) {
        eprintln!("wiptracker: the timer alarm cannot be played: {error}");
    }
}

impl Default for Beeper {
    fn default() -> Self {
        Self::new()
    }
}

impl Beeper {
    pub fn new() -> Self {
        Self
    }
}

impl Alarm for Beeper {
    /// Returns immediately: tones and notification happen on a scratch thread.
    fn sound(&self, task: &str) {
        let task = task.to_owned();
        std::thread::spawn(move || announce_task(&task));
    }

    /// Returns immediately, like [`Self::sound`]. Three falling tones where the task
    /// alarm rises — the day being over should not sound like one more task.
    fn sound_day_over(&self) {
        std::thread::spawn(announce_day_over);
    }
}

/// Everything a task alarm does, on whatever thread called it.
fn announce_task(task: &str) {
    notify(
        &format!("{task} reached its daily timer"),
        "The clock on the bar stays amber for the rest of the day.",
    );
    play(TASK_TONES);
}

/// Everything the day alarm does, on whatever thread called it.
fn announce_day_over() {
    notify(
        "The day's timer is reached",
        "The bar clock turns red for the rest of the day. The reminder repeats every \
         ten minutes; the menu can mute it for today.",
    );
    play(DAY_TONES);
}

/// A desktop notification, so a muted machine still hears about its timers. Failure is
/// silent: the beep alongside it is the fallback, and there is nothing to repair.
fn notify(summary: &str, body: &str) {
    let _ = notify_rust::Notification::new()
        .appname("WipTracker")
        .summary(summary)
        .body(body)
        .icon("wiptracker")
        .show();
}

/// Frequency and length of each tone, in order.
type Tones = &'static [(f32, u64)];

/// Two short rising tones: one task's daily timer.
const TASK_TONES: Tones = &[(880.0, 160), (1320.0, 220)];
/// Three longer falling tones: the whole day's timer.
const DAY_TONES: Tones = &[(1320.0, 220), (880.0, 220), (587.0, 380)];

fn play(tones: Tones) {
    let (stream, handle) = match rodio::OutputStream::try_default() {
        Ok(output) => output,
        Err(error) => {
            report(&error.to_string());
            return;
        }
    };
    let sink = match rodio::Sink::try_new(&handle) {
        Ok(sink) => sink,
        Err(error) => {
            report(&error.to_string());
            return;
        }
    };

    for (frequency, millis) in tones {
        sink.append(
            SineWave::new(*frequency)
                .take_duration(Duration::from_millis(*millis))
                .amplify(0.15),
        );
    }
    sink.sleep_until_end();
    drop(stream);
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Best-effort means callable anywhere: with no output device and no notification
    /// daemon this must return quietly, and with them it costs one short blip.
    #[test]
    fn every_announcement_survives_a_headless_machine() {
        announce_task("test");
        announce_day_over();
    }
}
