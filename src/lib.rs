// Denied rather than forbidden: two platform modules need a single documented `unsafe` each
// and say so with an `allow` of their own. Everything else stays safe.
#![deny(unsafe_code)]
// Shipped code may not panic: no unwrap, expect or panic, no unreachable, no
// indexing or slicing that can go out of bounds, no arithmetic that can overflow.
// Unit tests are exempt via `cfg(test)`; integration tests are separate crates.
#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::todo,
        clippy::unimplemented,
        clippy::unreachable,
        clippy::indexing_slicing,
        clippy::string_slice,
        clippy::arithmetic_side_effects
    )
)]

pub mod app;
pub mod domain;
pub mod infrastructure;
pub mod theme;
pub mod ui;
