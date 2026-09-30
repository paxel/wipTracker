# WipTracker

A one-line bar that shows the task the user is focused on right now and collects time only
for that task, so the day's numbers describe where attention went.

## Language

### Tasks

**Task**:
A named piece of work the user can focus on. It is open until finished and keeps its focus
time for its whole life.
_Avoid_: Ticket, item, job

**Task stack**:
The open tasks, ordered. The top one is the focused task; the pause task is always at the
bottom, so the stack is never empty.
_Avoid_: List, queue, backlog

**Focused task**:
The task on top of the stack. It is the only task that focus time is credited to.
_Avoid_: Active task, current task, running task

**Finished task**:
A task taken off the stack for good. It keeps its focus time and can be revived.
_Avoid_: Closed task, done task, completed task, archived task

**Revive**:
Putting a finished task back on top of the stack, focus time intact.
_Avoid_: Reopen, restore, unfinish

**Pause task**:
The built-in task that is focused whenever no other task is open. It can never be finished
or renamed, and its time does not count as worked time.
_Avoid_: Break, pause, idle task

### Time

**Focus time**:
The time a task was the focused task. It is the only time WipTracker records.
_Avoid_: Collected time, accrued time, tracked time, work time

**Credit**:
Adding focus time to a task for a span it was focused. Time spanning midnight is credited
to each calendar day separately.
_Avoid_: Collect, accrue, log, book

**Worked time**:
The focus time of a day with the pause task excluded. It is what the day timer measures
against, not the wall-clock span of the day.
_Avoid_: Day counter, day total, time worked

**Day**:
A calendar day's focus time, starting with the first credit and ending with the last or
with the moment the user closed it.
_Avoid_: Session, shift, workday

### Timers

**Timer**:
A limit on focus time: how much before the alarm sounds. Zero means no alarm.
_Avoid_: Alarm (for the limit), budget, quota, allowance

**Daily timer**:
A task's timer, measured against its focus time per day.
_Avoid_: Task timer, task limit

**Day timer**:
The timer for the whole day, measured against worked time.
_Avoid_: Daily timer (that is the per-task one), day limit

**Alarm**:
What sounds when a timer is reached. A task alarm and the day alarm are distinct, so the
two cannot be mistaken for each other.
_Avoid_: Timer (for the sound), notification, reminder
