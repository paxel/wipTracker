#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use wiptracker::app::{WipTracker, prefers_decorations};
use wiptracker::domain::ports::Store as _;
use wiptracker::infrastructure::redb_store::RedbStore;
use wiptracker::theme;

const USAGE: &str = "\
WipTracker — a one-line always-on-top bar showing the task you are focused on.

Usage: wiptracker [--version] [--help]

The app has no command-line interface beyond these two flags: everything happens on the
bar itself. See https://github.com/paxel/wipTracker for what the clicks do.";

/// The taskbar icon: a small cat reading, drawn by `packaging/make_icon.py`.
///
/// It is embedded as raw RGBA rather than PNG so the app needs no image decoder; the
/// script writes exactly 64x64 pixels.
fn icon() -> egui::IconData {
    const SIDE: u32 = 64;
    let rgba = include_bytes!("../assets/icon.rgba").to_vec();
    debug_assert_eq!(rgba.len(), (SIDE * SIDE * 4) as usize);
    egui::IconData {
        rgba,
        width: SIDE,
        height: SIDE,
    }
}

fn main() -> eframe::Result<()> {
    if let Some(argument) = std::env::args().nth(1) {
        match argument.as_str() {
            "--version" | "-V" => {
                println!("wiptracker {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            other => {
                eprintln!("wiptracker: unknown argument {other:?}\n\n{USAGE}");
                std::process::exit(2);
            }
        }
    }

    // The store is opened before the window so the bar can be placed where it was last
    // seen, and so an unreadable database is reported instead of being written over.
    let path = RedbStore::default_path();
    let opened = RedbStore::open(&path).and_then(|store| {
        let snapshot = store.load()?;
        Ok((store, snapshot))
    });

    let decorated = opened
        .as_ref()
        .ok()
        .and_then(|(_, snapshot)| snapshot.as_ref())
        .and_then(|snapshot| snapshot.decorated)
        .unwrap_or_else(prefers_decorations);

    // The bar is narrower when it wears a window frame, because the grip that drags an
    // undecorated window is not needed then.
    let size = theme::bar_size(decorated);
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("WipTracker")
        .with_icon(icon())
        .with_app_id("wiptracker")
        .with_inner_size(size)
        .with_min_inner_size(size)
        .with_max_inner_size(size)
        .with_decorations(decorated)
        .with_always_on_top()
        .with_resizable(false);
    if let Ok((_, Some(snapshot))) = &opened
        && let Some((x, y)) = snapshot.window_pos
    {
        viewport = viewport.with_position(egui::pos2(x, y));
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "WipTracker",
        options,
        Box::new(move |cc| {
            let app = match opened {
                Ok((store, snapshot)) => WipTracker::start(cc, Box::new(store), snapshot),
                Err(error) => {
                    eprintln!("wiptracker: {path:?}: {error}");
                    WipTracker::broken(cc, &error)
                }
            };
            Ok(Box::new(app))
        }),
    )
}
