//! Integration tests against a real X11 desktop: the app is spawned as a process, driven
//! with synthetic pointer input, and judged by the window states the window manager
//! actually applied — the layer the unit tests cannot see and where the real-world bugs
//! (bar not on top, windows in the taskbar, the hint killing the menu) lived.
//!
//! They need a session to run in: X11, an EWMH window manager, and `xte` for input.
//! Anywhere that is missing — CI runners, native Wayland — each test says so and passes
//! vacuously. The tests share the one pointer and the one screen, so they run under a
//! lock, and the pointer is put back where it was found.

#![cfg(target_os = "linux")]

use std::process::{Child, Command};
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, Instant};

use x11rb::connection::Connection as _;
use x11rb::protocol::xproto::{Atom, AtomEnum, ConnectionExt as _, Window};
use x11rb::rust_connection::RustConnection;

/// One test at a time: they share the pointer and the screen.
static DESKTOP: Mutex<()> = Mutex::new(());

struct Desk {
    conn: RustConnection,
    root: Window,
    net_client_list: Atom,
    net_wm_pid: Atom,
    net_wm_name: Atom,
    utf8_string: Atom,
    net_wm_state: Atom,
    state_above: Atom,
    skip_taskbar: Atom,
}

impl Desk {
    /// `None` where the preconditions are missing; the caller then skips.
    fn open() -> Option<Self> {
        let (conn, screen) = x11rb::connect(None).ok()?;
        let root = conn.setup().roots.get(screen)?.root;
        let atom = |name: &str| -> Option<Atom> {
            Some(
                conn.intern_atom(false, name.as_bytes())
                    .ok()?
                    .reply()
                    .ok()?
                    .atom,
            )
        };
        let desk = Self {
            root,
            net_client_list: atom("_NET_CLIENT_LIST")?,
            net_wm_pid: atom("_NET_WM_PID")?,
            net_wm_name: atom("_NET_WM_NAME")?,
            utf8_string: atom("UTF8_STRING")?,
            net_wm_state: atom("_NET_WM_STATE")?,
            state_above: atom("_NET_WM_STATE_ABOVE")?,
            skip_taskbar: atom("_NET_WM_STATE_SKIP_TASKBAR")?,
            conn,
        };
        // Without an EWMH window manager there is nobody to apply the states under test.
        let check = atom_of(&desk, "_NET_SUPPORTING_WM_CHECK")?;
        let managed = desk
            .conn
            .get_property(false, desk.root, check, AtomEnum::WINDOW, 0, 1)
            .ok()?
            .reply()
            .ok()?;
        (managed.value_len > 0).then_some(desk)
    }

    /// The windows of `pid`, as (id, title).
    fn windows_of(&self, pid: u32) -> Vec<(Window, String)> {
        let Ok(Ok(list)) = self
            .conn
            .get_property(
                false,
                self.root,
                self.net_client_list,
                AtomEnum::WINDOW,
                0,
                1024,
            )
            .map(|cookie| cookie.reply())
        else {
            return Vec::new();
        };
        let Some(ids) = list.value32() else {
            return Vec::new();
        };
        let mut ours = Vec::new();
        for window in ids {
            let owner = self
                .conn
                .get_property(false, window, self.net_wm_pid, AtomEnum::CARDINAL, 0, 1)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .and_then(|reply| reply.value32()?.next());
            if owner != Some(pid) {
                continue;
            }
            let name = self
                .conn
                .get_property(false, window, self.net_wm_name, self.utf8_string, 0, 256)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .map(|reply| String::from_utf8_lossy(&reply.value).into_owned())
                .unwrap_or_default();
            ours.push((window, name));
        }
        ours
    }

    /// Waits for a window of `pid` whose title matches `wanted` exactly.
    fn wait_for(&self, pid: u32, wanted: &str, patience: Duration) -> Option<Window> {
        let until = Instant::now() + patience;
        while Instant::now() < until {
            if let Some((window, _)) = self
                .windows_of(pid)
                .into_iter()
                .find(|(_, name)| name == wanted)
            {
                return Some(window);
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        None
    }

    fn has_state(&self, window: Window, state: Atom) -> bool {
        self.conn
            .get_property(false, window, self.net_wm_state, AtomEnum::ATOM, 0, 32)
            .ok()
            .and_then(|cookie| cookie.reply().ok())
            .and_then(|reply| Some(reply.value32()?.any(|atom| atom == state)))
            .unwrap_or(false)
    }

    /// Waits until `window` has (or has lost) `state`.
    fn wait_for_state(
        &self,
        window: Window,
        state: Atom,
        wanted: bool,
        patience: Duration,
    ) -> bool {
        let until = Instant::now() + patience;
        while Instant::now() < until {
            if self.has_state(window, state) == wanted {
                return true;
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        false
    }

    fn geometry(&self, window: Window) -> Option<(i16, i16, u16, u16)> {
        let geometry = self.conn.get_geometry(window).ok()?.reply().ok()?;
        let onto = self
            .conn
            .translate_coordinates(window, self.root, 0, 0)
            .ok()?
            .reply()
            .ok()?;
        Some((onto.dst_x, onto.dst_y, geometry.width, geometry.height))
    }

    fn pointer(&self) -> Option<(i16, i16)> {
        let reply = self.conn.query_pointer(self.root).ok()?.reply().ok()?;
        Some((reply.root_x, reply.root_y))
    }
}

fn atom_of(desk: &Desk, name: &str) -> Option<Atom> {
    Some(
        desk.conn
            .intern_atom(false, name.as_bytes())
            .ok()?
            .reply()
            .ok()?
            .atom,
    )
}

/// Moves the pointer and clicks with `xte`; the layer beneath winit, so the app cannot
/// tell these from a person.
fn click_at(x: i32, y: i32) {
    let _ = Command::new("xte")
        .arg(format!("mousemove {x} {y}"))
        .status();
    std::thread::sleep(Duration::from_millis(600));
    let _ = Command::new("xte").arg("mouseclick 1").status();
    std::thread::sleep(Duration::from_millis(600));
}

/// The spawned app against a scratch HOME, cleaned up however the test ends. A launcher
/// entry is pre-seeded so the one-time offer window stays out of the way.
struct App {
    child: Child,
    _home: tempfile::TempDir,
}

impl App {
    fn spawn() -> std::io::Result<Self> {
        let home = tempfile::tempdir()?;
        let applications = home.path().join(".local/share/applications");
        std::fs::create_dir_all(&applications)?;
        std::fs::write(applications.join("wiptracker.desktop"), "")?;
        let child = Command::new(env!("CARGO_BIN_EXE_wiptracker"))
            .env("HOME", home.path())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()?;
        Ok(Self { child, _home: home })
    }

    fn pid(&self) -> u32 {
        self.child.id()
    }
}

impl Drop for App {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// The desk, the lock, and the spawned app — or `None` with the reason printed, which
/// passes the test vacuously where there is no desktop to test against.
fn desktop() -> Option<(MutexGuard<'static, ()>, Desk, App)> {
    // The coverage script sets this: these tests only run where a desktop exists, so
    // letting them into the measurement would make the number differ between a
    // workstation and the headless CI runner — and the ratchet floor must not.
    if std::env::var_os("WIPTRACKER_SKIP_DESKTOP_TESTS").is_some() {
        eprintln!("skipping: WIPTRACKER_SKIP_DESKTOP_TESTS is set");
        return None;
    }
    let guard = DESKTOP
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let Some(desk) = Desk::open() else {
        eprintln!("skipping: no X11 session with an EWMH window manager");
        return None;
    };
    if !Command::new("xte")
        .arg("sleep 0")
        .status()
        .is_ok_and(|status| status.success())
    {
        eprintln!("skipping: xte (xautomation) is not installed");
        return None;
    }
    let app = match App::spawn() {
        Ok(app) => app,
        Err(error) => {
            eprintln!("skipping: could not spawn the app: {error}");
            return None;
        }
    };
    Some((guard, desk, app))
}

/// Restores the pointer to where the person left it, whichever way the test ends.
struct PointerGuard(Option<(i16, i16)>);

impl Drop for PointerGuard {
    fn drop(&mut self) {
        if let Some((x, y)) = self.0 {
            let _ = Command::new("xte")
                .arg(format!("mousemove {x} {y}"))
                .status();
        }
    }
}

const PATIENCE: Duration = Duration::from_secs(10);
/// Longer than the hint's hover delay, so waiting this long proves delayed effects.
const PAST_HINT_DELAY: Duration = Duration::from_secs(3);

/// Menu layout, mirrored from `ui/menu.rs`: frame margin, row height, separator height.
/// The row centers computed from these are what the clicks aim at.
const MENU_MARGIN: i32 = 8;
const MENU_ROW: i32 = 26;
const MENU_SEPARATOR: i32 = 6;

/// The center y of `row` (counting items only) with `separators` above it.
fn menu_row_y(row: i32, separators: i32) -> i32 {
    MENU_MARGIN + row * MENU_ROW + separators * MENU_SEPARATOR + MENU_ROW / 2
}

/// Clicks the burger (the rightmost control) and waits for the menu window.
fn open_menu(desk: &Desk, app: &App) -> Option<(Window, (i16, i16, u16, u16))> {
    let bar = desk.wait_for(app.pid(), "WipTracker", PATIENCE)?;
    let (x, y, width, height) = desk.geometry(bar)?;
    click_at(x as i32 + width as i32 - 14, y as i32 + height as i32 / 2);
    let menu = desk.wait_for(app.pid(), "WipTracker menu", PATIENCE)?;
    let geometry = desk.geometry(menu)?;
    Some((menu, geometry))
}

#[test]
fn the_bar_is_kept_above_and_shown_in_the_taskbar_by_default() {
    let Some((_guard, desk, app)) = desktop() else {
        return;
    };
    let bar = desk
        .wait_for(app.pid(), "WipTracker", PATIENCE)
        .expect("the bar window appears");
    assert!(
        desk.wait_for_state(bar, desk.state_above, true, PATIENCE),
        "the window manager keeps the bar above — the keep-above request survived the \
         hidden-until-first-frame start"
    );
    std::thread::sleep(PAST_HINT_DELAY);
    assert!(
        !desk.has_state(bar, desk.skip_taskbar),
        "without the preference the bar stays in the taskbar"
    );
}

#[test]
fn the_menu_skips_the_taskbar_and_survives_the_hint_delay() {
    let Some((_guard, desk, app)) = desktop() else {
        return;
    };
    let pointer = PointerGuard(desk.pointer());
    let (menu, _) = open_menu(&desk, &app).expect("the burger opens the menu");
    assert!(
        desk.wait_for_state(menu, desk.skip_taskbar, true, PATIENCE),
        "the menu never belongs in the taskbar"
    );
    // The pointer is still resting on the burger; before the fix the hint window popped
    // up past its delay, took the focus, and the menu closed itself.
    std::thread::sleep(PAST_HINT_DELAY);
    assert!(
        desk.wait_for(app.pid(), "WipTracker menu", Duration::from_secs(1))
            .is_some(),
        "the menu survives the pointer resting on the burger past the hint delay"
    );
    drop(pointer);
}

#[test]
fn the_taskbar_toggle_applies_while_running() {
    let Some((_guard, desk, app)) = desktop() else {
        return;
    };
    let pointer = PointerGuard(desk.pointer());
    let bar = desk
        .wait_for(app.pid(), "WipTracker", PATIENCE)
        .expect("the bar window appears");
    let (_, (x, y, _, height)) = open_menu(&desk, &app).expect("the burger opens the menu");
    // "leave the taskbar" is the last row.
    click_at(
        x as i32 + 135,
        y as i32 + height as i32 - MENU_ROW / 2 - MENU_MARGIN / 2,
    );
    assert!(
        desk.wait_for_state(bar, desk.skip_taskbar, true, PATIENCE),
        "the toggle takes the bar out of the taskbar without a restart"
    );
    drop(pointer);
}

#[test]
fn closing_the_day_quits_the_app() {
    let Some((_guard, desk, mut app)) = desktop() else {
        return;
    };
    let pointer = PointerGuard(desk.pointer());
    let (_, (x, y, _, _)) = open_menu(&desk, &app).expect("the burger opens the menu");
    // "end day" is the ninth item, below two separators.
    click_at(x as i32 + 135, y as i32 + menu_row_y(8, 2));
    let report = desk
        .wait_for(app.pid(), "WipTracker — end day", PATIENCE)
        .expect("the end-day window opens");
    let (rx, ry, _, _) = desk.geometry(report).expect("end-day geometry");
    // "Close day" sits under the export button; day empty, so the layout is fixed.
    click_at(rx as i32 + 38, ry as i32 + 170);
    let until = Instant::now() + PATIENCE;
    let exited = loop {
        if let Ok(Some(_)) = app.child.try_wait() {
            break true;
        }
        if Instant::now() > until {
            break false;
        }
        std::thread::sleep(Duration::from_millis(300));
    };
    assert!(exited, "closing the day saves and quits");
    drop(pointer);
}
