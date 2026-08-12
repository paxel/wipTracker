//! The task itself: what it is called and how much time it has collected in total.
//!
//! Per-day numbers are not kept here but in the tracker's day history, so that closing a
//! day cannot take them away.

use std::time::Duration;

use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};

pub type TaskId = u64;

/// The built-in break task. It is always present, can never be finished or renamed, and
/// is focused whenever no other task is open.
pub const PAUSE_ID: TaskId = 0;
pub const PAUSE_NAME: &str = "pause";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Task {
    pub id: TaskId,
    pub name: String,
    pub created_at: DateTime<Local>,
    /// Set when the task was finished; cleared again when it is revived.
    pub finished_at: Option<DateTime<Local>>,
    /// Focus time collected over the task's whole life.
    pub total: Duration,
}

impl Task {
    pub fn new(id: TaskId, name: impl Into<String>, created_at: DateTime<Local>) -> Self {
        Self {
            id,
            name: name.into(),
            created_at,
            finished_at: None,
            total: Duration::ZERO,
        }
    }

    pub fn pause(created_at: DateTime<Local>) -> Self {
        Self::new(PAUSE_ID, PAUSE_NAME, created_at)
    }

    pub fn is_pause(&self) -> bool {
        self.id == PAUSE_ID
    }

    pub fn is_finished(&self) -> bool {
        self.finished_at.is_some()
    }
}
