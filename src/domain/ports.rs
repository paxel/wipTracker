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
    /// The timer for the whole day's work. Zero means no alarm.
    #[serde(default)]
    pub day_timer: std::time::Duration,
    /// When time was last credited, so a short gap while the app was closed can be
    /// recovered on the next start. `None` before the first save.
    #[serde(default)]
    pub last_seen: Option<chrono::DateTime<chrono::Local>>,
    pub show_duration: bool,
    /// Whether the window wears its window manager's frame. `None` means the user has not
    /// chosen, so the platform default applies.
    pub decorated: Option<bool>,
    /// Whether the bar takes a place in the taskbar. `None` means the user has not
    /// chosen, and the default is to show up there.
    #[serde(default)]
    pub taskbar: Option<bool>,
    /// Window position in logical points, as last seen.
    pub window_pos: Option<(f32, f32)>,
    /// Whether the offer to add WipTracker to the application menu was declined for good.
    #[serde(default)]
    pub launcher_offer_dismissed: bool,
    /// Auto-pause once the user has been idle this long. Zero means never — the default,
    /// because watching the user's input is opt-in.
    #[serde(default)]
    pub idle_pause: std::time::Duration,
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

/// Something that can announce a timer running out — with noise, a notification, or
/// whatever the platform offers.
pub trait Alarm: Send + Sync {
    /// `task`'s daily timer was reached.
    fn sound(&self, task: &str);
    /// The whole day's timer was reached — distinct from a task, so the two cannot be
    /// mistaken for each other.
    fn sound_day_over(&self);
}

/// Something that knows how long the user has been away from keyboard and mouse,
/// machine-wide. `None` where the platform will not say.
pub trait IdleProbe: Send + Sync {
    fn idle(&self) -> Option<std::time::Duration>;
}

/// Somewhere a [`Snapshot`] can be kept.
pub trait Store {
    /// Reads the stored snapshot, or `None` on a first run.
    fn load(&self) -> Result<Option<Snapshot>, StoreError>;

    fn save(&self, snapshot: &Snapshot) -> Result<(), StoreError>;
}
