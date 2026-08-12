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
        std::thread::spawn(play);
    }
}

fn play() {
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

    let beep = |frequency: f32, millis: u64| {
        SineWave::new(frequency)
            .take_duration(Duration::from_millis(millis))
            .amplify(0.15)
    };
    sink.append(beep(880.0, 160));
    sink.append(beep(1320.0, 220));
    sink.sleep_until_end();
    drop(stream);
}
