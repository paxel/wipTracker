# Changelog

All notable changes to WipTracker are documented in this file.

## [1.1.0] - Unreleased

### Fixed

- **The menu, the hints and the report windows open on the monitor the bar is on.**
  With several monitors they used to be placed as if the bar's monitor began at the
  desktop's origin, which on a stacked layout put them behind the bar or off every
  screen. The app now asks the platform where each monitor is — `NSScreen` on macOS,
  RandR on X11, `EnumDisplayMonitors` on Windows — and keeps every window inside the
  bar's own.

### Changed

- **On macOS, starting from a terminal gives the prompt back.** The bar detaches itself
  from the shell it was started in, so closing that terminal no longer closes the bar.
  `--foreground` keeps the old behaviour.
- **On macOS, _start with my session_ is a launchd agent** in `~/Library/LaunchAgents`
  instead of a login item. It starts the bar directly at login, without a Terminal
  window, and without asking for permission to control System Events.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
