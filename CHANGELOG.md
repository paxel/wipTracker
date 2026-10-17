# Changelog

All notable changes to WipTracker are documented in this file.

## [1.3.0]

### Added

- **A pause reminder.** The timers window takes a clock time, and at that time, once a
  day, a small window asks whether it is time for a pause. _Pause now_ starts the pause
  from that moment; _Not now_ asks again a quarter of an hour later, until the pause is
  running. A pause already running at that time counts as the answer, and a reminder
  more than half an hour in the past when the app wakes up is missed for the day, not
  made up.

- **End earlier.** When the machine failed to shut down and the task ran through the
  night — or the whole start of a vacation — menu → _end earlier_ asks when you really
  stopped and takes everything after that off the task: the day of the cut ends there
  and is closed, days in between that held nothing else disappear, and the task keeps
  running from now. WipTracker offers it by itself on the first click or keystroke after
  a task ran through midnight, once per span. The same window fixes a forgotten pause on
  the same day.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
