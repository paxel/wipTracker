# Older changes

Released versions before the one in [CHANGELOG.md](CHANGELOG.md).

## [0.5.0]

### Fixed

- **The menu and the hint follow the bar onto a second monitor.** Both were clamped to the
  width of the monitor the bar reports, which is the size of one screen while positions
  count across all of them — so from about half way along a second monitor they stopped
  following and stuck near the middle of the desktop. The left edge is now always the
  bar's own; they are no wider than the bar, so there was nothing to clamp for.
- **The Homebrew install shows up in the application menu.** The launcher entry went into
  Homebrew's own share directory, which no desktop session reads, and named the binary and
  the icon by bare name — so even when a menu found the entry it could neither draw nor
  launch it. Both are absolute paths now, and `wiptracker-install-icon` copies the entry
  and the icons into `~/.local/share`, which every session does read. Until then the app
  could not be found in a menu or pinned to a panel.
- **The bar no longer relocates itself into a gap between monitors.** It used to decide a
  stored position was unreachable whenever it lay outside a rectangle the size of one
  monitor — true for any position on a second screen — and then centre the window inside
  that same rectangle, which on a desktop whose monitors are not aligned is a region no
  screen covers. The stored position is now used as given, and `--reset-position` opens
  where the window manager wants for the case that guessing was meant to solve.

## [0.4.0]

### Added

- **Start with your session, on every platform.** `install-icon.sh --autostart` on Linux
  writes the autostart entry (and `--no-autostart` removes it); on Windows the zip now
  carries `install-shortcut.ps1`, which puts WipTracker in the Start menu and, with
  `-Startup`, into your login; on macOS the README says where the Login Items setting is.
  The Windows script also fills the gap where only Scoop users had a Start-menu entry.
- **A hint window beside the bar.** Hovering a control explains it, and holding one shows
  the reading cat filling up as the hold runs. The bar's tooltips could never work: egui
  keeps a tooltip inside the window it belongs to, and that window is 32 pixels tall, so
  three lines of explanation were squeezed into a strip nothing could be read in. The hint
  takes no focus, stays out of the taskbar and passes the mouse straight through.

### Changed

- **The icon comes in every size a menu draws at**, instead of one 512 pixel file that
  each desktop had to downsample itself — 32 to 256 pixels for launchers and taskbars,
  1024 for the macOS bundle. All of them are installed by the `.deb`, by Homebrew and by
  `install-icon.sh`.
- **Launcher search finds WipTracker by what it does.** The desktop entry lists keywords —
  time, timer, tracking, focus, task, productivity — where before only the name matched.
  It also declares one main category instead of two, so it no longer risks appearing twice
  in the same menu.
- **Left click and hold are the only gestures.** Right click, middle click and double click
  are gone from the bar, so a trackpad, a touchpad and a touchscreen can all reach every
  command. Clicking the task name renames it; holding it for two seconds finishes it, or
  ends the break when `pause` is on top. A new **fork button** opens the task stack on a
  click and takes a break on a hold. `+` still starts a task, and holding it puts a
  finished task back. The burger toggles the menu, and holding it opens the daily timers.
  While a control is held it fills up, so the wait is visible; letting go early does the
  click instead.
- **Every gesture is also a menu entry.** The menu gained _new task_, _rename_, _finish_
  and _pause_, is grouped into sections, and each entry's tooltip names the gesture that
  does the same thing. A touchscreen shows no tooltips at all, so the menu is the only
  place the gestures can be discovered by looking.
- **The drag handle is the only way to move the bar.** The task name and the empty
  background used to drag the window too; the name cannot both be dragged and held for two
  seconds, and a stray press on the background moved the bar by accident. On a window with
  a frame the handle is dropped altogether — the title bar does the job — and the bar is
  ten pixels narrower for it.

### Fixed

- **The bar stays on top on Wayland again, by running under XWayland.** No Wayland client
  can ask to stay above other windows, and the bar was silently behaving like an ordinary
  window on every Wayland desktop — the one thing the app exists to do. It now asks for
  the X11 backend, which every mainstream Wayland session provides through XWayland, and
  window placement and the remembered position come back with it.
  `WIPTRACKER_BACKEND=wayland` forces the native path for anyone who prefers it; there the
  app says what it cannot do, once on startup and permanently at the bottom of the menu,
  and the README carries the compositor rules to use instead.
- **The menu and the rename field paint their own colours.** Both used to take their
  background from the theme, and a mac user saw white labels on white pills and white text
  in a white field even after the palette was pinned. Nothing about them reads the style
  any more, so nothing can override it, and the app repairs the palette whenever it finds
  it replaced rather than trusting one flag.
- **The menu opens where it fits.** It was always placed directly below the bar, so a bar
  near the bottom of the screen opened it past the edge. It now goes wherever it is fully
  visible, and scrolls if the screen is shorter than the list. Native Wayland is the
  exception: no client can place a window there, so the compositor decides.
- **The burger closes the menu again.** Pressing it took the focus off the menu window,
  which closed it a frame before the click arrived — and the click then reopened it, so the
  button looked like it could only open.

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
