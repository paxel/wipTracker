//! Turning durations into the short strings the bar and the reports show.

use std::time::Duration;

/// `H:MM:SS` once an hour has passed, `M:SS` before that.
pub fn clock(duration: Duration) -> String {
    let total = duration.as_secs();
    let (hours, minutes, seconds) = (total / 3600, (total % 3600) / 60, total % 60);
    if hours > 0 {
        format!("{hours}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes}:{seconds:02}")
    }
}

/// `2h 05m` style, for reports where seconds are noise.
pub fn coarse(duration: Duration) -> String {
    let total = duration.as_secs();
    let (hours, minutes) = (total / 3600, (total % 3600) / 60);
    if hours > 0 {
        format!("{hours}h {minutes:02}m")
    } else {
        format!("{minutes}m")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_drops_the_hour_until_it_is_needed() {
        assert_eq!(clock(Duration::from_secs(0)), "0:00");
        assert_eq!(clock(Duration::from_secs(75)), "1:15");
        assert_eq!(clock(Duration::from_secs(3600)), "1:00:00");
        assert_eq!(clock(Duration::from_secs(3661)), "1:01:01");
    }

    #[test]
    fn coarse_rounds_down_to_minutes() {
        assert_eq!(coarse(Duration::from_secs(59)), "0m");
        assert_eq!(coarse(Duration::from_secs(3600 + 299)), "1h 04m");
    }
}
