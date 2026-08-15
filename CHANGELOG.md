# Changelog

All notable changes to WipTracker are documented in this file.

## [0.9.0]

### Added

- **Integration tests against a real desktop.** The app is spawned as a process, driven
  with synthetic mouse input, and judged by what the window manager actually did:
  keep-above applied, taskbar entries hidden, the menu surviving a hint, closing the day
  quitting the process. They run wherever an X11 session with a window manager and
  `xautomation` exist, and skip themselves cleanly anywhere else, CI included.

### Fixed

- **The menu, the hint and the report windows no longer show up in the taskbar** — and
  the _leave the taskbar_ toggle now actually works, immediately, no restart. winit has
  no X11 API for the taskbar, so the earlier attempt silently did nothing; the app now
  asks the window manager itself (`_NET_WM_STATE_SKIP_TASKBAR`) for every window it owns.
- **A hint no longer closes the open menu.** The menu closes itself when the focus goes
  somewhere else — and the hint window appearing next to it, once its two seconds were
  up, was exactly that. While the menu is open, no hints.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
