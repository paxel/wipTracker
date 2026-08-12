#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use wiptracker::app::{WipTracker, prefers_decorations};
use wiptracker::domain::ports::Store as _;
use wiptracker::infrastructure::redb_store::RedbStore;
use wiptracker::theme;

fn main() -> eframe::Result<()> {
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

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("WipTracker")
        .with_app_id("wiptracker")
        .with_inner_size(theme::BAR_SIZE)
        .with_min_inner_size(theme::BAR_SIZE)
        .with_max_inner_size(theme::BAR_SIZE)
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
