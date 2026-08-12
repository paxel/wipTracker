//! The always-on-top one-line bar.

use egui::{Align, Frame, Label, Layout, Margin, Response, RichText, Sense, Stroke, Ui};

use crate::theme;
use crate::ui::widgets::{Icon, Press, grip, icon_button, sweep, tooltip, track_press};

/// What the user asked for by interacting with the bar this frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BarAction {
    #[default]
    None,
    AddTask,
    FinishTask,
    ToggleMenu,
    StartRename,
    /// Show the task stack so another task can be picked.
    OpenStack,
    /// Straight to a break.
    SwitchToPause,
    /// Put a finished task back on the stack.
    OpenRevive,
    /// Set the focused task's daily timer.
    OpenTimer,
}

/// How long the destructive hold takes. Finishing a task has to be meant.
pub const HOLD_FINISH: f32 = 2.0;
/// How long every other hold takes.
pub const HOLD_QUICK: f32 = 0.5;

const NAME_TOOLTIP: &str = "Click: rename this task\nHold for 2 seconds: finish it, or end \
                            the break when paused";
const FORK_TOOLTIP: &str = "Click: show the task stack\nHold: take a break";
const PLUS_TOOLTIP: &str = "Click: start a new task\nHold: put a finished task back";
const MENU_TOOLTIP: &str = "Click: open or close the menu\nHold: set the daily timer";
const GRIP_TOOLTIP: &str = "Drag to move the bar";
/// What the clock shows, and what to say about it on hover.
#[derive(Clone, Copy)]
pub struct Clock<'a> {
    /// Today's time on this task, already formatted.
    pub today: &'a str,
    /// The hover text: the all-time total, and the timer if there is one.
    pub tooltip: &'a str,
    /// Whether today's time has passed the task's daily timer.
    pub over_limit: bool,
}

/// How the bar is drawn this frame.
#[derive(Clone, Copy)]
pub struct BarState<'a> {
    pub clock: Option<Clock<'a>>,
    /// Whether the window wears its window manager's frame, which decides whether the
    /// grip is drawn and therefore how wide the bar is.
    pub decorated: bool,
    /// Whether there is a finished task that could be put back on the stack.
    pub can_revive: bool,
}

/// Hands the drag to the window manager, which then owns it completely.
///
/// Only the native gesture: an earlier version also moved the window by hand on every
/// frame the response reported a drag, as a fallback for desktops that ignore the gesture.
/// That made the bar stick to the pointer — once the window manager takes a pointer grab,
/// the button release never reaches egui, so the response keeps reporting a drag and the
/// manual path keeps moving the window until the next click. Desktops that ignore the
/// gesture are Wayland compositors, and those already get a window frame by default, which
/// is how the bar is dragged there.
fn drag_window(ui: &Ui, response: &Response) {
    if response.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
}

pub fn frame() -> Frame {
    Frame::new()
        .fill(theme::BACKGROUND)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .inner_margin(Margin::symmetric(theme::BAR_MARGIN as i8, 0))
}

/// Draws the bar with the task name replaced by a caller-supplied editor.
pub fn show_with_editor(
    ui: &mut Ui,
    state: BarState<'_>,
    editor: impl FnOnce(&mut Ui),
) -> BarAction {
    show_inner(ui, None, state, Some(editor))
}

/// Replaces the whole bar with a message explaining why the app will not run.
pub fn show_error(ui: &mut Ui, message: &str) {
    ui.horizontal_centered(|ui| {
        ui.add(
            Label::new(RichText::new(message).color(egui::Color32::from_rgb(0xE8, 0x7A, 0x7A)))
                .truncate(),
        )
        .on_hover_text(message);
    });
}

pub fn show(ui: &mut Ui, name: &str, state: BarState<'_>) -> BarAction {
    show_inner(ui, Some(name), state, None::<fn(&mut Ui)>)
}

/// Turns what a press produced into the action it stands for.
fn action_of(press: Press, click: BarAction, hold: BarAction) -> Option<BarAction> {
    if press.long_pressed {
        Some(hold)
    } else if press.clicked {
        Some(click)
    } else {
        None
    }
}

/// Draws the task name, and reports what holding or clicking it asked for.
///
/// One widget, not a label stacked on an invisible button: a second widget over the same
/// rectangle takes the pointer for itself, and the hold then never starts. That leaves the
/// sweep to be painted over the text rather than under it, which is what
/// [`theme::HOLD_FILL_OVER`] is for.
fn show_name(ui: &mut Ui, name: &str) -> Option<BarAction> {
    let response = ui.add(
        Label::new(RichText::new(name).color(theme::TEXT))
            .truncate()
            .selectable(false)
            .sense(Sense::click()),
    );
    let press = track_press(ui, &response, HOLD_FINISH, true);
    sweep(ui, response.rect, press.progress, theme::HOLD_FILL_OVER);
    tooltip(response, format!("{name}\n\n{NAME_TOOLTIP}"));
    action_of(press, BarAction::StartRename, BarAction::FinishTask)
}

/// Draws the bar contents and reports what the user did.
///
/// The row is allocated at an exact size rather than filling the parent, so the layout is
/// identical whether the parent is the real window or an unbounded test harness.
fn show_inner(
    ui: &mut Ui,
    name: Option<&str>,
    state: BarState<'_>,
    editor: Option<impl FnOnce(&mut Ui)>,
) -> BarAction {
    let mut action = BarAction::None;

    // The window has a fixed size, so the row does too: relying on the parent's available
    // space would make the layout depend on how the bar happens to be embedded.
    let bar = theme::bar_size(state.decorated);
    let width = bar.x - 2.0 * theme::BAR_MARGIN;
    let height = bar.y;
    ui.spacing_mut().item_spacing.x = 2.0;

    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        Layout::left_to_right(Align::Center),
        |ui| {
            let clock_width = if state.clock.is_some() {
                theme::CLOCK_WIDTH
            } else {
                0.0
            };
            let label_width = (width
                - theme::grip_width(state.decorated)
                - clock_width
                - theme::BUTTON_COUNT * theme::BUTTON_SIZE.x
                - 6.0)
                .max(0.0);

            // The grip is the only drag surface. Nothing else on the bar moves the window:
            // a stray press on the background used to drag it, and the name cannot both be
            // dragged and held for two seconds.
            if !state.decorated {
                let handle = tooltip(grip(ui), GRIP_TOOLTIP);
                drag_window(ui, &handle);
            }

            ui.allocate_ui_with_layout(
                egui::vec2(label_width, height),
                Layout::left_to_right(Align::Center),
                |ui| {
                    if let Some(editor) = editor {
                        editor(ui);
                    } else if let Some(name) = name
                        && let Some(asked) = show_name(ui, name)
                    {
                        action = asked;
                    }
                },
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                let (burger, press) = icon_button(ui, Icon::Burger, HOLD_QUICK, true);
                tooltip(burger, MENU_TOOLTIP);
                if let Some(asked) = action_of(press, BarAction::ToggleMenu, BarAction::OpenTimer) {
                    action = asked;
                }

                // Nothing to revive means no sweep: a hold that fills up and then does
                // nothing reads as a broken button.
                let (plus, press) = icon_button(ui, Icon::Plus, HOLD_QUICK, state.can_revive);
                tooltip(plus, PLUS_TOOLTIP);
                if let Some(asked) = action_of(press, BarAction::AddTask, BarAction::OpenRevive) {
                    action = asked;
                }

                let (fork, press) = icon_button(ui, Icon::Fork, HOLD_QUICK, true);
                tooltip(fork, FORK_TOOLTIP);
                if let Some(asked) =
                    action_of(press, BarAction::OpenStack, BarAction::SwitchToPause)
                {
                    action = asked;
                }

                if let Some(clock) = state.clock {
                    let color = if clock.over_limit {
                        theme::OVER_LIMIT
                    } else {
                        theme::TEXT_DIM
                    };
                    tooltip(
                        ui.add(
                            Label::new(RichText::new(clock.today).color(color).monospace())
                                .selectable(false),
                        ),
                        clock.tooltip,
                    );
                }
            });
        },
    );

    action
}
