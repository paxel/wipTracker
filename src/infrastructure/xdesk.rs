//! Speaking EWMH to the X server directly, for what winit has no API for.
//!
//! Keeping a window out of the taskbar is `_NET_WM_STATE_SKIP_TASKBAR`, a state the
//! window manager owns: a client asks for it with a client message, and only for a
//! mapped window. winit exposes none of that on X11 — its `with_taskbar` is
//! Windows-only — so the app asks the server itself. The sweep runs against every
//! window of this process, so the menu, the hint and the report windows are covered
//! the frame after they appear, whoever created them.
//!
//! Everywhere but X11 this module is a stub that does nothing.

#[cfg(target_os = "linux")]
pub use real::XDesk;

/// One frame's worth of taskbar upkeep, connecting on the first call: the whole dance in
/// one place so the app only holds the state. Embedded viewports — the test harness, the
/// web build — are not real windows, so there is nothing to sweep there.
pub fn sweep_frame(
    desk: &mut Option<XDesk>,
    tried: &mut bool,
    embedded: bool,
    bar_in_taskbar: bool,
) {
    if embedded {
        return;
    }
    if !*tried {
        *tried = true;
        *desk = XDesk::connect();
    }
    if let Some(desk) = desk {
        desk.sweep(bar_in_taskbar);
    }
}

#[cfg(not(target_os = "linux"))]
pub struct XDesk;

#[cfg(not(target_os = "linux"))]
impl XDesk {
    pub fn connect() -> Option<Self> {
        None
    }

    pub fn sweep(&self, _bar_in_taskbar: bool) {}
}

#[cfg(target_os = "linux")]
mod real {
    use x11rb::connection::Connection as _;
    use x11rb::protocol::xproto::{
        Atom, AtomEnum, ClientMessageEvent, ConnectionExt as _, EventMask, Window,
    };
    use x11rb::rust_connection::RustConnection;

    /// From the EWMH spec: `_NET_WM_STATE` client message data.l[0].
    const STATE_REMOVE: u32 = 0;
    const STATE_ADD: u32 = 1;

    pub struct XDesk {
        conn: RustConnection,
        root: Window,
        net_client_list: Atom,
        net_wm_pid: Atom,
        net_wm_name: Atom,
        utf8_string: Atom,
        net_wm_state: Atom,
        skip_taskbar: Atom,
        skip_pager: Atom,
    }

    impl XDesk {
        /// `None` where there is no X server to talk to — native Wayland, a headless
        /// test runner — in which case there is no taskbar state to manage either.
        pub fn connect() -> Option<Self> {
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
            Some(Self {
                root,
                net_client_list: atom("_NET_CLIENT_LIST")?,
                net_wm_pid: atom("_NET_WM_PID")?,
                net_wm_name: atom("_NET_WM_NAME")?,
                utf8_string: atom("UTF8_STRING")?,
                net_wm_state: atom("_NET_WM_STATE")?,
                skip_taskbar: atom("_NET_WM_STATE_SKIP_TASKBAR")?,
                skip_pager: atom("_NET_WM_STATE_SKIP_PAGER")?,
                conn,
            })
        }

        /// Brings every window of this process to its wanted taskbar state: the bar
        /// follows the preference, everything else — menu, hint, offer, the report
        /// windows — never belongs in a taskbar. Windows already right are left alone,
        /// so running this every frame costs a few property reads and no messages.
        pub fn sweep(&self, bar_in_taskbar: bool) {
            let pid = std::process::id();
            let Some(windows) = self.our_windows(pid) else {
                return;
            };
            for (window, name) in windows {
                let is_bar = name == "WipTracker";
                let wanted_skip = !is_bar || !bar_in_taskbar;
                let has_skip = self.has_skip_taskbar(window);
                if wanted_skip != has_skip {
                    self.ask_skip_taskbar(window, wanted_skip);
                }
            }
            let _ = self.conn.flush();
        }

        /// The managed windows belonging to `pid`, with their titles.
        fn our_windows(&self, pid: u32) -> Option<Vec<(Window, String)>> {
            let list = self
                .conn
                .get_property(
                    false,
                    self.root,
                    self.net_client_list,
                    AtomEnum::WINDOW,
                    0,
                    1024,
                )
                .ok()?
                .reply()
                .ok()?;
            let mut ours = Vec::new();
            for window in list.value32()? {
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
            Some(ours)
        }

        fn has_skip_taskbar(&self, window: Window) -> bool {
            self.conn
                .get_property(false, window, self.net_wm_state, AtomEnum::ATOM, 0, 32)
                .ok()
                .and_then(|cookie| cookie.reply().ok())
                .and_then(|reply| Some(reply.value32()?.any(|atom| atom == self.skip_taskbar)))
                .unwrap_or(false)
        }

        /// The EWMH way to change a state on a mapped window: a client message to the
        /// root, which the window manager acts on.
        fn ask_skip_taskbar(&self, window: Window, skip: bool) {
            let action = if skip { STATE_ADD } else { STATE_REMOVE };
            let event = ClientMessageEvent::new(
                32,
                window,
                self.net_wm_state,
                [action, self.skip_taskbar, self.skip_pager, 0, 0],
            );
            let _ = self.conn.send_event(
                false,
                self.root,
                EventMask::SUBSTRUCTURE_REDIRECT | EventMask::SUBSTRUCTURE_NOTIFY,
                event,
            );
        }
    }
}
