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

### Fixed

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
