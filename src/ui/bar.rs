//! The always-on-top one-line bar.

use egui::{
    Align, Frame, Label, Layout, Margin, Response, RichText, Sense, Stroke, Ui, Vec2,
    ViewportCommand,
};

use crate::theme;
use crate::ui::widgets::{Icon, grip, icon_button};

/// What the user asked for by interacting with the bar this frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BarAction {
    #[default]
    None,
    AddTask,
    FinishTask,
    ToggleMenu,
    StartRename,
    /// Middle-click on the task name: show the task stack so another task can be picked.
    OpenStack,
    /// Middle-click on `+`: straight to a break.
    SwitchToPause,
}

const NAME_TOOLTIP: &str = "Left click: rename this task\nMiddle click: show the task stack";
const PLUS_TOOLTIP: &str =
    "Left click: start a new task\nRight click: finish this task\nMiddle click: take a break";
const MENU_TOOLTIP: &str = "Left click: open the menu";
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

/// Moves the window while `response` is dragged.
///
/// The first choice is the window manager's own move gesture, which snaps and behaves like
/// dragging any other window. Some environments ignore that request and keep sending drag
/// events instead; when that happens the window is moved by hand, which needs a known
/// window position and so does not work on Wayland.
fn drag_window(ui: &Ui, response: &Response) {
    if response.drag_started() {
        ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
        return;
    }
    if !response.dragged() {
        return;
    }
    let delta = response.drag_delta();
    if delta == Vec2::ZERO {
        return;
    }
    if let Some(rect) = ui.ctx().input(|i| i.viewport().outer_rect) {
        ui.ctx()
            .send_viewport_cmd(ViewportCommand::OuterPosition(rect.min + delta));
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
    clock: Option<Clock<'_>>,
    editor: impl FnOnce(&mut Ui),
) -> BarAction {
    show_inner(ui, None, clock, Some(editor))
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

pub fn show(ui: &mut Ui, name: &str, clock: Option<Clock<'_>>) -> BarAction {
    show_inner(ui, Some(name), clock, None::<fn(&mut Ui)>)
}

/// Draws the bar contents and reports what the user did.
///
/// The row is allocated at an exact size rather than filling the parent, so the layout is
/// identical whether the parent is the real window or an unbounded test harness.
fn show_inner(
    ui: &mut Ui,
    name: Option<&str>,
    clock: Option<Clock<'_>>,
    editor: Option<impl FnOnce(&mut Ui)>,
) -> BarAction {
    let mut action = BarAction::None;

    // The window has a fixed size, so the row does too: relying on the parent's available
    // space would make the layout depend on how the bar happens to be embedded.
    let width = theme::BAR_SIZE.x - 2.0 * theme::BAR_MARGIN;
    let height = theme::BAR_SIZE.y;
    ui.spacing_mut().item_spacing.x = 2.0;

    ui.allocate_ui_with_layout(
        egui::vec2(width, height),
        Layout::left_to_right(Align::Center),
        |ui| {
            let row = ui.max_rect();
            let surface = ui.interact(row, ui.id().with("drag_surface"), Sense::click_and_drag());
            drag_window(ui, &surface);

            let clock_width = if clock.is_some() {
                theme::CLOCK_WIDTH
            } else {
                0.0
            };
            let label_width =
                (width - theme::GRIP_WIDTH - clock_width - 2.0 * theme::BUTTON_SIZE.x - 6.0)
                    .max(0.0);

            let handle = grip(ui).on_hover_text(GRIP_TOOLTIP);
            drag_window(ui, &handle);

            ui.allocate_ui_with_layout(
                egui::vec2(label_width, height),
                Layout::left_to_right(Align::Center),
                |ui| {
                    if let Some(editor) = editor {
                        editor(ui);
                    } else if let Some(name) = name {
                        let response = ui
                            .add(
                                Label::new(RichText::new(name).color(theme::TEXT))
                                    .truncate()
                                    .selectable(false)
                                    .sense(Sense::click_and_drag()),
                            )
                            .on_hover_text(format!("{name}\n\n{NAME_TOOLTIP}"));
                        drag_window(ui, &response);
                        if response.clicked() {
                            action = BarAction::StartRename;
                        } else if response.middle_clicked() {
                            action = BarAction::OpenStack;
                        }
                    }
                },
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, Icon::Burger)
                    .on_hover_text(MENU_TOOLTIP)
                    .clicked()
                {
                    action = BarAction::ToggleMenu;
                }
                let plus = icon_button(ui, Icon::Plus).on_hover_text(PLUS_TOOLTIP);
                if plus.clicked() {
                    action = BarAction::AddTask;
                } else if plus.secondary_clicked() {
                    action = BarAction::FinishTask;
                } else if plus.middle_clicked() {
                    action = BarAction::SwitchToPause;
                }

                if let Some(clock) = clock {
                    let color = if clock.over_limit {
                        theme::OVER_LIMIT
                    } else {
                        theme::TEXT_DIM
                    };
                    ui.add(
                        Label::new(RichText::new(clock.today).color(color).monospace())
                            .selectable(false),
                    )
                    .on_hover_text(clock.tooltip);
                }
            });
        },
    );

    action
}
