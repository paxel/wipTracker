//! The eframe application: owns the state and drives the bar and its windows.

use std::time::{Duration, Instant};

use chrono::{DateTime, Local};

use crate::domain::ports::{Snapshot, Store, StoreError};
use crate::domain::task::{PAUSE_ID, TaskId};
use crate::domain::tracker::Tracker;
use crate::theme;
use crate::ui::bar::{self, BarAction};
use crate::ui::format;
use crate::ui::menu::{self, MenuAction};
use crate::ui::windows::{self, OpenWindows};

/// Whether this environment gets a window frame unless the user says otherwise.
///
/// An undecorated window can only be moved if the environment honours the window
/// manager's move gesture. Wayland compositors frequently do not, and there is no way to
/// move the window by hand there either, so a frame is the only way out. X11, macOS and
/// Windows all behave, and get the clean frameless bar.
pub fn prefers_decorations() -> bool {
    std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("DISPLAY").is_none()
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
    window_pos: Option<(f32, f32)>,
    menu_open: bool,
    menu_was_focused: bool,
    /// Set when the frame preference changed and the window has yet to be told.
    decorations_pending: bool,
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
                app
            }
            None => Self::with_tracker(cc, Tracker::new(now)),
        };
        app.store = Some(store);
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
        let mut visuals = egui::Visuals::dark();
        visuals.panel_fill = theme::BACKGROUND;
        visuals.window_fill = theme::BACKGROUND;
        visuals.widgets.hovered.bg_fill = theme::BUTTON_HOVER;
        visuals.widgets.hovered.weak_bg_fill = theme::BUTTON_HOVER;
        cc.egui_ctx.set_visuals(visuals);

        Self {
            tracker,
            show_duration: true,
            decorated: None,
            store: None,
            fatal: None,
            dirty: false,
            last_save: Instant::now(),
            window_pos: None,
            menu_open: false,
            menu_was_focused: false,
            decorations_pending: false,
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
    }

    pub fn is_menu_open(&self) -> bool {
        self.menu_open
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
        let snapshot = self
            .tracker
            .snapshot(self.show_duration, self.decorated, self.window_pos);
        match store.save(&snapshot) {
            Ok(()) => {
                self.dirty = false;
                self.last_save = Instant::now();
            }
            Err(error) => self.fatal = Some(error.to_string()),
        }
    }

    fn remember_window_pos(&mut self, ctx: &egui::Context) {
        // Wayland never reports a window position; there the bar simply opens where the
        // compositor puts it.
        let Some(rect) = ctx.input(|i| i.viewport().outer_rect) else {
            return;
        };
        // Deliberately not marked dirty: a drag would otherwise commit a transaction per
        // frame. The periodic save and the save on exit pick the position up.
        self.window_pos = Some((rect.min.x, rect.min.y));
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

    fn apply_bar_action(&mut self, action: BarAction, now: DateTime<Local>) {
        match action {
            BarAction::AddTask => {
                self.tracker.push_new_task(now);
                self.rename = None;
                self.dirty = true;
            }
            BarAction::FinishTask => {
                self.tracker.finish_focused(now);
                self.rename = None;
                self.dirty = true;
            }
            BarAction::ToggleMenu => {
                self.menu_open = !self.menu_open;
                self.menu_was_focused = false;
            }
            BarAction::StartRename => self.start_rename(),
            BarAction::OpenSelect => {
                self.menu_open = true;
                self.menu_was_focused = false;
            }
            BarAction::SwitchToPause => {
                let _ = self.tracker.select(PAUSE_ID, now);
                self.dirty = true;
            }
            BarAction::None => {}
        }
    }

    fn apply_menu_action(&mut self, action: MenuAction, now: DateTime<Local>) {
        match action {
            MenuAction::Select(id) => {
                let _ = self.tracker.select(id, now);
                self.dirty = true;
            }
            MenuAction::ToggleDuration => {
                self.show_duration = !self.show_duration;
                self.dirty = true;
            }
            MenuAction::ToggleDecorations => {
                self.decorated = Some(!self.is_decorated());
                self.decorations_pending = true;
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
        let response = ui.add_sized(
            egui::vec2(theme::LABEL_WIDTH, theme::BAR_SIZE.y - 8.0),
            egui::TextEdit::singleline(&mut rename.text)
                .text_color(theme::TEXT)
                .desired_width(theme::LABEL_WIDTH),
        );
        if !rename.focus_requested {
            response.request_focus();
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

        self.tracker.accrue(now);
        ctx.request_repaint_after(Duration::from_secs(1));
        self.remember_window_pos(&ctx);
        if self.decorations_pending {
            self.decorations_pending = false;
            ctx.send_viewport_cmd(egui::ViewportCommand::Decorations(self.is_decorated()));
        }

        let name = self.tracker.focused_name().to_owned();
        let clock = self
            .show_duration
            .then(|| self.tracker.focused().map(|task| format::clock(task.total)))
            .flatten();

        let mut action = BarAction::None;
        let mut renaming = false;
        egui::CentralPanel::default()
            .frame(bar::frame())
            .show(ui, |ui| {
                if self.rename.is_some() {
                    renaming = true;
                    action = bar::show_with_editor(ui, clock.as_deref(), |ui| {
                        self.show_rename_editor(ui);
                    });
                } else {
                    action = bar::show(ui, &name, clock.as_deref());
                }
            });
        let _ = renaming;
        self.apply_bar_action(action, now);

        if self.menu_open {
            let outcome = menu::show(
                &ctx,
                &self.tracker,
                self.show_duration,
                self.is_decorated(),
                self.window_pos,
                self.menu_was_focused,
            );
            self.menu_open = outcome.keep_open;
            self.menu_was_focused = outcome.was_focused;
            self.apply_menu_action(outcome.action, now);
        }

        let outcome = windows::show_all(&ctx, &mut self.windows, &mut self.tracker, now);
        if outcome.changed {
            self.dirty = true;
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
