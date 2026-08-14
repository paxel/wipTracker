# Changelog

All notable changes to WipTracker are documented in this file.

## [0.6.0]

### Added

- **WipTracker offers to put itself into the application menu.** On Linux, when no menu
  on the machine can see a WipTracker entry — which is every Homebrew install, since no
  desktop looks inside Homebrew's directories — a one-time window offers to write the
  launcher entry and icons into `~/.local/share`, where every desktop looks. Decline and
  it never asks again; `--install-launcher` and `--remove-launcher` do the same from the
  command line. Uninstalling stays clean without any hook: the entry names the binary in
  `TryExec`, and menus hide an entry whose binary is gone.

- **A timer for the whole day.** The timer window gained a _the whole day_ row above the
  tasks: when everything worked today — breaks not counted — reaches it, WipTracker plays
  a noise of its own (three falling tones, unlike the task alarm's two rising ones) and
  the bar clock turns red for the rest of the day, outranking the amber of a task timer.
  The end-day report shows the counter behind it: _Worked X of Y_, red once over.
- **A ten-minute reminder once the day is over.** The day alarm repeats every ten minutes
  until _mute day reminder_ in the menu silences it — for that day only; tomorrow it
  reminds again on its own.

- **Auto-pause when idle, strictly opt-in.** A new _auto-pause when idle_ row in the
  timer window (off unless chosen) starts the break by itself once keyboard and mouse
  have been quiet that long — and takes the quiet minutes off the task, so the break is
  counted from when the input stopped, not from when it was noticed. Off means WipTracker
  never looks at your input at all.
- **Timers announce themselves as desktop notifications** as well as beeps, so a muted
  machine still hears about them. The task alarm names the task; the day alarm says how
  to mute the reminder.
- **_start with my session_ in the menu.** One toggle writes or removes the autostart
  entry (Linux), a Startup-folder script (Windows), or a login item (macOS) — no more
  documented one-liners per platform.

### Fixed

- **Suspend no longer credits the night to the focused task.** With the app left running,
  closing the lid meant the whole sleep was booked onto whatever was on top, because the
  once-a-second frames stopped and the next frame credited the entire silence. A hole in
  the frame stream longer than two minutes is now skipped: the time passed, nobody worked
  it. Deliberately separate from the four-hour restart rule, which still credits a short
  accidental close.
- **A touchpad can drag the bar again.** The grip waited for the press to be recognised
  as "definitely not a click" before handing the window to the window manager, and the
  gesture rework had closed the time-based half of that recognition — the half a slow,
  short-travel touchpad press depends on. A mouse flick still qualified by distance, which
  is why only touchpads were dead. The grip now starts the move on the press itself, the
  way a title bar does; it means nothing but "move", so there was never anything to wait
  for.
- **The menu and the hint no longer cover a decorated bar.** Both were placed at the
  window's top plus the bar's own 32 pixel height — but a window with a frame is taller
  than the bar it contains, so on a mac they opened inside the window, covering everything
  but the title bar. They now place against the window's real bottom edge, and flipping
  above the bar clears the whole frame too.
- **Only the left button holds.** A two-second press of the right or the middle button on
  the task name used to finish the task, because the hold never asked which button was
  down. Clicks were already left-only; now the holds are as well.
- **The hint window is narrower than the bar**, so on a bar flush against the right screen
  edge it no longer sticks out past the edge and loses its last few words.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
