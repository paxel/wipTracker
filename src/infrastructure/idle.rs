//! How long the user has been away, asked of the operating system.
//!
//! Best-effort like the beeper: a platform that will not say simply reports nothing,
//! and auto-pause quietly never triggers.

use std::time::Duration;

use crate::domain::ports::IdleProbe;

/// The system's own idle clock — last input event anywhere, not just in this window.
pub struct SystemIdle;

impl IdleProbe for SystemIdle {
    fn idle(&self) -> Option<Duration> {
        user_idle::UserIdle::get_time()
            .ok()
            .map(|idle| idle.duration())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Callable anywhere: a session that will not say simply answers `None`.
    #[test]
    fn the_probe_answers_or_declines_but_never_panics() {
        let _ = SystemIdle.idle();
    }
}
