# Changelog

All notable changes to WipTracker are documented in this file.

## [1.1.0] - Unreleased

### Fixed

- **The menu, the hints and the report windows stay on the monitor the bar is on.**
  They flip above the bar when they would fall off the bottom of that monitor, and are
  cut to its height, on every monitor rather than only the primary one. The app now asks
  the platform where each monitor is — `NSScreen` on macOS, RandR on X11,
  `EnumDisplayMonitors` on Windows — instead of guessing from the monitor's size alone.

### Changed

- **On macOS, starting from a terminal gives the prompt back.** The bar detaches itself
  from the shell it was started in, so closing that terminal no longer closes the bar.
  `--foreground` keeps the old behaviour.
- **On macOS, _start with my session_ is a launchd agent** in `~/Library/LaunchAgents`
  instead of a login item. It starts the bar directly at login, without a Terminal
  window, and without asking for permission to control System Events.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
