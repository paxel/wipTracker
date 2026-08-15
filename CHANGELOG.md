# Changelog

All notable changes to WipTracker are documented in this file.

## [0.7.0]

### Added

- **_leave the taskbar_ in the menu.** The bar is always visible anyway, so its taskbar
  entry can go: one toggle keeps it out of the taskbar (and, on Windows, off the taskbar
  icon list). Like the window frame it takes effect after a restart, and the entry comes
  back with the same toggle.

### Changed

- **Hints and tooltips wait two seconds before appearing.** The hint window used to pop
  up the instant the pointer touched a control, which put an explanation in the way of
  every pass over the bar. It now waits until the pointer has rested on the bar a while —
  except during a hold, whose progress is shown at once. The tooltips inside the timer
  and report windows wait the same two seconds.

### Fixed

- **The bar actually stays on top again.** eframe opens every window hidden until the
  first frame is painted, and X11 window managers ignore the keep-above request winit
  sends while a window is hidden — so the bar started underneath everything and stayed
  there. The request is repeated once the window is visible, which is when it sticks.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
