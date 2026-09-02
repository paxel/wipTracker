# Changelog

All notable changes to WipTracker are documented in this file.

## [1.1.0] - Unreleased

### Changed

- **On macOS, starting from a terminal gives the prompt back.** The bar detaches itself
  from the shell it was started in, so closing that terminal no longer closes the bar.
  `--foreground` keeps the old behaviour.
- **On macOS, _start with my session_ is a launchd agent** in `~/Library/LaunchAgents`
  instead of a login item. It starts the bar directly at login, without a Terminal
  window, and without asking for permission to control System Events.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
