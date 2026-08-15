# Changelog

All notable changes to WipTracker are documented in this file.

## [0.8.0]

### Added

- **_hide hints_ in the menu.** The hover explanations can now be turned off entirely —
  and back on — with one menu entry. On by default.

### Removed

- **The cat is gone from the hint window.** During a hold it doubled the progress sweep
  the held control already draws, so the hint window now shows only its text.

### Fixed

- **Upgrading no longer breaks the application-menu entry.** The entry named the binary
  by the path it ran from — for Homebrew a directory that carries the version and is
  deleted by the next upgrade, leaving a menu entry that points at nothing and, despite
  `TryExec`, keeps being shown. Two repairs: the entry now names the binary's stable
  `$PATH` name (Homebrew's `bin` symlink, repointed on every upgrade) instead of the
  versioned directory — and at startup, an entry or autostart entry this app once wrote
  that no longer names the running binary is silently rewritten, so existing broken
  entries heal themselves on the first start after an update.

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
