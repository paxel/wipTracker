//! The sound a task's daily timer makes when it runs out.
//!
//! Audio is best-effort: a machine with no working output device should keep tracking
//! time rather than fail, so every error here is swallowed after being reported once.

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
    /// Returns immediately: the tones are played on a scratch thread.
    fn sound(&self) {
        std::thread::spawn(|| play(TASK_TONES));
    }

    /// Returns immediately, like [`Self::sound`]. Three falling tones where the task
    /// alarm rises — the day being over should not sound like one more task.
    fn sound_day_over(&self) {
        std::thread::spawn(|| play(DAY_TONES));
    }
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
