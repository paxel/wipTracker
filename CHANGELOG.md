# Changelog

All notable changes to WipTracker are documented in this file.

## [1.5.0]

### Added

- When the day's timer is reached, a small window asks whether to end the day: _End day_
  saves and quits, _Not yet_ asks again a quarter of an hour later
  ([wip-1028](https://github.com/paxel/wipTracker/issues/52)).

### Changed

- The day's alarm sounds once; it no longer repeats every ten minutes, and the menu entry
  _mute day reminder_ is gone with it
  ([wip-1028](https://github.com/paxel/wipTracker/issues/52)).
- The task stack, revive, the timers, groom, the day's report, end earlier and the week are
  the tabs of one window, where each used to open a window of its own
  ([wip-1025](https://github.com/paxel/wipTracker/issues/49)).
- Today's report is the tab "Today" and can be looked at without ending anything; its one
  button is now called _End day_, and the menu entry _today_
  ([wip-1027](https://github.com/paxel/wipTracker/issues/51)).
- The buttons in the windows look like buttons: larger, raised, and they sink when pressed
  ([wip-1029](https://github.com/paxel/wipTracker/issues/53)).

### Fixed

- WipTracker no longer asks when you really stopped on every start after a night: it
  offers to end earlier only when it ran through midnight itself and counted that time
  ([wip-1026](https://github.com/paxel/wipTracker/issues/50)).

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
