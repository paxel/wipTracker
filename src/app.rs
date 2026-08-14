//! The eframe application: owns the state and drives the bar and its windows.

use std::time::{Duration, Instant};

use chrono::{DateTime, Local};

use crate::domain::ports::{Alarm, IdleProbe, Snapshot, Store, StoreError};
use crate::domain::task::{PAUSE_ID, TaskId};
use crate::domain::tracker::Tracker;
use crate::infrastructure::beeper::Beeper;
use crate::infrastructure::idle::SystemIdle;
use crate::infrastructure::launcher;
use crate::theme;
use crate::ui::bar::{self, BarAction};
use crate::ui::format;
use crate::ui::hint;
use crate::ui::menu::{self, MenuAction};
use crate::ui::windows::{self, OpenWindows};

/// Which windowing backend the app asks for on Linux.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Backend {
    X11,
    Wayland,
}

/// The environment variable that overrides the choice below, for anyone who would rather
/// have the native path and can live without the bar staying on top.
pub const BACKEND_OVERRIDE: &str = "WIPTRACKER_BACKEND";

/// Which backend to ask for.
///
/// Three things this app is built on are missing on Wayland, and none of them fail loudly:
/// a window cannot be placed, cannot be kept above other windows, and there is no popup
/// protocol. The library underneath implements none of them — `set_window_level` is an
/// empty function there — so always-on-top, the point of the app, silently does nothing.
///
/// winit picks Wayland whenever `WAYLAND_DISPLAY` is set, even when XWayland is running.
/// Under XWayland all three work again, and nearly every Wayland desktop runs it, so that
/// is what the app asks for. `WIPTRACKER_BACKEND=wayland` forces the native path anyway.
pub fn choose_backend() -> Backend {
    backend_for(
        std::env::var_os("WAYLAND_DISPLAY").is_some(),
        x11_is_reachable(),
        std::env::var(BACKEND_OVERRIDE).ok().as_deref(),
    )
}

/// Whether there is an X server behind `DISPLAY` that will actually answer.
///
/// A `DISPLAY` left over from an X server that has gone is common enough — exported in a
/// shell profile, inherited from a dead session — and winit cannot recover from being
/// pointed at one: asking for X11 and failing ends the process, and a second event loop
/// cannot be built to try the other backend afterwards. So the socket is opened before the
/// choice is made rather than after.
///
/// A display on another host is taken at its word; only the local socket is cheap enough
/// to test.
#[cfg(unix)]
fn x11_is_reachable() -> bool {
    let Some(display) = std::env::var_os("DISPLAY") else {
        return false;
    };
    let display = display.to_string_lossy().into_owned();
    let Some((host, rest)) = display.split_once(':') else {
        return false;
    };
    if !host.is_empty() && host != "unix" {
        return true;
    }
    let number = rest.split('.').next().unwrap_or_default();
    std::os::unix::net::UnixStream::connect(format!("/tmp/.X11-unix/X{number}")).is_ok()
}

/// Windows has no X server and no unix sockets to look for one on. The answer is never
/// used there — `WAYLAND_DISPLAY` is not set either, so the choice never reaches it — but
/// the function has to exist for the code around it to compile.
#[cfg(not(unix))]
fn x11_is_reachable() -> bool {
    false
}

/// The rule behind [`choose_backend`], kept apart from the environment so it can be tested.
fn backend_for(wayland: bool, x11_reachable: bool, forced: Option<&str>) -> Backend {
    match forced {
        Some("wayland") => Backend::Wayland,
        Some("x11") => Backend::X11,
        _ if wayland && !x11_reachable => Backend::Wayland,
        _ => Backend::X11,
    }
}

/// The backend to ask winit for, or `None` where winit's own choice is already right.
///
/// Only a Wayland session needs the choice made for it: winit would pick Wayland even
/// with XWayland running. Everywhere else forcing a backend can only go wrong.
pub fn backend_to_force() -> Option<Backend> {
    std::env::var_os("WAYLAND_DISPLAY")
        .is_some()
        .then(choose_backend)
}

/// What to tell the user when the app really is running on Wayland.
pub const WAYLAND_NOTICE: &str = "Wayland cannot keep a window above the others, and XWayland — which can — is not \
     running. Use your compositor's own rule; see the README.";

/// Whether this environment gets a window frame unless the user says otherwise.
///
/// A frameless window has to be placed and moved by the app, which is exactly what native
/// Wayland refuses to do. Everywhere else, including under XWayland, the bar is frameless.
pub fn prefers_decorations() -> bool {
    prefers_decorations_on(choose_backend())
}

/// [`prefers_decorations`] for a backend that has already been chosen, which the startup
/// path needs: it may have to try one backend and then the other.
pub fn prefers_decorations_on(backend: Backend) -> bool {
    backend == Backend::Wayland
}

/// Pins the dark look, whatever the desktop's own theme is.
///
/// egui follows the system theme by default and re-applies it every frame, which on a
/// light desktop left the bar's own dark frame filled with light widgets: white buttons
/// carrying near-white labels, and a white text field with white text. Pinning the
/// preference is what makes the explicit colours below stick.
fn install_theme(ctx: &egui::Context) {
    ctx.set_theme(egui::ThemePreference::Dark);
    // egui refuses to call a press a click once it has lasted longer than this, and it is
    // one global setting, so it cannot serve both a half-second hold and a two-second one.
    // Lifting it hands the decision to `track_press`, which knows which widget was held
    // and for how long. Dragging still works: a drag is what happens once the pointer has
    // moved further than a click may.
    ctx.options_mut(|options| options.input_options.max_click_duration = f64::INFINITY);

    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = theme::BACKGROUND;
    visuals.window_fill = theme::BACKGROUND;
    // The background behind a text field, which is what the rename editor sits on.
    visuals.extreme_bg_color = theme::FIELD;
    visuals.widgets.noninteractive.bg_fill = theme::BACKGROUND;
    visuals.widgets.inactive.bg_fill = theme::BUTTON_IDLE;
    visuals.widgets.inactive.weak_bg_fill = theme::BUTTON_IDLE;
    visuals.widgets.hovered.bg_fill = theme::BUTTON_HOVER;
    visuals.widgets.hovered.weak_bg_fill = theme::BUTTON_HOVER;
    visuals.widgets.active.bg_fill = theme::BUTTON_ACTIVE;
    visuals.widgets.active.weak_bg_fill = theme::BUTTON_ACTIVE;
    visuals.selection.bg_fill = theme::BUTTON_ACTIVE;
    visuals.override_text_color = None;

    ctx.set_visuals_of(egui::Theme::Dark, visuals.clone());
    ctx.set_visuals_of(egui::Theme::Light, visuals);
}

/// How often state is written even when nothing changed, so a crash costs little.
const SAVE_INTERVAL: Duration = Duration::from_secs(30);

pub struct WipTracker {
    tracker: Tracker,
    show_duration: bool,
    /// `None` until the user chooses; see [`prefers_decorations`].
    decorated: Option<bool>,
    store: Option<Box<dyn Store>>,
    /// Set when the store could not be read: the app then refuses to write over it.
    fatal: Option<String>,
    dirty: bool,
    last_save: Instant,
    /// The window's outer top-left, persisted so the bar reopens where it was.
    window_pos: Option<(f32, f32)>,
    /// The whole outer rect, frame included — what the menu and the hint place against.
    /// Not persisted; refreshed every frame.
    window_rect: Option<egui::Rect>,
    menu_open: bool,
    menu_was_focused: bool,
    /// When the menu last closed because the focus went somewhere else. Clicking the
    /// burger while the menu is open does exactly that a frame before the click lands, so
    /// without this the click would reopen what it was meant to close.
    menu_dismissed_at: Option<f64>,
    /// A short message shown in the menu, such as "restart to apply".
    notice: Option<String>,
    /// Sounds when a task's daily timer runs out. `None` in tests that want silence.
    alarm: Option<Box<dyn Alarm>>,
    /// Knows how long the user has been away. `None` in tests that fake it directly.
    idle_probe: Option<Box<dyn IdleProbe>>,
    /// The tasks whose alarm sounded this session, kept for the tests to inspect.
    alarms_sounded: Vec<TaskId>,
    /// How often the day alarm sounded this session, kept for the tests to inspect.
    day_alarms_sounded: usize,
    /// Whether the offer to add WipTracker to the application menu was declined for good.
    launcher_offer_dismissed: bool,
    /// Whether WipTracker starts with the session, read once at startup; `None` where
    /// the platform will not say, which disables the menu toggle.
    autostart: Option<bool>,
    /// When the previous frame ran. A hole in the frame stream is a suspend: the span is
    /// skipped rather than credited, or a closed lid would hand the task the whole night.
    last_frame: Option<DateTime<Local>>,
    /// Where the launcher entry would be installed. Settable so tests write to scratch.
    launcher_data_home: Option<std::path::PathBuf>,
    /// Overrides where the autostart entry goes, so tests never touch the real config.
    autostart_config_home: Option<std::path::PathBuf>,
    rename: Option<Rename>,
    windows: OpenWindows,
}

struct Rename {
    id: TaskId,
    text: String,
    focus_requested: bool,
}

impl WipTracker {
    /// The app as the binary starts it: reading from, and writing to, `store`.
    pub fn start(
        cc: &eframe::CreationContext<'_>,
        store: Box<dyn Store>,
        snapshot: Option<Snapshot>,
    ) -> Self {
        let now = Local::now();
        let mut app = match snapshot {
            Some(snapshot) => {
                let tracker = Tracker::from_snapshot(&snapshot, now);
                let mut app = Self::with_tracker(cc, tracker);
                app.show_duration = snapshot.show_duration;
                app.decorated = snapshot.decorated;
                app.window_pos = snapshot.window_pos;
                app.launcher_offer_dismissed = snapshot.launcher_offer_dismissed;
                app
            }
            None => Self::with_tracker(cc, Tracker::new(now)),
        };
        app.store = Some(store);
        app.alarm = Some(Box::new(Beeper::new()));
        app.idle_probe = Some(Box::new(SystemIdle));
        app.autostart = launcher::autostart_enabled();
        // Only Linux menus lose track of a binary outside the system prefixes; macOS has
        // the bundle and Windows the Start-menu shortcut. Asked once, at most — and never
        // where viewports cannot be real windows (the test harness, the web build): there
        // the offer would be drawn over the bar and swallow its clicks, and the answer
        // would depend on whatever machine the tests happen to run on.
        if cfg!(target_os = "linux")
            && !app.launcher_offer_dismissed
            && !cc.egui_ctx.embed_viewports()
            && !launcher::is_visible()
        {
            app.windows.launcher_offer = true;
        }
        app
    }

    /// The app in a state where it can only report why it will not run.
    pub fn broken(cc: &eframe::CreationContext<'_>, error: &StoreError) -> Self {
        let mut app = Self::with_tracker(cc, Tracker::new(Local::now()));
        app.fatal = Some(error.to_string());
        app
    }

    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        Self::with_tracker(cc, Tracker::new(Local::now()))
    }

    /// Builds the app around a prepared tracker. Used by tests to start from a known state.
    pub fn with_tracker(cc: &eframe::CreationContext<'_>, tracker: Tracker) -> Self {
        install_theme(&cc.egui_ctx);

        Self {
            tracker,
            show_duration: true,
            decorated: None,
            store: None,
            fatal: None,
            dirty: false,
            last_save: Instant::now(),
            window_pos: None,
            window_rect: None,
            menu_open: false,
            menu_was_focused: false,
            menu_dismissed_at: None,
            notice: None,
            alarm: None,
            idle_probe: None,
            alarms_sounded: Vec::new(),
            day_alarms_sounded: 0,
            launcher_offer_dismissed: false,
            autostart: None,
            last_frame: None,
            launcher_data_home: launcher::data_home(),
            autostart_config_home: None,
            rename: None,
            windows: OpenWindows::default(),
        }
    }

    pub fn tracker(&self) -> &Tracker {
        &self.tracker
    }

    pub fn tracker_mut(&mut self) -> &mut Tracker {
        &mut self.tracker
    }

    pub fn set_show_duration(&mut self, show: bool) {
        self.show_duration = show;
    }

    /// Whether the window currently wears its window manager's frame.
    pub fn is_decorated(&self) -> bool {
        self.decorated.unwrap_or_else(prefers_decorations)
    }

    pub fn windows(&self) -> &OpenWindows {
        &self.windows
    }

    pub fn windows_mut(&mut self) -> &mut OpenWindows {
        &mut self.windows
    }

    pub fn set_menu_open(&mut self, open: bool) {
        self.menu_open = open;
        self.menu_was_focused = false;
        self.menu_dismissed_at = None;
    }

    pub fn is_menu_open(&self) -> bool {
        self.menu_open
    }

    /// Replaces the alarm, so a test can hear it without a sound card.
    pub fn set_alarm(&mut self, alarm: Box<dyn Alarm>) {
        self.alarm = Some(alarm);
    }

    /// Replaces the idle probe, so a test can be idle without waiting.
    pub fn set_idle_probe(&mut self, probe: Box<dyn IdleProbe>) {
        self.idle_probe = Some(probe);
    }

    /// The tasks whose daily timer has gone off since the app started.
    pub fn alarms_sounded(&self) -> &[TaskId] {
        &self.alarms_sounded
    }

    /// How often the day alarm has sounded since the app started.
    pub fn day_alarms_sounded(&self) -> usize {
        self.day_alarms_sounded
    }

    pub fn is_renaming(&self) -> bool {
        self.rename.is_some()
    }

    /// Writes the state out if anything changed, or if the save interval has elapsed.
    fn maybe_save(&mut self) {
        if self.fatal.is_some() {
            return;
        }
        if !self.dirty && self.last_save.elapsed() < SAVE_INTERVAL {
            return;
        }
        let Some(store) = &self.store else {
            self.dirty = false;
            return;
        };
        let mut snapshot =
            self.tracker
                .snapshot(self.show_duration, self.decorated, self.window_pos);
        snapshot.launcher_offer_dismissed = self.launcher_offer_dismissed;
        match store.save(&snapshot) {
            Ok(()) => {
                self.dirty = false;
                self.last_save = Instant::now();
            }
            Err(error) => self.fatal = Some(error.to_string()),
        }
    }

    /// Today's time on the focused task, what to say about it on hover, and which timer —
    /// task or whole day — has been passed. The day outranks the task: red beats amber.
    fn clock_parts(&self, now: DateTime<Local>) -> Option<(String, String, bar::Over)> {
        let task = self.tracker.focused()?;
        let day = now.date_naive();
        let today = self.tracker.duration_on(task.id, day);
        let mut tooltip = format!(
            "Today: {}\nAll time: {}",
            format::clock(today),
            format::clock(task.total)
        );
        let over_limit = task.has_timer() && today >= task.timer;
        if task.has_timer() {
            tooltip.push_str(&format!("\nDaily timer: {}", format::coarse(task.timer)));
            if over_limit {
                tooltip.push_str(" — reached");
            }
        }
        let day_timer = self.tracker.day_timer();
        if !day_timer.is_zero() {
            tooltip.push_str(&format!(
                "\nDay: {} of {}",
                format::coarse(self.tracker.worked_on(day)),
                format::coarse(day_timer)
            ));
        }
        let over = if self.tracker.day_over(day) {
            tooltip.push_str(" — the day is over");
            bar::Over::Day
        } else if over_limit {
            bar::Over::Task
        } else {
            bar::Over::None
        };
        Some((format::clock(today), tooltip, over))
    }

    fn remember_window_pos(&mut self, ctx: &egui::Context) {
        // Wayland never reports a window position; there the bar simply opens where the
        // compositor puts it.
        let Some(rect) = ctx.input(|i| i.viewport().outer_rect) else {
            return;
        };
        self.window_rect = Some(rect);
        // Deliberately not marked dirty: a drag would otherwise commit a transaction per
        // frame. The periodic save and the save on exit pick the position up.
        self.window_pos = Some((rect.min.x, rect.min.y));
    }

    /// Writes the launcher entry and icons, and says how it went in the menu's notice.
    fn install_launcher(&mut self) {
        let Some(data_home) = self.launcher_data_home.clone() else {
            self.notice = Some("No home directory to install into.".to_owned());
            return;
        };
        let exe = std::env::current_exe().unwrap_or_default();
        match launcher::install_into(&data_home, &exe) {
            Ok(()) => {
                launcher::refresh_caches(&data_home);
                self.notice = Some("Added to the application menu.".to_owned());
            }
            Err(error) => self.notice = Some(format!("Could not add the entry: {error}")),
        }
    }

    /// Points the installer somewhere else, so a test can watch it write.
    pub fn set_launcher_data_home(&mut self, path: std::path::PathBuf) {
        self.launcher_data_home = Some(path);
    }

    /// Points the autostart switch somewhere else, and arms the toggle, for tests.
    pub fn set_autostart_config_home(&mut self, path: std::path::PathBuf) {
        self.autostart = Some(path.join("autostart/wiptracker.desktop").exists());
        self.autostart_config_home = Some(path);
    }

    /// Begins renaming the focused task, as a right-click on its name does.
    pub fn start_rename(&mut self) {
        let id = self.tracker.focused_id();
        let Some(task) = self.tracker.task(id) else {
            return;
        };
        if task.is_pause() {
            return;
        }
        self.rename = Some(Rename {
            id,
            text: task.name.clone(),
            focus_requested: false,
        });
    }

    /// Starts a task and opens its name for editing.
    ///
    /// A fresh task is called "new task 7"; the point of creating it is to say what it
    /// really is, so the name is immediately open for editing with the placeholder
    /// selected.
    fn add_task(&mut self, now: DateTime<Local>) {
        self.tracker.push_new_task(now);
        self.start_rename();
        self.dirty = true;
    }

    fn finish_task(&mut self, now: DateTime<Local>) {
        self.tracker.finish_focused(now);
        self.rename = None;
        self.dirty = true;
    }

    fn switch_to_pause(&mut self, now: DateTime<Local>) {
        let _ = self.tracker.select(PAUSE_ID, now);
        self.dirty = true;
    }

    /// How long a burger click is ignored after the menu closed by losing the focus.
    const REOPEN_GUARD: f64 = 0.25;

    fn toggle_menu(&mut self, time: f64) {
        // The press that reaches the burger is the same press that took the focus off the
        // menu window, which already closed it. Reopening here would make the burger look
        // like it only ever opens the menu.
        if !self.menu_open
            && self
                .menu_dismissed_at
                .is_some_and(|dismissed| time - dismissed < Self::REOPEN_GUARD)
        {
            self.menu_dismissed_at = None;
            return;
        }
        self.menu_open = !self.menu_open;
        self.menu_was_focused = false;
        self.menu_dismissed_at = None;
    }

    fn apply_bar_action(&mut self, action: BarAction, now: DateTime<Local>, time: f64) {
        match action {
            BarAction::AddTask => self.add_task(now),
            BarAction::FinishTask => self.finish_task(now),
            BarAction::ToggleMenu => self.toggle_menu(time),
            BarAction::StartRename => self.start_rename(),
            BarAction::OpenStack => self.windows.stack = true,
            BarAction::SwitchToPause => self.switch_to_pause(now),
            BarAction::OpenRevive => self.windows.revive = true,
            BarAction::OpenTimer => self.windows.timer = true,
            BarAction::None => {}
        }
    }

    fn apply_menu_action(&mut self, action: MenuAction, now: DateTime<Local>) {
        match action {
            MenuAction::NewTask => self.add_task(now),
            MenuAction::Rename => self.start_rename(),
            MenuAction::Finish => self.finish_task(now),
            MenuAction::Pause => self.switch_to_pause(now),
            MenuAction::OpenStack => self.windows.stack = true,
            MenuAction::OpenTimer => self.windows.timer = true,
            MenuAction::ToggleDuration => {
                self.show_duration = !self.show_duration;
                self.dirty = true;
            }
            MenuAction::ToggleDecorations => {
                // Changing this on a live window leaves the frame drawn over the bar, so
                // the preference is only stored; the window is built from it at startup.
                self.decorated = Some(!self.is_decorated());
                self.notice = Some(if self.is_decorated() {
                    "Restart WipTracker to get the window frame.".to_owned()
                } else {
                    "Restart WipTracker to drop the window frame.".to_owned()
                });
                self.dirty = true;
            }
            MenuAction::ToggleAutostart => {
                let wanted = self.autostart != Some(true);
                let exe = std::env::current_exe().unwrap_or_default();
                let switched = match &self.autostart_config_home {
                    Some(config) => launcher::set_autostart_in(config, wanted, &exe),
                    None => launcher::set_autostart(wanted, &exe),
                };
                match switched {
                    Ok(()) => {
                        self.autostart = Some(wanted);
                        self.notice = Some(if wanted {
                            "WipTracker starts with your session now.".to_owned()
                        } else {
                            "WipTracker no longer starts with your session.".to_owned()
                        });
                    }
                    Err(error) => self.notice = Some(format!("Could not change that: {error}")),
                }
            }
            MenuAction::ToggleNag => {
                let day = now.date_naive();
                let muted = self.tracker.nag_muted(day);
                self.tracker.set_nag_muted(day, !muted);
                self.dirty = true;
            }
            MenuAction::OpenGroom => self.windows.groom = true,
            MenuAction::OpenEndDay => self.windows.end_day = true,
            MenuAction::OpenWeek => self.windows.week = true,
            MenuAction::OpenRevive => self.windows.revive = true,
            MenuAction::None => {}
        }
    }

    fn show_rename_editor(&mut self, ui: &mut egui::Ui) -> bool {
        let Some(rename) = &mut self.rename else {
            return false;
        };
        let id = egui::Id::new("rename_editor");
        // Both colours are named here rather than left to the theme. The field's
        // background otherwise comes from `visuals.extreme_bg_color`, and a mac user saw
        // white text in a white field — whatever reaches the style on that platform, an
        // explicit colour cannot be overridden by it.
        let output = egui::TextEdit::singleline(&mut rename.text)
            .id(id)
            .text_color(theme::TEXT)
            .background_color(theme::FIELD)
            .desired_width(theme::LABEL_WIDTH)
            .show(ui);
        let response = output.response;
        if !rename.focus_requested {
            response.request_focus();
            // Select the whole placeholder, so the first keystroke replaces it.
            let mut state = output.state;
            state
                .cursor
                .set_char_range(Some(egui::text::CCursorRange::two(
                    egui::text::CCursor::new(0),
                    egui::text::CCursor::new(rename.text.chars().count()),
                )));
            state.store(ui.ctx(), id);
            rename.focus_requested = true;
        }

        let cancel = ui.input(|i| i.key_pressed(egui::Key::Escape));
        let commit = response.lost_focus() && !cancel;
        if cancel {
            self.rename = None;
        } else if commit && let Some(rename) = self.rename.take() {
            let _ = self.tracker.rename(rename.id, &rename.text);
            self.dirty = true;
        }
        true
    }
}

impl eframe::App for WipTracker {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        let now = Local::now();

        if let Some(error) = self.fatal.clone() {
            egui::CentralPanel::default()
                .frame(bar::frame())
                .show(ui, |ui| bar::show_error(ui, &error));
            return;
        }

        // The desktop can switch to a light theme while the app runs, and egui follows it
        // by default, so the preference is re-asserted rather than set once at startup.
        //
        // The palette is checked as well as the preference. Something that replaces the
        // style without touching the preference would leave the app light and the older
        // guard would never have noticed — which is the shape of the mac report that has
        // not been explained, so the app repairs itself instead of trusting one flag.
        if ctx.options(|options| options.theme_preference) != egui::ThemePreference::Dark
            || ctx.style_of(egui::Theme::Dark).visuals.extreme_bg_color != theme::FIELD
        {
            install_theme(&ctx);
        }

        // The first frame after a start is the restart-recovery span and stays credited;
        // after that, a hole in the once-a-second frame stream is a suspend.
        if self
            .last_frame
            .is_some_and(|last| now - last > Tracker::CONTINUOUS_GAP)
        {
            self.tracker.skip_to(now);
        }
        self.last_frame = Some(now);

        self.tracker.accrue(now);
        if !self.tracker.idle_pause().is_zero()
            && let Some(idle) = self.idle_probe.as_ref().and_then(|probe| probe.idle())
            && self.tracker.pause_after_idle(idle, now)
        {
            self.dirty = true;
        }
        for id in self.tracker.take_due_alarms(now) {
            self.alarms_sounded.push(id);
            if let Some(alarm) = &self.alarm {
                let name = self.tracker.task(id).map(|task| task.name.as_str());
                alarm.sound(name.unwrap_or("a task"));
            }
            self.dirty = true;
        }
        if self.tracker.take_due_day_alarm(now) {
            self.day_alarms_sounded += 1;
            if let Some(alarm) = &self.alarm {
                alarm.sound_day_over();
            }
            self.dirty = true;
        }
        ctx.request_repaint_after(Duration::from_secs(1));
        self.remember_window_pos(&ctx);

        let name = self.tracker.focused_name().to_owned();
        // The clock shows today's time so that the number agrees with the task's timer and
        // with the end-day report; the all-time total lives in the tooltip.
        let clock_parts = self.show_duration.then(|| self.clock_parts(now)).flatten();
        let clock = clock_parts
            .as_ref()
            .map(|(today, tooltip, over)| bar::Clock {
                today,
                tooltip,
                over: *over,
            });

        let can_revive = !self
            .tracker
            .recently_finished(windows::REVIVE_DAYS, now)
            .is_empty();
        let state = bar::BarState {
            clock,
            decorated: self.is_decorated(),
            can_revive,
        };
        let time = ctx.input(|input| input.time);
        let placement = crate::ui::place::Placement {
            bar: self.window_rect,
            monitor: ctx.input(|input| input.viewport().monitor_size),
        };

        let mut outcome = bar::BarOutcome::default();
        let mut renaming = false;
        egui::CentralPanel::default()
            .frame(bar::frame())
            .show(ui, |ui| {
                if self.rename.is_some() {
                    renaming = true;
                    outcome = bar::show_with_editor(ui, state, |ui| {
                        self.show_rename_editor(ui);
                    });
                } else {
                    outcome = bar::show(ui, &name, state);
                }
            });
        let _ = renaming;
        self.apply_bar_action(outcome.action, now, time);

        // The hint window only exists while there is something to explain, so it appears
        // and disappears with the pointer.
        if let Some(hint) = &outcome.hint {
            hint::show(&ctx, hint, &placement);
        }

        if self.menu_open {
            let outcome = menu::show(
                &ctx,
                &menu::MenuContext {
                    can_revive,
                    paused: self.tracker.focused_id() == PAUSE_ID,
                    show_duration: self.show_duration,
                    decorated: self.is_decorated(),
                    day_timer_set: !self.tracker.day_timer().is_zero(),
                    nag_muted: self.tracker.nag_muted(now.date_naive()),
                    autostart: self.autostart,
                    notice: self.notice.as_deref(),
                    placement,
                    platform_notice: (choose_backend() == Backend::Wayland)
                        .then_some(WAYLAND_NOTICE),
                    was_focused: self.menu_was_focused,
                },
            );
            self.menu_open = outcome.keep_open;
            if !self.menu_open {
                self.notice = None;
                // Only a dismissal by focus loss can be undone by the burger click that
                // caused it; picking an entry closes the menu on purpose.
                if outcome.action == MenuAction::None {
                    self.menu_dismissed_at = Some(time);
                }
            }
            self.menu_was_focused = outcome.was_focused;
            self.apply_menu_action(outcome.action, now);
        }

        if self.windows.launcher_offer {
            match windows::launcher_offer(&ctx, &mut self.windows) {
                windows::LauncherChoice::Install => {
                    self.install_launcher();
                    self.launcher_offer_dismissed = true;
                    self.dirty = true;
                }
                windows::LauncherChoice::Dismiss => {
                    self.launcher_offer_dismissed = true;
                    self.dirty = true;
                }
                windows::LauncherChoice::None => {}
            }
        }

        let outcome = windows::show_all(&ctx, &mut self.windows, &mut self.tracker, now);
        if outcome.changed {
            self.dirty = true;
        }
        // Copying from the root context, not from inside the report window: an immediate
        // viewport's output never reaches the platform, so the clipboard would stay empty.
        if let Some(json) = outcome.copy {
            ctx.copy_text(json);
        }

        self.maybe_save();

        // Closing the day is the end of the session: there is nothing left for the bar to
        // show, and a clock left running overnight would credit the wrong day.
        if outcome.day_closed {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }

    fn on_exit(&mut self) {
        self.dirty = true;
        self.maybe_save();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// XWayland is what makes always-on-top work again, so it wins whenever it answers.
    #[test]
    fn a_wayland_session_with_xwayland_uses_x11() {
        assert_eq!(backend_for(true, true, None), Backend::X11);
    }

    /// A `DISPLAY` that nothing answers on counts as no X11 at all: asking winit for it
    /// would end the process rather than degrade.
    #[test]
    fn a_wayland_session_without_xwayland_has_no_choice() {
        assert_eq!(backend_for(true, false, None), Backend::Wayland);
        assert!(prefers_decorations_on(Backend::Wayland));
        assert!(!prefers_decorations_on(Backend::X11));
    }

    #[test]
    fn a_plain_x11_session_uses_x11() {
        assert_eq!(backend_for(false, true, None), Backend::X11);
    }

    #[test]
    fn the_override_wins_either_way() {
        assert_eq!(backend_for(true, true, Some("wayland")), Backend::Wayland);
        assert_eq!(backend_for(true, false, Some("x11")), Backend::X11);
        // Anything else is ignored rather than refused.
        assert_eq!(backend_for(true, true, Some("nonsense")), Backend::X11);
    }
}
