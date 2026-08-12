//! The shape WipTracker hands to other tools.
//!
//! One row per task per day, the same for a single day and for a whole week, so whatever
//! reads it needs one code path. Task ids are left out — they mean nothing outside this
//! app — and nothing is filtered: `pause` and finished tasks are rows like any other, and
//! the consumer decides what counts as billable.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use crate::domain::tracker::Tracker;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Row {
    /// The calendar day, as `YYYY-MM-DD`.
    pub date: NaiveDate,
    pub task: String,
    /// Focus time collected by that task on that day.
    pub seconds: u64,
}

/// The rows for `days`, longest first within each day.
pub fn rows(tracker: &Tracker, days: &[NaiveDate]) -> Vec<Row> {
    let mut rows = Vec::new();
    for day in days {
        for (task, duration) in tracker.tasks_active_on(*day) {
            rows.push(Row {
                date: *day,
                task: task.name.clone(),
                seconds: duration.as_secs(),
            });
        }
    }
    rows
}

/// The rows for `days` as pretty-printed JSON, ready for the clipboard.
pub fn to_json(tracker: &Tracker, days: &[NaiveDate]) -> String {
    let rows = rows(tracker, days);
    serde_json::to_string_pretty(&rows).unwrap_or_else(|_| "[]".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Local, TimeZone as _};

    fn at(day: u32, hour: u32) -> chrono::DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 8, day, hour, 0, 0)
            .single()
            .expect("valid local time")
    }

    fn tracker() -> Tracker {
        let mut tracker = Tracker::new(at(10, 9));
        let task = tracker.push_new_task(at(10, 9));
        tracker.rename(task, "arbeit").expect("rename");
        tracker
            .select(crate::domain::task::PAUSE_ID, at(10, 12))
            .expect("select pause");
        tracker.accrue(at(10, 13));
        tracker
    }

    #[test]
    fn a_day_exports_one_row_per_task_including_pause() {
        let tracker = tracker();
        let rows = rows(&tracker, &[at(10, 9).date_naive()]);

        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].task, "arbeit");
        assert_eq!(rows[0].seconds, 3 * 3600);
        assert_eq!(rows[1].task, "pause");
        assert_eq!(rows[1].seconds, 3600);
        assert!(rows.iter().all(|row| row.date == at(10, 9).date_naive()));
    }

    #[test]
    fn a_day_without_work_exports_nothing() {
        let tracker = tracker();
        assert!(rows(&tracker, &[at(11, 9).date_naive()]).is_empty());
    }

    #[test]
    fn the_json_is_a_flat_array_of_rows() {
        let tracker = tracker();
        let json = to_json(&tracker, &[at(10, 9).date_naive()]);
        let parsed: Vec<Row> = serde_json::from_str(&json).expect("valid json");

        assert_eq!(parsed, rows(&tracker, &[at(10, 9).date_naive()]));
        assert!(json.contains("\"date\": \"2026-08-10\""));
        assert!(json.contains("\"task\": \"arbeit\""));
        assert!(json.contains("\"seconds\": 10800"));
    }
}
