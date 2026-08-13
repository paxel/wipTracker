# Older changes

Released versions before the one in [CHANGELOG.md](CHANGELOG.md).

## [0.3.0]

### Added

- **JSON export.** The end-day and week windows have an _export_ button that copies the
  data to the clipboard as a flat array of rows — `{ "date", "task", "seconds" }`, one per
  task per day, identical for both windows — so another tool can book time from it.
  Nothing is filtered: `pause` and finished tasks are rows like any other.
- **Linux desktop integration.** `brew install paxel/tap/wiptracker` works on Linux as
  well as macOS and now installs the desktop entry and icon along with the binary. A `.deb`
  does the same for Debian-based systems, and the tarball carries the files plus
  `install-icon.sh` for everyone else. This is what makes a taskbar icon appear on Wayland,
  where a client cannot set its own window icon and the compositor matches the window's app
  id to a desktop entry instead.
- **A short gap while the app was closed is credited to the task that was focused.** Under
  four hours, and only within the same calendar day, so an accidental close no longer
  costs the work in between. Longer absences and anything crossing midnight are still
  never counted.

### Changed

- **Tooltips are anchored to the control they describe**, a gap away from it, and larger.
  They used to open under the mouse pointer, which covered the first words.
- **The report windows open near the bar**, or centred on the screen when the bar's
  position is unknown, instead of being left in the top-left corner.

### Fixed

- **Dragging the bar no longer leaves it stuck to the pointer.** The window was moved by
  two mechanisms at once: the window manager's own gesture, plus a per-frame fallback. Once
  the window manager takes a pointer grab the button release never reaches the app, so the
  fallback kept moving the window until the next click. Only the native gesture remains.

## [0.2.0]

### Added

- **Daily timers.** Menu → _timer_ gives each open task a daily limit, plus a default that
  new tasks inherit. When a task has been worked on that long today, WipTracker beeps once
  and its clock on the bar turns amber for the rest of the day. Zero means no alarm. Audio
  is best-effort: without an output device the timers still work, they just stay silent.
- **The task stack has its own window.** Menu → _select_, or middle-click the task name.
  Focused task first, the rest in stack order, `pause` last, each row showing today's time
  and the all-time total. Clicking a task works on it again.
- **Every control explains itself on hover**, including which mouse button does what.
- **`--version` and `--help`**, so package managers have something to test against.
- **An app icon** — a cat reading — in the taskbar, the dock and the macOS bundle.

### Changed

- **Left click is the primary action everywhere.** Left-clicking the task name renames it;
  the old right-click binding is gone. `+` keeps left for a new task, right to finish,
  middle for a break.
- **A new task opens its own name for editing**, with the `new task 7` placeholder
  selected, so the first keystroke replaces it.
- **The bar clock shows today's time**, not the all-time total, so the number agrees with
  the timers and the end-day report. The total is in the hover tooltip and the task stack.
- **The burger menu is a list of commands** — select, timer, groom, end day, week, revive
  and the two toggles — instead of also carrying the task list inline.
- **_revive_ lists the last 30 days** of finished tasks, so the list stays readable.
  Nothing is deleted; older tasks stay in the data and in the week view.
- **The window frame toggle takes effect at the next start.** Adding a frame to a running
  window drew it over the bar, so the preference is stored and the menu says so.
- **Releases are cut the way the other paxel apps are**: a tagged build gates on the full
  Linux suite, ships per-architecture macOS tarballs plus a universal `.dmg`, takes its
  release notes from this file, and pushes the Homebrew formula and Scoop manifest.

### Fixed

- **The desktop's light theme no longer leaks into the bar.** egui follows the system theme
  and re-applies it every frame, which on a light desktop turned the menu into white
  buttons with near-white labels and the rename field into white text on white. WipTracker
  now pins its own palette.
- **A stored window position on a monitor that no longer exists is ignored**, instead of
  opening the bar somewhere unreachable.

## [0.1.0]

The first release: a one-line, always-on-top bar showing the task you are focused on.

- **One focused task, everything else on a stack under it.** Left-click `+` for a new task,
  right-click `+` to finish the focused one, middle-click `+` to take a break.
- **A permanent `pause` task** that can never be finished, so breaks appear in the reports
  like any other task and the bar is never empty.
- **Time is only collected for the task on top**, split correctly across midnight, and
  never credited for hours when the app was closed.
- **Renaming** the focused task in place, from a right-click on its name.
- **A burger menu** carrying the task list inline, plus _groom_ (finish several tasks at
  once), _end day_ (today's report; closing the day saves and quits), _week_ (a grid of
  tasks against weekdays) and _revive_ (put a finished task back on the stack).
- **A drag handle** at the left edge, with the task name and empty space working too, and a
  window frame that can be switched on for desktops that will not move a bare window.
- **State in an embedded redb database** in the platform's data directory, written on every
  change and at least every 30 seconds, restored on startup down to the focused task and
  the window position.
