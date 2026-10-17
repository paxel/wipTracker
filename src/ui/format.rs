//! Turning durations into the short strings the bar and the reports show.

use std::time::Duration;

use chrono::NaiveTime;

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

/// A clock time typed as `HH:MM`, with a single-digit hour allowed. `None` for anything
/// else, an empty string included.
pub fn parse_clock(text: &str) -> Option<NaiveTime> {
    NaiveTime::parse_from_str(text.trim(), "%H:%M").ok()
}

/// `HH:MM`, the way [`parse_clock`] reads it back.
pub fn clock_time(time: NaiveTime) -> String {
    time.format("%H:%M").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_clock_time_round_trips_and_forgives_a_short_hour() {
        let half_past = NaiveTime::from_hms_opt(12, 30, 0);
        assert_eq!(parse_clock("12:30"), half_past);
        assert_eq!(parse_clock(" 12:30 "), half_past);
        assert_eq!(parse_clock("9:05"), NaiveTime::from_hms_opt(9, 5, 0));
        assert_eq!(half_past.map(clock_time).as_deref(), Some("12:30"));
    }

    #[test]
    fn a_clock_time_rejects_what_is_not_one() {
        for text in ["", "noon", "25:00", "12:60", "12", "12:30:15"] {
            assert_eq!(parse_clock(text), None, "{text:?}");
        }
    }

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
