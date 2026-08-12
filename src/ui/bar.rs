//! The always-on-top one-line bar.

use egui::{Align, Frame, Label, Layout, Margin, RichText, Sense, Stroke, Ui, ViewportCommand};

use crate::theme;
use crate::ui::widgets::{Icon, icon_button};

/// What the user asked for by interacting with the bar this frame.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum BarAction {
    #[default]
    None,
    AddTask,
    FinishTask,
    ToggleMenu,
    StartRename,
}

pub fn frame() -> Frame {
    Frame::new()
        .fill(theme::BACKGROUND)
        .stroke(Stroke::new(1.0, theme::BORDER))
        .inner_margin(Margin::symmetric(theme::BAR_MARGIN as i8, 0))
}

/// Draws the bar contents and reports what the user did.
///
/// The row is allocated at an exact size rather than filling the parent, so the layout is
/// identical whether the parent is the real window or an unbounded test harness.
/// Draws the bar with the task name replaced by a caller-supplied editor.
pub fn show_with_editor(
    ui: &mut Ui,
    clock: Option<&str>,
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

pub fn show(ui: &mut Ui, name: &str, clock: Option<&str>) -> BarAction {
    show_inner(ui, Some(name), clock, None::<fn(&mut Ui)>)
}

fn show_inner(
    ui: &mut Ui,
    name: Option<&str>,
    clock: Option<&str>,
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
            let drag = ui.interact(row, ui.id().with("drag_surface"), Sense::click_and_drag());
            if drag.drag_started() {
                ui.ctx().send_viewport_cmd(ViewportCommand::StartDrag);
            }

            let clock_width = if clock.is_some() {
                theme::CLOCK_WIDTH
            } else {
                0.0
            };
            let label_width = (width - clock_width - 2.0 * theme::BUTTON_SIZE.x - 4.0).max(0.0);

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
                                    .sense(Sense::click()),
                            )
                            .on_hover_text(name);
                        if response.secondary_clicked() {
                            action = BarAction::StartRename;
                        }
                    }
                },
            );

            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if icon_button(ui, Icon::Burger).clicked() {
                    action = BarAction::ToggleMenu;
                }
                let plus = icon_button(ui, Icon::Plus);
                if plus.clicked() {
                    action = BarAction::AddTask;
                } else if plus.secondary_clicked() {
                    action = BarAction::FinishTask;
                }

                if let Some(clock) = clock {
                    ui.add(
                        Label::new(RichText::new(clock).color(theme::TEXT_DIM).monospace())
                            .selectable(false),
                    );
                }
            });
        },
    );

    action
}
