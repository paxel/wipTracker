# Changelog

All notable changes to WipTracker are documented in this file.

## [1.2.0] - Unreleased

### Changed

- **The pause is called _pause_ everywhere.** The menu says _end pause_ and _Pause_
  instead of _end break_ and _Take a break_; hints, tooltips and the report say pauses
  where they said breaks.

### Added

- **An event journal.** `journal.log` next to the database records every start with
  what was recovered, every change of the focused task with both names, every
  auto-pause with the idle span it took back, the day being closed, and every exit
  with the day's worked time. A day whose numbers look wrong can now be read up
  instead of guessed at.

- **The end-day and the week window are easier to read.** The buttons have a border
  and stand apart from the table, the day's worked time is the headline, durations line
  up on the digits under a header row, and the week names its dates and marks today's
  column.

### Fixed

- **Time keeps counting while the bar is out of sight.** On macOS the bar stopped
  counting on every other Space, behind a fullscreen app, while the display slept and
  behind the lock screen, and the whole span was dropped once it was back; on Linux a
  window covering the bar did the same. A day with a quarter of it in fullscreen
  reported six hours for eight.
- **A sleeping machine no longer costs the task its time.** Any silence of more than
  two minutes, a closed lid included, used to be dropped from the focused task. The
  task stays what you are working on until you switch or pause, so the whole span
  counts now. A restart still recovers only up to four hours on the same day.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
