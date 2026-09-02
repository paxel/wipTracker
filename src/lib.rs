// Denied rather than forbidden: two platform modules need a single documented `unsafe` each
// and say so with an `allow` of their own. Everything else stays safe.
#![deny(unsafe_code)]

pub mod app;
pub mod domain;
pub mod infrastructure;
pub mod theme;
pub mod ui;
