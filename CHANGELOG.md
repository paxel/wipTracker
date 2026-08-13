# Changelog

All notable changes to WipTracker are documented in this file.

## [0.5.0]

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
- **The menu opens where it fits.** It was always placed directly below the bar, so a bar
  near the bottom of the screen opened it past the edge. It now goes wherever it is fully
  visible, and scrolls if the screen is shorter than the list. Native Wayland is the
  exception: no client can place a window there, so the compositor decides.
- **The burger closes the menu again.** Pressing it took the focus off the menu window,
  which closed it a frame before the click arrived — and the click then reopened it, so the
  button looked like it could only open.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
