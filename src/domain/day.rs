//! What happened on one calendar day.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

use crate::domain::task::TaskId;

/// The focus time collected on a single day, plus when that day began and ended.
///
/// Start and end are derived from activity, never entered by hand: the day starts with
/// the first time credited to any task and ends with the last, or with the moment the
/// user closed the day.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct DayRecord {
    pub started_at: Option<DateTime<Local>>,
    pub ended_at: Option<DateTime<Local>>,
    /// Whether the user has explicitly closed this day.
    pub closed: bool,
    pub per_task: BTreeMap<TaskId, Duration>,
    /// Tasks whose timer already sounded today, so it sounds only once.
    #[serde(default)]
    pub alarmed: BTreeSet<TaskId>,
    /// When the day alarm last sounded, driving both "once" and the ten-minute repeat.
    #[serde(default)]
    pub day_alarmed: Option<DateTime<Local>>,
    /// Whether the repeating day reminder has been muted for this day. A fresh record —
    /// tomorrow's — starts unmuted, which is what makes the reminder come back daily.
    #[serde(default)]
    pub nag_muted: bool,
}

impl DayRecord {
    pub fn duration_of(&self, task: TaskId) -> Duration {
        self.per_task.get(&task).copied().unwrap_or(Duration::ZERO)
    }

    pub fn total(&self) -> Duration {
        self.per_task.values().sum()
    }

    /// Credits `task` with `amount` collected between `from` and `to`.
    pub fn credit(
        &mut self,
        task: TaskId,
        amount: Duration,
        from: DateTime<Local>,
        to: DateTime<Local>,
    ) {
        if amount.is_zero() {
            return;
        }
        *self.per_task.entry(task).or_default() += amount;
        self.started_at = Some(match self.started_at {
            Some(existing) => existing.min(from),
            None => from,
        });
        self.ended_at = Some(match self.ended_at {
            Some(existing) => existing.max(to),
            None => to,
        });
    }
}
