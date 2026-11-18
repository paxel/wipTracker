# Changelog

All notable changes to WipTracker are documented in this file.

## [1.4.0]

### Changed

- The state file moves to a newer database format: the first start converts it and keeps
  the file from before beside it as `wiptracker.redb.v2`. WipTracker 1.3.1 and older
  cannot open the converted file, only that copy
  ([wip-1023](https://github.com/paxel/wipTracker/issues/45)).

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
