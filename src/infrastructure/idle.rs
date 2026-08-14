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
        // Asking with no X server at all is not an error in the library underneath but a
        // null-pointer dereference: it opens the display and uses it unchecked. On Linux
        // the socket is probed first, which keeps a headless machine — and the native
        // Wayland path — answering `None` instead of crashing the bar.
        if cfg!(target_os = "linux") && !crate::app::x11_is_reachable() {
            return None;
        }
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
