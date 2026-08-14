//! The task stack and its focus-time accounting.
//!
//! Everything here is pure: the current time is always passed in, so the rules can be
//! tested without waiting for a clock.

use std::collections::BTreeMap;
use std::time::Duration;

use chrono::{DateTime, Local, NaiveDate, TimeDelta, TimeZone as _};

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
    /// The daily timer every new task starts with. Zero means no alarm.
    default_timer: Duration,
    /// The timer for the whole day's work, pause excluded. Zero means no alarm.
    day_timer: Duration,
    /// Auto-pause once the user has been idle this long. Zero means never.
    idle_pause: Duration,
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
            default_timer: Duration::ZERO,
            day_timer: Duration::ZERO,
            idle_pause: Duration::ZERO,
            active_since: now,
        }
    }

    /// How long a gap while the app was closed may be and still count.
    ///
    /// Closing the app by mistake and reopening it shortly after should not cost the time
    /// in between; a genuinely long absence should not be credited to a task nobody was
    /// working on.
    pub const RECOVERABLE_GAP: TimeDelta = TimeDelta::hours(4);

    /// Rebuilds a tracker from stored parts. Anything inconsistent is repaired rather
    /// than rejected, so a half-written stack cannot lock the user out of their history.
    ///
    /// A short gap since the stored `last_seen` is credited to the task that was focused
    /// when the app closed: under [`Self::RECOVERABLE_GAP`], and only within the same
    /// calendar day, so nothing is ever credited across a night.
    pub fn from_snapshot(snapshot: &Snapshot, now: DateTime<Local>) -> Self {
        let resume_from = snapshot
            .last_seen
            .filter(|last| {
                *last <= now
                    && now - *last <= Self::RECOVERABLE_GAP
                    && last.date_naive() == now.date_naive()
            })
            .unwrap_or(now);

        let mut tracker = Self {
            next_id: snapshot.tasks.keys().copied().max().unwrap_or(PAUSE_ID) + 1,
            tasks: snapshot.tasks.clone(),
            stack: snapshot.stack.clone(),
            history: snapshot.history.clone(),
            next_number: snapshot.next_number.max(1),
            default_timer: snapshot.default_timer,
            day_timer: snapshot.day_timer,
            idle_pause: snapshot.idle_pause,
            active_since: resume_from,
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
            default_timer: self.default_timer,
            day_timer: self.day_timer,
            idle_pause: self.idle_pause,
            last_seen: Some(self.active_since),
            show_duration,
            decorated,
            window_pos,
            // An app preference, not the tracker's: the caller sets it on the way out.
            launcher_offer_dismissed: false,
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

    /// The tasks finished within the last `days`, most recently finished first.
    ///
    /// Older ones are still in the data and in the week overview; they are simply not
    /// offered for reviving, so the list stays readable after a year of use.
    pub fn recently_finished(&self, days: i64, now: DateTime<Local>) -> Vec<&Task> {
        let cutoff = now - chrono::TimeDelta::days(days);
        self.finished_tasks()
            .into_iter()
            .filter(|task| task.finished_at.is_some_and(|at| at >= cutoff))
            .collect()
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

    /// How far apart two frames may lie and still count as continuous work.
    ///
    /// Frames arrive about once a second while the app runs, so a much longer silence
    /// means the machine was suspended or frozen — nobody was working. Two minutes is far
    /// above any real stall, and short enough that a lid closed over lunch cannot credit
    /// the afternoon. Deliberately stricter than [`Self::RECOVERABLE_GAP`], which covers
    /// restarting the app on purpose; the caller watching the frames decides which of the
    /// two applies and uses [`Self::skip_to`] for a sleep.
    pub const CONTINUOUS_GAP: TimeDelta = TimeDelta::minutes(2);

    /// Moves the accrual mark to `now` without crediting anything.
    ///
    /// For the span the machine slept through: the time passed, but nobody worked it.
    pub fn skip_to(&mut self, now: DateTime<Local>) {
        self.active_since = self.active_since.max(now);
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
        self.tasks
            .insert(id, Task::new(id, name, now).with_timer(self.default_timer));
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

    pub fn default_timer(&self) -> Duration {
        self.default_timer
    }

    /// Sets the timer new tasks start with. Zero switches the alarm off.
    pub fn set_default_timer(&mut self, timer: Duration) {
        self.default_timer = timer;
    }

    /// Sets one task's daily timer. Zero switches its alarm off.
    pub fn set_timer(&mut self, id: TaskId, timer: Duration) -> Result<(), TrackerError> {
        let task = self
            .tasks
            .get_mut(&id)
            .ok_or(TrackerError::UnknownTask(id))?;
        task.timer = timer;
        Ok(())
    }

    /// How long was worked on `day`, the pause task excluded.
    ///
    /// This is the day counter: breaks do not count, and neither does time when the app
    /// was closed. It is not the wall-clock span of the day — that is what the day
    /// record's start and end are for.
    pub fn worked_on(&self, day: NaiveDate) -> Duration {
        self.history.get(&day).map_or(Duration::ZERO, |record| {
            record.total() - record.duration_of(PAUSE_ID)
        })
    }

    pub fn day_timer(&self) -> Duration {
        self.day_timer
    }

    pub fn idle_pause(&self) -> Duration {
        self.idle_pause
    }

    /// Sets how long the user may be idle before the break starts on its own. Zero
    /// switches auto-pause off — the default, since watching input is opt-in.
    pub fn set_idle_pause(&mut self, idle_pause: Duration) {
        self.idle_pause = idle_pause;
    }

    /// Starts the break because the user has been idle for `idle`, if that is wanted,
    /// long enough, and a task is focused at all. Returns whether the break began.
    ///
    /// The credited tail is taken back: the task stops counting from the moment the
    /// input stopped, not from the moment the threshold was noticed — otherwise every
    /// auto-pause would gift the task its own trigger delay.
    pub fn pause_after_idle(&mut self, idle: Duration, now: DateTime<Local>) -> bool {
        if self.idle_pause.is_zero() || idle < self.idle_pause {
            return false;
        }
        if self.focused_id() == PAUSE_ID {
            return false;
        }
        self.accrue(now);
        self.uncredit(idle, now);
        let _ = self.select(PAUSE_ID, now);
        true
    }

    /// Takes back up to `span` of what the focused task was credited today. Only today:
    /// an idle span is minutes, not days.
    fn uncredit(&mut self, span: Duration, now: DateTime<Local>) {
        let focused = self.focused_id();
        let Some(record) = self.history.get_mut(&now.date_naive()) else {
            return;
        };
        let Some(credited) = record.per_task.get_mut(&focused) else {
            return;
        };
        let taken = span.min(*credited);
        *credited -= taken;
        if let Some(task) = self.tasks.get_mut(&focused) {
            task.total = task.total.saturating_sub(taken);
        }
    }

    /// Sets the timer for the whole day's work. Zero switches the alarm off.
    pub fn set_day_timer(&mut self, timer: Duration) {
        self.day_timer = timer;
    }

    /// Whether today's work has reached the day timer.
    pub fn day_over(&self, day: NaiveDate) -> bool {
        !self.day_timer.is_zero() && self.worked_on(day) >= self.day_timer
    }

    /// How long the reminder waits before sounding again, while it is not muted.
    pub const NAG_INTERVAL: TimeDelta = TimeDelta::minutes(10);

    /// Whether the day alarm should sound now. Reported once when the timer is reached,
    /// and then, as long as the reminder is not muted, once every [`Self::NAG_INTERVAL`].
    ///
    /// Call after [`Self::accrue`]; the caller is expected to make the noise.
    pub fn take_due_day_alarm(&mut self, now: DateTime<Local>) -> bool {
        if !self.day_over(now.date_naive()) {
            return false;
        }
        let Some(record) = self.history.get_mut(&now.date_naive()) else {
            return false;
        };
        let due = match record.day_alarmed {
            None => true,
            Some(last) => !record.nag_muted && now - last >= Self::NAG_INTERVAL,
        };
        if due {
            record.day_alarmed = Some(now);
        }
        due
    }

    /// Whether the repeating day reminder is muted for `day`. Tomorrow starts unmuted.
    pub fn nag_muted(&self, day: NaiveDate) -> bool {
        self.history
            .get(&day)
            .is_some_and(|record| record.nag_muted)
    }

    /// Mutes or unmutes the repeating day reminder, for this day only.
    pub fn set_nag_muted(&mut self, day: NaiveDate, muted: bool) {
        self.history.entry(day).or_default().nag_muted = muted;
    }

    /// The tasks whose daily timer has just been reached, each reported once per day.
    ///
    /// Call after [`Self::accrue`]; the caller is expected to sound the alarm.
    pub fn take_due_alarms(&mut self, now: DateTime<Local>) -> Vec<TaskId> {
        let day = now.date_naive();
        let Some(record) = self.history.get(&day) else {
            return Vec::new();
        };
        let due: Vec<TaskId> = record
            .per_task
            .iter()
            .filter(|(id, spent)| {
                !record.alarmed.contains(id)
                    && self
                        .tasks
                        .get(id)
                        .is_some_and(|task| task.has_timer() && **spent >= task.timer)
            })
            .map(|(id, _)| *id)
            .collect();
        if !due.is_empty()
            && let Some(record) = self.history.get_mut(&day)
        {
            record.alarmed.extend(due.iter().copied());
        }
        due
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
            default_timer: Duration::ZERO,
            day_timer: Duration::ZERO,
            last_seen: None,
            launcher_offer_dismissed: false,
            idle_pause: Duration::ZERO,
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
    fn a_short_gap_while_the_app_was_closed_is_credited() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.accrue(at(1, 10));
        let snapshot = tracker.snapshot(true, None, None);

        // Reopened an hour later on the same day: the hour counts.
        let resumed = Tracker::from_snapshot(&snapshot, at(1, 11));
        let mut resumed = resumed;
        resumed.accrue(at(1, 11));
        assert_eq!(resumed.task(task).map(|t| t.total), Some(hours(2)));
    }

    #[test]
    fn a_long_gap_is_not_credited() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.accrue(at(1, 10));
        let snapshot = tracker.snapshot(true, None, None);

        // Five hours later is past the threshold: only the first hour survives.
        let mut resumed = Tracker::from_snapshot(&snapshot, at(1, 15));
        resumed.accrue(at(1, 15));
        assert_eq!(resumed.task(task).map(|t| t.total), Some(hours(1)));
    }

    #[test]
    fn a_gap_across_midnight_is_not_credited() {
        let mut tracker = Tracker::new(at(1, 23));
        let task = tracker.push_new_task(at(1, 23));
        tracker.accrue(at(1, 23) + TimeDelta::minutes(30));
        let snapshot = tracker.snapshot(true, None, None);

        // Two hours later, but on the next day: nothing is recovered.
        let mut resumed = Tracker::from_snapshot(&snapshot, at(2, 1));
        resumed.accrue(at(2, 1));
        assert_eq!(
            resumed.task(task).map(|t| t.total),
            Some(Duration::from_secs(1800))
        );
    }

    #[test]
    fn a_recovered_gap_goes_to_whatever_was_focused_including_pause() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.push_new_task(at(1, 9));
        tracker.select(PAUSE_ID, at(1, 10)).expect("select pause");
        let snapshot = tracker.snapshot(true, None, None);

        let mut resumed = Tracker::from_snapshot(&snapshot, at(1, 11));
        resumed.accrue(at(1, 11));
        assert_eq!(resumed.task(PAUSE_ID).map(|t| t.total), Some(hours(1)));
    }

    #[test]
    fn revivable_tasks_stop_at_the_cutoff() {
        let mut tracker = Tracker::new(at(1, 9));
        let old = tracker.push_new_task(at(1, 9));
        tracker.finish_focused(at(1, 10));
        let recent = tracker.push_new_task(at(20, 9));
        tracker.finish_focused(at(20, 10));

        let ids: Vec<TaskId> = tracker
            .recently_finished(30, at(25, 9))
            .iter()
            .map(|task| task.id)
            .collect();
        assert_eq!(ids, vec![recent, old], "both are inside 30 days");

        let ids: Vec<TaskId> = tracker
            .recently_finished(30, at(1, 9) + chrono::TimeDelta::days(40))
            .iter()
            .map(|task| task.id)
            .collect();
        assert_eq!(ids, vec![recent], "the older one has aged out");
        assert_eq!(
            tracker.finished_tasks().len(),
            2,
            "nothing is deleted, only hidden from the revive list"
        );
    }

    #[test]
    fn new_tasks_inherit_the_default_timer() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.set_default_timer(Duration::from_secs(1800));
        let task = tracker.push_new_task(at(1, 9));
        assert_eq!(
            tracker.task(task).map(|t| t.timer),
            Some(Duration::from_secs(1800))
        );
    }

    #[test]
    fn a_timer_sounds_once_when_the_daily_limit_is_reached() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.set_timer(task, hours(2)).expect("set timer");

        tracker.accrue(at(1, 10));
        assert!(tracker.take_due_alarms(at(1, 10)).is_empty(), "not due yet");

        tracker.accrue(at(1, 11));
        assert_eq!(tracker.take_due_alarms(at(1, 11)), vec![task]);
        assert!(
            tracker.take_due_alarms(at(1, 11)).is_empty(),
            "the alarm sounds only once a day"
        );
    }

    #[test]
    fn the_same_task_can_sound_again_the_next_day() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.set_timer(task, hours(1)).expect("set timer");
        tracker.accrue(at(1, 11));
        assert_eq!(tracker.take_due_alarms(at(1, 11)), vec![task]);

        tracker.accrue(at(2, 11));
        assert_eq!(tracker.take_due_alarms(at(2, 11)), vec![task]);
    }

    #[test]
    fn a_zero_timer_never_sounds() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.accrue(at(1, 17));
        assert!(tracker.take_due_alarms(at(1, 17)).is_empty());
        assert_eq!(tracker.task(task).map(|t| t.timer), Some(Duration::ZERO));
    }

    #[test]
    fn split_by_day_handles_a_span_inside_one_day() {
        let from = at(1, 9);
        let to = at(1, 10);
        assert_eq!(split_by_day(from, to), vec![(from.date_naive(), from, to)]);
    }

    /// The day counter is work only: an hour on a task and an hour of pause is one hour.
    #[test]
    fn the_day_counter_excludes_the_pause() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.push_new_task(at(1, 9));
        tracker.accrue(at(1, 10));
        tracker.select(PAUSE_ID, at(1, 10)).expect("pause");
        tracker.accrue(at(1, 11));

        assert_eq!(tracker.worked_on(at(1, 9).date_naive()), hours(1));
    }

    #[test]
    fn the_day_alarm_sounds_once_when_the_day_timer_is_reached() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.set_day_timer(hours(2));
        tracker.push_new_task(at(1, 9));
        tracker.accrue(at(1, 10));
        assert!(
            !tracker.take_due_day_alarm(at(1, 10)),
            "one hour is not two"
        );

        tracker.accrue(at(1, 11));
        assert!(tracker.take_due_day_alarm(at(1, 11)));
        assert!(
            !tracker.take_due_day_alarm(at(1, 11)),
            "the same moment does not sound twice"
        );
        assert!(tracker.day_over(at(1, 11).date_naive()));
    }

    /// Once over, the reminder repeats every ten minutes until muted — and muting is only
    /// for that day.
    #[test]
    fn the_day_alarm_nags_every_ten_minutes_until_muted() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.set_day_timer(hours(1));
        tracker.push_new_task(at(1, 9));
        tracker.accrue(at(1, 10));
        assert!(tracker.take_due_day_alarm(at(1, 10)));

        let five_later = at(1, 10) + TimeDelta::minutes(5);
        tracker.accrue(five_later);
        assert!(
            !tracker.take_due_day_alarm(five_later),
            "five minutes is inside the interval"
        );

        let ten_later = at(1, 10) + TimeDelta::minutes(10);
        tracker.accrue(ten_later);
        assert!(tracker.take_due_day_alarm(ten_later), "ten minutes is due");

        tracker.set_nag_muted(ten_later.date_naive(), true);
        let twenty_later = at(1, 10) + TimeDelta::minutes(20);
        tracker.accrue(twenty_later);
        assert!(
            !tracker.take_due_day_alarm(twenty_later),
            "muted for the day"
        );

        // The next day starts unmuted: a fresh record carries no mute.
        assert!(!tracker.nag_muted(at(2, 9).date_naive()));
    }

    /// Without a day timer there is never a day alarm, whatever was worked.
    #[test]
    fn no_day_timer_means_no_day_alarm() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.push_new_task(at(1, 9));
        tracker.accrue(at(1, 19));
        assert!(!tracker.take_due_day_alarm(at(1, 19)));
        assert!(!tracker.day_over(at(1, 19).date_naive()));
    }

    /// The day timer survives a save and a restart.
    #[test]
    fn the_day_timer_is_persisted() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.set_day_timer(hours(8));
        let snapshot = tracker.snapshot(true, None, None);
        let restored = Tracker::from_snapshot(&snapshot, at(1, 10));
        assert_eq!(restored.day_timer(), hours(8));
    }

    /// A suspend is skipped, not credited: the mark moves, the numbers do not.
    #[test]
    fn a_skipped_span_credits_nothing() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.accrue(at(1, 10));
        tracker.skip_to(at(1, 18));
        tracker.accrue(at(1, 18) + TimeDelta::seconds(1));

        let worked = tracker.duration_on(task, at(1, 9).date_naive());
        assert_eq!(worked, hours(1) + Duration::from_secs(1));
    }

    /// Auto-pause is off until asked for, and then takes the idle tail off the task:
    /// the break starts when the input stopped, not when the threshold was noticed.
    #[test]
    fn idling_long_enough_starts_the_break_and_uncredits_the_tail() {
        let mut tracker = Tracker::new(at(1, 9));
        let task = tracker.push_new_task(at(1, 9));
        tracker.accrue(at(1, 10));

        let idle = Duration::from_secs(600);
        assert!(
            !tracker.pause_after_idle(idle, at(1, 10)),
            "off by default: watching input is opt-in"
        );

        tracker.set_idle_pause(Duration::from_secs(300));
        assert!(tracker.pause_after_idle(idle, at(1, 10)));
        assert_eq!(tracker.focused_id(), PAUSE_ID);
        assert_eq!(
            tracker.duration_on(task, at(1, 9).date_naive()),
            hours(1) - idle,
            "the ten idle minutes are taken back"
        );

        assert!(
            !tracker.pause_after_idle(idle, at(1, 11)),
            "already paused: nothing to do"
        );
    }

    #[test]
    fn a_short_idle_does_not_pause() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.push_new_task(at(1, 9));
        tracker.set_idle_pause(Duration::from_secs(600));
        assert!(!tracker.pause_after_idle(Duration::from_secs(60), at(1, 10)));
        assert_ne!(tracker.focused_id(), PAUSE_ID);
    }

    #[test]
    fn the_idle_pause_setting_is_persisted() {
        let mut tracker = Tracker::new(at(1, 9));
        tracker.set_idle_pause(Duration::from_secs(300));
        let snapshot = tracker.snapshot(true, None, None);
        let restored = Tracker::from_snapshot(&snapshot, at(1, 10));
        assert_eq!(restored.idle_pause(), Duration::from_secs(300));
    }
}
