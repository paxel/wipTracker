//! The task stack and its focus-time accounting.
//!
//! Everything here is pure: the current time is always passed in, so the rules can be
//! tested without waiting for a clock.

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, TimeZone as _};

use crate::domain::day::DayRecord;
use crate::domain::ports::Snapshot;
use crate::domain::task::{PAUSE_ID, PAUSE_NAME, Task, TaskId};

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum TrackerError {
    #[error("the pause task cannot be renamed")]
    PauseIsUnrenameable,
    #[error("a task name must not be empty")]
    EmptyName,
    #[error("no task with id {0}")]
    UnknownTask(TaskId),
    #[error("task {0} is finished")]
    TaskIsFinished(TaskId),
    #[error("task {0} is not finished")]
    TaskIsNotFinished(TaskId),
}

pub struct Tracker {
    tasks: BTreeMap<TaskId, Task>,
    /// Bottom to top; the last entry is the focused task. Never empty.
    stack: Vec<TaskId>,
    /// What was worked on, day by day.
    history: BTreeMap<NaiveDate, DayRecord>,
    next_id: TaskId,
    /// The number used for the next automatically named task. Never reused.
    next_number: u64,
    /// When the currently focused task started collecting time.
    active_since: DateTime<Local>,
}

impl Tracker {
    /// A tracker holding nothing but the pause task.
    pub fn new(now: DateTime<Local>) -> Self {
        let mut tasks = BTreeMap::new();
        tasks.insert(PAUSE_ID, Task::pause(now));
        Self {
            tasks,
            stack: vec![PAUSE_ID],
            history: BTreeMap::new(),
            next_id: PAUSE_ID + 1,
            next_number: 1,
            active_since: now,
        }
    }

    /// Rebuilds a tracker from stored parts. Anything inconsistent is repaired rather
    /// than rejected, so a half-written stack cannot lock the user out of their history.
    pub fn from_snapshot(snapshot: &Snapshot, now: DateTime<Local>) -> Self {
        let mut tracker = Self {
            next_id: snapshot.tasks.keys().copied().max().unwrap_or(PAUSE_ID) + 1,
            tasks: snapshot.tasks.clone(),
            stack: snapshot.stack.clone(),
            history: snapshot.history.clone(),
            next_number: snapshot.next_number.max(1),
            active_since: now,
        };
        tracker
            .tasks
            .entry(PAUSE_ID)
            .or_insert_with(|| Task::pause(now));
        let tasks = &tracker.tasks;
        tracker
            .stack
            .retain(|id| tasks.get(id).is_some_and(|task| !task.is_finished()));
        tracker.stack.dedup();
        if !tracker.stack.contains(&PAUSE_ID) {
            tracker.stack.insert(0, PAUSE_ID);
        }
        tracker
    }

    /// Everything worth storing.
    pub fn snapshot(
        &self,
        show_duration: bool,
        decorated: Option<bool>,
        window_pos: Option<(f32, f32)>,
    ) -> Snapshot {
        Snapshot {
            tasks: self.tasks.clone(),
            stack: self.stack.clone(),
            history: self.history.clone(),
            next_number: self.next_number,
            show_duration,
            decorated,
            window_pos,
        }
    }

    pub fn focused_id(&self) -> TaskId {
        self.stack.last().copied().unwrap_or(PAUSE_ID)
    }

    pub fn focused(&self) -> Option<&Task> {
        self.task(self.focused_id())
    }

    /// The name to show in the bar. Falls back to the pause task's name, which is what an
    /// empty stack means.
    pub fn focused_name(&self) -> &str {
        self.focused().map_or(PAUSE_NAME, |task| task.name.as_str())
    }

    pub fn task(&self, id: TaskId) -> Option<&Task> {
        self.tasks.get(&id)
    }

    /// The open tasks, top of stack first, with the pause task last.
    pub fn open_tasks_top_first(&self) -> Vec<&Task> {
        let mut ordered: Vec<&Task> = self
            .stack
            .iter()
            .rev()
            .filter_map(|id| self.tasks.get(id))
            .collect();
        ordered.sort_by_key(|task| task.is_pause());
        ordered
    }

    /// The finished tasks, most recently finished first.
    pub fn finished_tasks(&self) -> Vec<&Task> {
        let mut finished: Vec<&Task> = self
            .tasks
            .values()
            .filter(|task| task.is_finished())
            .collect();
        finished.sort_by_key(|task| std::cmp::Reverse(task.finished_at));
        finished
    }

    pub fn next_task_number(&self) -> u64 {
        self.next_number
    }

    pub fn stack_bottom_first(&self) -> &[TaskId] {
        &self.stack
    }

    pub fn day(&self, day: NaiveDate) -> Option<&DayRecord> {
        self.history.get(&day)
    }

    pub fn history(&self) -> &BTreeMap<NaiveDate, DayRecord> {
        &self.history
    }

    /// How long `task` was focused on `day`.
    pub fn duration_on(&self, task: TaskId, day: NaiveDate) -> Duration {
        self.history
            .get(&day)
            .map_or(Duration::ZERO, |record| record.duration_of(task))
    }

    /// Credits the focused task with the time since the last accrual and resets the mark.
    ///
    /// Time spanning midnight is split so that each calendar day is credited separately.
    pub fn accrue(&mut self, now: DateTime<Local>) {
        let from = self.active_since;
        self.active_since = now;
        if now <= from {
            return;
        }
        let focused = self.focused_id();
        let mut credited = Duration::ZERO;
        for (day, start, end) in split_by_day(from, now) {
            let Ok(amount) = (end - start).to_std() else {
                continue;
            };
            self.history
                .entry(day)
                .or_default()
                .credit(focused, amount, start, end);
            credited += amount;
        }
        if let Some(task) = self.tasks.get_mut(&focused) {
            task.total += credited;
        }
    }

    /// Creates `new task N` and focuses it.
    pub fn push_new_task(&mut self, now: DateTime<Local>) -> TaskId {
        self.accrue(now);
        let id = self.next_id;
        self.next_id += 1;
        let name = format!("new task {}", self.next_number);
        self.next_number += 1;
        self.tasks.insert(id, Task::new(id, name, now));
        self.stack.push(id);
        id
    }

    /// Finishes the focused task, or ends a break if the pause task is focused.
    ///
    /// Returns the id of the task that was finished, if any.
    pub fn finish_focused(&mut self, now: DateTime<Local>) -> Option<TaskId> {
        self.accrue(now);
        let id = self.focused_id();
        if id == PAUSE_ID {
            self.end_break();
            return None;
        }
        self.stack.pop();
        if let Some(task) = self.tasks.get_mut(&id) {
            task.finished_at = Some(now);
        }
        self.ensure_not_empty();
        Some(id)
    }

    /// Finishes several tasks at once, wherever they sit on the stack.
    ///
    /// The pause task is silently skipped; it cannot be finished.
    pub fn finish_all(&mut self, ids: &[TaskId], now: DateTime<Local>) {
        self.accrue(now);
        for id in ids {
            if *id == PAUSE_ID {
                continue;
            }
            if let Some(task) = self.tasks.get_mut(id)
                && !task.is_finished()
            {
                task.finished_at = Some(now);
                self.stack.retain(|stacked| stacked != id);
            }
        }
        self.ensure_not_empty();
    }

    /// Moves an open task to the top of the stack.
    pub fn select(&mut self, id: TaskId, now: DateTime<Local>) -> Result<(), TrackerError> {
        let task = self.tasks.get(&id).ok_or(TrackerError::UnknownTask(id))?;
        if task.is_finished() {
            return Err(TrackerError::TaskIsFinished(id));
        }
        self.accrue(now);
        self.stack.retain(|stacked| *stacked != id);
        self.stack.push(id);
        Ok(())
    }

    /// Puts a finished task back on top of the stack, keeping its collected time.
    pub fn revive(&mut self, id: TaskId, now: DateTime<Local>) -> Result<(), TrackerError> {
        self.accrue(now);
        let task = self
            .tasks
            .get_mut(&id)
            .ok_or(TrackerError::UnknownTask(id))?;
        if !task.is_finished() {
            return Err(TrackerError::TaskIsNotFinished(id));
        }
        task.finished_at = None;
        self.stack.retain(|stacked| *stacked != id);
        self.stack.push(id);
        Ok(())
    }

    pub fn rename(&mut self, id: TaskId, name: &str) -> Result<(), TrackerError> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(TrackerError::EmptyName);
        }
        if id == PAUSE_ID {
            return Err(TrackerError::PauseIsUnrenameable);
        }
        let task = self
            .tasks
            .get_mut(&id)
            .ok_or(TrackerError::UnknownTask(id))?;
        task.name = trimmed.to_owned();
        Ok(())
    }

    /// Ends the day: stamps its end time and marks it closed.
    ///
    /// The day's numbers stay in the history so the week overview can still show them;
    /// tomorrow simply starts a new day, and the open stack is untouched.
    pub fn close_day(&mut self, now: DateTime<Local>) {
        self.accrue(now);
        let record = self.history.entry(now.date_naive()).or_default();
        record.ended_at = Some(now);
        record.closed = true;
    }

    /// Tasks that collected time on `day`, longest first.
    pub fn tasks_active_on(&self, day: NaiveDate) -> Vec<(&Task, Duration)> {
        let Some(record) = self.history.get(&day) else {
            return Vec::new();
        };
        let mut active: Vec<(&Task, Duration)> = record
            .per_task
            .iter()
            .filter_map(|(id, duration)| self.tasks.get(id).map(|task| (task, *duration)))
            .collect();
        active.sort_by_key(|(_, duration)| std::cmp::Reverse(*duration));
        active
    }

    fn end_break(&mut self) {
        if self.stack.last() == Some(&PAUSE_ID) && self.stack.len() > 1 {
            self.stack.pop();
            self.stack.insert(0, PAUSE_ID);
        }
    }

    fn ensure_not_empty(&mut self) {
        if self.stack.is_empty() {
            self.stack.push(PAUSE_ID);
        }
    }
}

/// Splits the span `from..to` at midnight, one chunk per calendar day.
fn split_by_day(
    from: DateTime<Local>,
    to: DateTime<Local>,
) -> Vec<(NaiveDate, DateTime<Local>, DateTime<Local>)> {
    let mut chunks = Vec::new();
    let mut cursor = from;
    while cursor < to {
        let day = cursor.date_naive();
        let next_midnight = day
            .succ_opt()
            .and_then(|next| next.and_hms_opt(0, 0, 0))
            .and_then(|naive| Local.from_local_datetime(&naive).earliest());
        let chunk_end = match next_midnight {
            Some(midnight) if midnight < to => midnight,
            _ => to,
        };
        if chunk_end <= cursor {
            break;
        }
        chunks.push((day, cursor, chunk_end));
        cursor = chunk_end;
    }
    chunks
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(day: u32, hour: u32) -> DateTime<Local> {
        Local
            .with_ymd_and_hms(2026, 8, day, hour, 0, 0)
            .single()
            .expect("valid local time")
    }

    fn hours(count: u64) -> Duration {
        Duration::from_secs(count * 3600)
    }

    #[test]
    fn starts_focused_on_pause() {
        let tracker = Tracker::new(at(1, 9));
        assert_eq!(tracker.focused_id(), PAUSE_ID);
        assert_eq!(tracker.focused_name(), PAUSE_NAME);
    }

    #[test]
    fn push_creates_numbered_tasks_that_never_reuse_a_number() {
        let mut tracker = Tracker::new(at(1, 9));
        let first = tracker.push_new_task(at(1, 9));
        let second = tracker.push_new_task(at(1, 10));
        assert_eq!(
            tracker.task(first).map(|t| t.name.as_str()),
            Some("new task 1")
        );
        assert_eq!(
            tracker.task(second).map(|t| t.name.as_str()),
            Some("new task 2")
        );

        tracker.finish_focused(at(1, 11));
        tracker.finish_focused(at(1, 12));
        let third = tracker.push_new_task(at(1, 13));
        assert_eq!(
            tracker.task(third).map(|t| t.name.as_str()),
            Some("new task 3")
        );
    }

    #[test]
    fn finishing_focuses_the_task_underneath() {
        let mut tracker = Tracker::new(at(1, 9));
        let first = tracker.push_new_task(at(1, 9));
        let second = tracker.push_new_task(at(1, 10));
        assert_eq!(tracker.focused_id(), second);

        let finished = tracker.finish_focused(at(1, 11));
        assert_eq!(finished, Some(second));
        assert_eq!(tracker.focused_id(), first);
        assert!(tracker.task(second).is_some_and(Task::is_finished));
    }

    #[test]
    fn finishing_the_last_task_falls_back_to_pause() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.push_new_task(at(1, 9));
        tracker.finish_focused(at(1, 10));
        assert_eq!(tracker.focused_id(), PAUSE_ID);
    }

    #[test]
    fn pause_cannot_be_finished_and_ends_the_break_instead() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.select(PAUSE_ID, at(1, 10)).expect("select pause");
        assert_eq!(tracker.focused_id(), PAUSE_ID);

        let finished = tracker.finish_focused(at(1, 11));
        assert_eq!(finished, None);
        assert_eq!(tracker.focused_id(), task);
        assert!(tracker.task(PAUSE_ID).is_some_and(|t| !t.is_finished()));
    }

    #[test]
    fn breaks_collect_time_like_any_other_task() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.push_new_task(at(1, 9));
        tracker.select(PAUSE_ID, at(1, 12)).expect("select pause");
        tracker.finish_focused(at(1, 13));

        assert_eq!(tracker.task(PAUSE_ID).map(|t| t.total), Some(hours(1)));
        assert_eq!(
            tracker.duration_on(PAUSE_ID, at(1, 9).date_naive()),
            hours(1)
        );
    }

    #[test]
    fn pause_cannot_be_renamed() {
        let mut tracker = Tracker::new(at(1, 9));
        assert_eq!(
            tracker.rename(PAUSE_ID, "lunch"),
            Err(TrackerError::PauseIsUnrenameable)
        );
    }

    #[test]
    fn renaming_rejects_blank_names() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        assert_eq!(tracker.rename(task, "   "), Err(TrackerError::EmptyName));
        assert_eq!(
            tracker.task(task).map(|t| t.name.as_str()),
            Some("new task 1")
        );
        tracker
            .rename(task, "  write the report  ")
            .expect("rename");
        assert_eq!(
            tracker.task(task).map(|t| t.name.as_str()),
            Some("write the report")
        );
    }

    #[test]
    fn only_the_focused_task_collects_time() {
        let mut tracker = Tracker::new(at(1, 9));
        let first = tracker.push_new_task(at(1, 9));
        let second = tracker.push_new_task(at(1, 10));
        tracker.accrue(at(1, 11));

        assert_eq!(tracker.task(first).map(|t| t.total), Some(hours(1)));
        assert_eq!(tracker.task(second).map(|t| t.total), Some(hours(1)));
        assert_eq!(
            tracker.task(PAUSE_ID).map(|t| t.total),
            Some(Duration::ZERO)
        );
    }

    #[test]
    fn selecting_resumes_a_paused_task() {
        let mut tracker = Tracker::new(at(1, 9));
        let first = tracker.push_new_task(at(1, 9));
        tracker.push_new_task(at(1, 10));
        tracker.select(first, at(1, 11)).expect("select");
        tracker.accrue(at(1, 12));

        assert_eq!(tracker.task(first).map(|t| t.total), Some(hours(2)));
    }

    #[test]
    fn time_across_midnight_lands_on_both_days() {
        let mut tracker = Tracker::new(at(1, 23));
        let task = tracker.push_new_task(at(1, 23));
        tracker.accrue(at(2, 1));

        assert_eq!(tracker.duration_on(task, at(1, 23).date_naive()), hours(1));
        assert_eq!(tracker.duration_on(task, at(2, 1).date_naive()), hours(1));
        assert_eq!(tracker.task(task).map(|t| t.total), Some(hours(2)));
    }

    #[test]
    fn a_clock_jumping_backwards_credits_nothing() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 10));
        tracker.accrue(at(1, 9));
        assert_eq!(tracker.task(task).map(|t| t.total), Some(Duration::ZERO));
    }

    #[test]
    fn the_day_starts_and_ends_with_activity() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.push_new_task(at(1, 9));
        tracker.accrue(at(1, 17));

        let record = tracker.day(at(1, 9).date_naive()).expect("day record");
        assert_eq!(record.started_at, Some(at(1, 9)));
        assert_eq!(record.ended_at, Some(at(1, 17)));
        assert!(!record.closed);
        assert_eq!(record.total(), hours(8));
    }

    #[test]
    fn closing_the_day_keeps_its_numbers_and_the_stack() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.close_day(at(1, 10));

        let day = at(1, 9).date_naive();
        assert_eq!(tracker.duration_on(task, day), hours(1));
        assert_eq!(tracker.task(task).map(|t| t.total), Some(hours(1)));
        assert!(tracker.day(day).is_some_and(|record| record.closed));
        assert_eq!(tracker.focused_id(), task);
    }

    #[test]
    fn a_new_day_starts_from_zero() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.close_day(at(1, 17));

        let first = at(1, 9).date_naive();
        let second = at(2, 10).date_naive();
        assert_eq!(tracker.duration_on(task, first), hours(8));
        assert_eq!(tracker.duration_on(task, second), Duration::ZERO);

        // Working on past a closed day keeps crediting real time rather than losing it.
        tracker.accrue(at(2, 10));
        assert_eq!(tracker.duration_on(task, second), hours(10));
    }

    #[test]
    fn reviving_puts_a_finished_task_back_on_top() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.finish_focused(at(1, 10));
        assert_eq!(tracker.focused_id(), PAUSE_ID);

        tracker.revive(task, at(1, 11)).expect("revive");
        assert_eq!(tracker.focused_id(), task);
        tracker.accrue(at(1, 12));
        assert_eq!(tracker.task(task).map(|t| t.total), Some(hours(2)));
        assert!(tracker.task(task).is_some_and(|t| !t.is_finished()));
    }

    #[test]
    fn time_after_a_revival_lands_on_the_current_day() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.finish_focused(at(1, 10));
        tracker.revive(task, at(3, 9)).expect("revive");
        tracker.accrue(at(3, 10));

        assert_eq!(tracker.duration_on(task, at(1, 9).date_naive()), hours(1));
        assert_eq!(tracker.duration_on(task, at(3, 9).date_naive()), hours(1));
    }

    #[test]
    fn finish_all_skips_pause_and_keeps_the_rest_of_the_stack() {
        let mut tracker = Tracker::new(at(1, 9));
        let first = tracker.push_new_task(at(1, 9));
        let second = tracker.push_new_task(at(1, 10));
        let third = tracker.push_new_task(at(1, 11));

        tracker.finish_all(&[PAUSE_ID, first, third], at(1, 12));
        assert_eq!(tracker.focused_id(), second);
        assert!(tracker.task(PAUSE_ID).is_some_and(|t| !t.is_finished()));
        assert!(tracker.task(first).is_some_and(Task::is_finished));
        assert!(tracker.task(third).is_some_and(Task::is_finished));
    }

    #[test]
    fn open_tasks_are_listed_top_first_with_pause_last() {
        let mut tracker = Tracker::new(at(1, 9));
        let first = tracker.push_new_task(at(1, 9));
        let second = tracker.push_new_task(at(1, 10));

        let ids: Vec<TaskId> = tracker
            .open_tasks_top_first()
            .iter()
            .map(|task| task.id)
            .collect();
        assert_eq!(ids, vec![second, first, PAUSE_ID]);
    }

    #[test]
    fn a_snapshot_round_trip_keeps_the_state() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.accrue(at(1, 11));

        let snapshot = tracker.snapshot(false, None, Some((10.0, 20.0)));
        let restored = Tracker::from_snapshot(&snapshot, at(1, 11));

        assert_eq!(restored.focused_id(), task);
        assert_eq!(restored.task(task).map(|t| t.total), Some(hours(2)));
        assert_eq!(restored.next_task_number(), 2);
        assert_eq!(restored.duration_on(task, at(1, 9).date_naive()), hours(2));
    }

    #[test]
    fn restoring_drops_finished_tasks_from_the_stack_and_keeps_pause() {
        let mut tasks = BTreeMap::new();
        tasks.insert(PAUSE_ID, Task::pause(at(1, 9)));
        let mut done = Task::new(7, "done", at(1, 9));
        done.finished_at = Some(at(1, 10));
        tasks.insert(7, done);
        tasks.insert(8, Task::new(8, "open", at(1, 9)));

        let snapshot = Snapshot {
            tasks,
            stack: vec![7, 8],
            history: BTreeMap::new(),
            next_number: 9,
            show_duration: true,
            decorated: None,
            window_pos: None,
        };
        let tracker = Tracker::from_snapshot(&snapshot, at(1, 11));
        assert_eq!(tracker.stack_bottom_first(), [PAUSE_ID, 8]);
        assert_eq!(tracker.focused_id(), 8);
        assert_eq!(tracker.next_task_number(), 9);
    }

    #[test]
    fn split_by_day_handles_a_span_inside_one_day() {
        let from = at(1, 9);
        let to = at(1, 10);
        assert_eq!(split_by_day(from, to), vec![(from.date_naive(), from, to)]);
    }
}
