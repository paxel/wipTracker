//! Loading the store from a damaged file: an error, never a panic.
//!
//! The first byte picks the damage. Even: the rest is the whole file, which mostly tests
//! redb's own checks. Odd: the rest is split at NUL bytes into values for the tasks
//! table and the meta keys of a valid database, which reaches wipTracker's own JSON
//! decoding behind redb.
#![no_main]

use libfuzzer_sys::fuzz_target;
use redb::{Database, TableDefinition};
use wiptracker::domain::ports::Store as _;
use wiptracker::infrastructure::redb_store::RedbStore;

const TASKS: TableDefinition<'_, u64, &str> = TableDefinition::new("tasks");
const META: TableDefinition<'_, &str, &str> = TableDefinition::new("meta");
const KEYS: [&str; 17] = [
    "stack",
    "history",
    "next_number",
    "default_timer",
    "day_timer",
    "launcher_offer_dismissed",
    "idle_pause",
    "pause_reminder",
    "last_seen",
    "focused_since",
    "show_duration",
    "decorated",
    "taskbar",
    "hints",
    "window_pos",
    "light_mode",
    "hue_shift",
];

fn write_values(path: &std::path::Path, data: &[u8]) -> Option<()> {
    let database = Database::create(path).ok()?;
    let transaction = database.begin_write().ok()?;
    {
        let mut values = data.split(|byte| *byte == 0).map(String::from_utf8_lossy);
        let mut tasks = transaction.open_table(TASKS).ok()?;
        if let Some(task) = values.next() {
            tasks.insert(1, task.as_ref()).ok()?;
        }
        let mut meta = transaction.open_table(META).ok()?;
        for (key, value) in KEYS.iter().zip(values) {
            meta.insert(*key, value.as_ref()).ok()?;
        }
    }
    transaction.commit().ok()
}

fuzz_target!(|data: &[u8]| {
    let Some((mode, rest)) = data.split_first() else {
        return;
    };
    let Ok(dir) = tempfile::tempdir() else {
        return;
    };
    let path = dir.path().join("wiptracker.redb");
    if mode % 2 == 0 {
        if std::fs::write(&path, rest).is_err() {
            return;
        }
    } else if write_values(&path, rest).is_none() {
        return;
    }
    if let Ok(store) = RedbStore::open(&path) {
        let _ = store.load();
    }
});
