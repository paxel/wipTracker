//! The time a person types into the reminder and timer fields: any text, never a panic.
#![no_main]

use libfuzzer_sys::fuzz_target;
use wiptracker::ui::format::{clock_time, parse_clock};

fuzz_target!(|text: &str| {
    if let Some(time) = parse_clock(text) {
        // What it accepts, it writes back in a form it reads the same way.
        assert_eq!(parse_clock(&clock_time(time)), Some(time));
    }
});
