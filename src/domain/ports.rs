//! What the domain needs from the outside world.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use chrono::NaiveDate;

use crate::domain::day::DayRecord;
use crate::domain::task::{Task, TaskId};

/// Everything worth surviving a restart.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Snapshot {
    pub tasks: BTreeMap<TaskId, Task>,
    /// Bottom of the stack first.
    pub stack: Vec<TaskId>,
    /// What was worked on, day by day.
    pub history: BTreeMap<NaiveDate, DayRecord>,
    pub next_number: u64,
    /// The daily timer new tasks start with. Zero means no alarm.
    #[serde(default)]
    pub default_timer: std::time::Duration,
    pub show_duration: bool,
    /// Whether the window wears its window manager's frame. `None` means the user has not
    /// chosen, so the platform default applies.
    pub decorated: Option<bool>,
    /// Window position in logical points, as last seen.
    pub window_pos: Option<(f32, f32)>,
}

#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("the store could not be opened: {0}")]
    Open(String),
    #[error("the store could not be read: {0}")]
    Read(String),
    #[error("the store could not be written: {0}")]
    Write(String),
    #[error("the stored data is not readable: {0}")]
    Corrupt(String),
}

/// Something that can make a noise when a task's daily timer runs out.
pub trait Alarm: Send + Sync {
    fn sound(&self);
}

/// Somewhere a [`Snapshot`] can be kept.
pub trait Store {
    /// Reads the stored snapshot, or `None` on a first run.
    fn load(&self) -> Result<Option<Snapshot>, StoreError>;

    fn save(&self, snapshot: &Snapshot) -> Result<(), StoreError>;
}
