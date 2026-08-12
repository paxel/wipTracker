# WipTracker

A one-line, always-on-top bar that shows the task you are focused on right now.

Everything else you are juggling sits on a stack underneath it. Time is only collected for
the task on top, so the numbers at the end of the day describe where your attention
actually went — not how long a ticket was open.

![The WipTracker bar](docs/bar.png)

![What each part of the bar does](docs/bar-anatomy.svg)

| Where     | Left click | Right click            | Middle click       |
| --------- | ---------- | ---------------------- | ------------------ |
| task name | —          | rename                 | open the task list |
| `+`       | new task   | finish the current one | take a break       |
| `≡`       | menu       | —                      | —                  |

## Install

Download the archive for your platform from the
[latest release](https://github.com/paxel/wipTracker/releases/latest).

**Linux** — unpack and run the binary:

```sh
tar -xzf wiptracker-*-linux-x86_64.tar.gz
./wiptracker
```

**Windows** — unpack the zip and run `wiptracker.exe`.

**macOS** — unpack the zip and move `WipTracker.app` to `/Applications`. The build is not
signed with an Apple Developer certificate, so Gatekeeper will refuse it on first launch
("WipTracker is damaged"). Clear the quarantine flag once:

```sh
xattr -cr /Applications/WipTracker.app
```

Scoop and Homebrew packaging will follow in
[paxel/scoop-bucket](https://github.com/paxel/scoop-bucket) and
[paxel/homebrew-tap](https://github.com/paxel/homebrew-tap).

## Usage

**Start a task.** Left-click `+`. A task called `new task 1` appears on the bar and its
clock starts. The number never repeats, so `new task 7` means the same task tomorrow.

**Rename it.** Right-click the task name. The name turns into a text field: type, then
press Enter to keep it or Escape to go back to the old name.

**Interrupted?** Left-click `+` again. The new task goes on top of the stack and takes over
the bar; the one underneath stops collecting time but stays open.

**Switch back.** Middle-click the task name — or open the menu with `≡` — and pick a task
under _switch to_. It moves to the
top of the stack and starts collecting time again. The list is in stack order, with the
current task marked and `pause` at the bottom.

**Take a break.** Middle-click `+`, or pick `pause` from the menu. It is a permanent task that can never be
finished, so breaks show up in the reports like everything else. Right-click `+` while
`pause` is on top to end the break and return to what you were doing.

**Finish a task.** Right-click `+`. The task leaves the stack, its end time is recorded,
and the task underneath comes back. There is no confirmation — see _revive_ below if you
did not mean it. When nothing else is open, `pause` takes over.

**Clean up.** Menu → _groom_ opens a window listing every open task with its total time.
Tick several and press _Finish selected_ to close them all at once.

**End the day.** Menu → _end day_ shows when the day started, what you worked on, how long
each took, and the day's total. Day start and end are derived from your activity, never
typed in. Press _Close day_ to stamp the day as finished — WipTracker saves and quits,
because a clock left running overnight would credit tomorrow's hours to today. Open tasks
stay on the stack and come back when you start it again. Closing the window without
pressing the button changes nothing.

**Look back.** Menu → _week_ shows a grid: one row per task, one column per weekday, plus
totals down the side and along the bottom. Use _previous_ / _next_ / _today_ to move
between weeks, or type any date as `YYYY-MM-DD` into _jump to date_ and press Enter to see
the week containing it.

**Undo a finish.** Menu → _revive_ lists finished tasks, most recent first. Clicking one
puts it back on top of the stack and clears its end time; its total keeps counting from
where it left off. The entry is greyed out when nothing has been finished yet, and the
window closes itself once the last finished task has been brought back.

**Hide the clock.** Menu → _hide duration_ leaves just the task name on the bar.

**Move the bar.** Drag the dotted handle at the left edge — or the task name, or anywhere
else that is not a button. The position
is remembered. WipTracker first asks the window manager for its own move gesture and falls
back to moving the window itself.

**Can't move it?** Menu → _show window frame_ gives the window its normal title bar, which
every environment knows how to drag. The choice is remembered, and _hide window frame_
takes it away again. On Wayland the frame is on by default, because Wayland compositors
often ignore the move gesture and never report a window position for the fallback to use;
X11, macOS and Windows start frameless.

## Data

Everything is stored in an embedded [redb](https://github.com/cberner/redb) database:

| Platform | Location                                                   |
| -------- | ---------------------------------------------------------- |
| Linux    | `$XDG_DATA_HOME/wiptracker/wiptracker.redb` (or `~/.local/share/…`) |
| macOS    | `~/Library/Application Support/WipTracker/wiptracker.redb`  |
| Windows  | `%APPDATA%\WipTracker\data\wiptracker.redb`                 |

State is written whenever something changes and at least every 30 seconds, so a crash
costs a few seconds at most. Back the file up by copying it; start fresh by deleting it.

## Known limitations

- **Wayland ignores always-on-top.** There is no Wayland protocol for it, so the bar
  behaves like a normal window on GNOME, KDE and friends. Use the compositor's own window
  rule (KDE: _Special Window Settings → Keep above other windows_) or run under XWayland.
  Windows, macOS and X11 work as expected.
- **Wayland also often ignores the move gesture**, which is why the window frame is on by
  default there; see _Can't move it?_ above.
- **The macOS build is unsigned.** See the `xattr` step above.
- **Only one instance at a time.** The database is locked while WipTracker runs; a second
  instance reports the lock in the bar instead of starting.
- **The clock stops with the app.** Time while WipTracker is not running is never counted,
  even if a task was focused when you quit.

## Development

```sh
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

The UI tests render the real app headlessly through `egui_kittest` and write PNGs to
`target/render/`, so layout and contrast can be checked without a desktop session.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
