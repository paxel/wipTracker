//! Keeps the snapshot in an embedded redb database.
//!
//! Tasks live in their own table keyed by id, so a single task changing does not rewrite
//! the rest; the small pieces of state that describe the stack itself live in a second
//! table as JSON values.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use redb::{Database, ReadableTable as _, TableDefinition};

use crate::domain::day::DayRecord;
use crate::domain::ports::{Snapshot, Store, StoreError};
use crate::domain::task::{Task, TaskId};

const TASKS: TableDefinition<'_, u64, &str> = TableDefinition::new("tasks");
const META: TableDefinition<'_, &str, &str> = TableDefinition::new("meta");

const KEY_STACK: &str = "stack";
const KEY_HISTORY: &str = "history";
const KEY_NEXT_NUMBER: &str = "next_number";
const KEY_DEFAULT_TIMER: &str = "default_timer";
const KEY_SHOW_DURATION: &str = "show_duration";
const KEY_DECORATED: &str = "decorated";
const KEY_WINDOW_POS: &str = "window_pos";

pub struct RedbStore {
    database: Database,
}

impl RedbStore {
    /// Opens, and if necessary creates, the database at `path`.
    pub fn open(path: impl AsRef<Path>) -> Result<Self, StoreError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| StoreError::Open(error.to_string()))?;
        }
        let database =
            Database::create(path).map_err(|error| StoreError::Open(error.to_string()))?;
        Ok(Self { database })
    }

    /// The file the app uses when the user has not asked for anything else.
    pub fn default_path() -> PathBuf {
        data_dir().join("wiptracker.redb")
    }
}

impl Store for RedbStore {
    fn load(&self) -> Result<Option<Snapshot>, StoreError> {
        let transaction = self
            .database
            .begin_read()
            .map_err(|error| StoreError::Read(error.to_string()))?;

        let tasks_table = match transaction.open_table(TASKS) {
            Ok(table) => table,
            // A database that has never been written has no tables yet.
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(StoreError::Read(error.to_string())),
        };

        let mut tasks: BTreeMap<TaskId, Task> = BTreeMap::new();
        for entry in tasks_table
            .iter()
            .map_err(|error| StoreError::Read(error.to_string()))?
        {
            let (key, value) = entry.map_err(|error| StoreError::Read(error.to_string()))?;
            let task: Task = serde_json::from_str(value.value())
                .map_err(|error| StoreError::Corrupt(error.to_string()))?;
            tasks.insert(key.value(), task);
        }

        let meta = match transaction.open_table(META) {
            Ok(table) => table,
            Err(redb::TableError::TableDoesNotExist(_)) => return Ok(None),
            Err(error) => return Err(StoreError::Read(error.to_string())),
        };
        let read_meta = |key: &str| -> Result<Option<String>, StoreError> {
            Ok(meta
                .get(key)
                .map_err(|error| StoreError::Read(error.to_string()))?
                .map(|value| value.value().to_owned()))
        };
        let parse = |raw: Option<String>| -> Result<Option<serde_json::Value>, StoreError> {
            raw.map(|text| {
                serde_json::from_str(&text).map_err(|error| StoreError::Corrupt(error.to_string()))
            })
            .transpose()
        };

        let stack = parse(read_meta(KEY_STACK)?)?
            .map(serde_json::from_value::<Vec<TaskId>>)
            .transpose()
            .map_err(|error| StoreError::Corrupt(error.to_string()))?
            .unwrap_or_default();
        let history = parse(read_meta(KEY_HISTORY)?)?
            .map(serde_json::from_value::<BTreeMap<chrono::NaiveDate, DayRecord>>)
            .transpose()
            .map_err(|error| StoreError::Corrupt(error.to_string()))?
            .unwrap_or_default();
        let next_number = parse(read_meta(KEY_NEXT_NUMBER)?)?
            .and_then(|value| value.as_u64())
            .unwrap_or(1);
        let default_timer = parse(read_meta(KEY_DEFAULT_TIMER)?)?
            .map(serde_json::from_value::<std::time::Duration>)
            .transpose()
            .map_err(|error| StoreError::Corrupt(error.to_string()))?
            .unwrap_or_default();
        let show_duration = parse(read_meta(KEY_SHOW_DURATION)?)?
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
        let decorated = parse(read_meta(KEY_DECORATED)?)?
            .map(serde_json::from_value::<Option<bool>>)
            .transpose()
            .map_err(|error| StoreError::Corrupt(error.to_string()))?
            .flatten();
        let window_pos = parse(read_meta(KEY_WINDOW_POS)?)?
            .map(serde_json::from_value::<Option<(f32, f32)>>)
            .transpose()
            .map_err(|error| StoreError::Corrupt(error.to_string()))?
            .flatten();

        Ok(Some(Snapshot {
            tasks,
            stack,
            history,
            next_number,
            default_timer,
            show_duration,
            decorated,
            window_pos,
        }))
    }

    fn save(&self, snapshot: &Snapshot) -> Result<(), StoreError> {
        let transaction = self
            .database
            .begin_write()
            .map_err(|error| StoreError::Write(error.to_string()))?;
        {
            let mut tasks = transaction
                .open_table(TASKS)
                .map_err(|error| StoreError::Write(error.to_string()))?;
            tasks
                .retain(|id, _| snapshot.tasks.contains_key(&id))
                .map_err(|error| StoreError::Write(error.to_string()))?;
            for (id, task) in &snapshot.tasks {
                let encoded = serde_json::to_string(task)
                    .map_err(|error| StoreError::Write(error.to_string()))?;
                tasks
                    .insert(*id, encoded.as_str())
                    .map_err(|error| StoreError::Write(error.to_string()))?;
            }

            let mut meta = transaction
                .open_table(META)
                .map_err(|error| StoreError::Write(error.to_string()))?;
            let mut put = |key: &str, value: &serde_json::Value| -> Result<(), StoreError> {
                let encoded = serde_json::to_string(value)
                    .map_err(|error| StoreError::Write(error.to_string()))?;
                meta.insert(key, encoded.as_str())
                    .map_err(|error| StoreError::Write(error.to_string()))?;
                Ok(())
            };
            put(KEY_STACK, &serde_json::json!(snapshot.stack))?;
            put(KEY_HISTORY, &serde_json::json!(snapshot.history))?;
            put(KEY_NEXT_NUMBER, &serde_json::json!(snapshot.next_number))?;
            put(
                KEY_DEFAULT_TIMER,
                &serde_json::json!(snapshot.default_timer),
            )?;
            put(
                KEY_SHOW_DURATION,
                &serde_json::json!(snapshot.show_duration),
            )?;
            put(KEY_DECORATED, &serde_json::json!(snapshot.decorated))?;
            put(KEY_WINDOW_POS, &serde_json::json!(snapshot.window_pos))?;
        }
        transaction
            .commit()
            .map_err(|error| StoreError::Write(error.to_string()))
    }
}

/// The per-platform directory the database lives in.
///
/// Linux and the BSDs follow the XDG base directory spec, macOS uses
/// `~/Library/Application Support`, Windows uses `%APPDATA%`.
fn data_dir() -> PathBuf {
    let home = std::env::var_os("HOME")
        .or_else(|| std::env::var_os("USERPROFILE"))
        .map(PathBuf::from);

    if cfg!(target_os = "macos") {
        if let Some(home) = home {
            return home
                .join("Library")
                .join("Application Support")
                .join("WipTracker");
        }
    } else if cfg!(target_os = "windows") {
        if let Some(appdata) = std::env::var_os("APPDATA") {
            return PathBuf::from(appdata).join("WipTracker").join("data");
        }
    } else {
        if let Some(xdg) = std::env::var_os("XDG_DATA_HOME").map(PathBuf::from)
            && xdg.is_absolute()
        {
            return xdg.join("wiptracker");
        }
        if let Some(home) = home {
            return home.join(".local").join("share").join("wiptracker");
        }
    }
    PathBuf::from(".")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::task::PAUSE_ID;
    use chrono::{Local, TimeZone as _};
    use std::time::Duration;

    fn snapshot() -> Snapshot {
        let created = Local
            .with_ymd_and_hms(2026, 8, 12, 9, 0, 0)
            .single()
            .expect("valid local time");
        let mut tasks = BTreeMap::new();
        tasks.insert(PAUSE_ID, Task::pause(created));
        let mut task = Task::new(1, "write the report", created);
        task.total = Duration::from_secs(1800);
        tasks.insert(1, task);

        let mut record = DayRecord::default();
        record.credit(
            1,
            Duration::from_secs(1800),
            created,
            created + chrono::TimeDelta::minutes(30),
        );
        let mut history = BTreeMap::new();
        history.insert(created.date_naive(), record);

        Snapshot {
            tasks,
            stack: vec![PAUSE_ID, 1],
            history,
            next_number: 2,
            default_timer: Duration::from_secs(3600),
            show_duration: false,
            decorated: Some(true),
            window_pos: Some((120.0, 40.0)),
        }
    }

    #[test]
    fn a_fresh_database_holds_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = RedbStore::open(dir.path().join("state.redb")).expect("open");
        assert_eq!(store.load().expect("load"), None);
    }

    #[test]
    fn a_saved_snapshot_reads_back_unchanged() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("state.redb");
        let original = snapshot();
        {
            let store = RedbStore::open(&path).expect("open");
            store.save(&original).expect("save");
        }
        let store = RedbStore::open(&path).expect("reopen");
        assert_eq!(store.load().expect("load"), Some(original));
    }

    #[test]
    fn removed_tasks_do_not_linger() {
        let dir = tempfile::tempdir().expect("temp dir");
        let store = RedbStore::open(dir.path().join("state.redb")).expect("open");
        store.save(&snapshot()).expect("save");

        let mut trimmed = snapshot();
        trimmed.tasks.remove(&1);
        trimmed.stack = vec![PAUSE_ID];
        store.save(&trimmed).expect("save again");

        assert_eq!(store.load().expect("load"), Some(trimmed));
    }

    #[test]
    fn a_file_that_is_not_a_database_is_reported() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("state.redb");
        std::fs::write(&path, b"this is not a database").expect("write");
        assert!(RedbStore::open(&path).is_err());
    }
}
