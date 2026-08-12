//! The burger menu: a short list of commands, each opening a window of its own.
//!
//! The bar is a tiny window, so the menu cannot be drawn inside it: it opens as its own
//! small undecorated window just below the bar.

use egui::{Align, Context, Layout, RichText, ViewportBuilder, ViewportId};

use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MenuAction {
    #[default]
    None,
    OpenStack,
    OpenTimer,
    OpenGroom,
    OpenEndDay,
    OpenWeek,
    OpenRevive,
    ToggleDuration,
    ToggleDecorations,
}

pub struct MenuOutcome {
    pub action: MenuAction,
    pub keep_open: bool,
    /// Whether the menu window has held the keyboard focus at least once.
    pub was_focused: bool,
}

const MENU_WIDTH: f32 = 270.0;

/// Draws the menu window and reports what was picked.
///
/// `below` is where the bar sits, so the menu can open underneath it; on Wayland the
/// position is unknown and the compositor decides.
/// `was_focused` says whether the menu already held the focus, which is what makes
/// "clicking somewhere else closes the menu" safe: a window that never had the focus is
/// still opening, not being dismissed.
pub fn show(
    ctx: &Context,
    can_revive: bool,
    show_duration: bool,
    decorated: bool,
    notice: Option<&str>,
    below: Option<(f32, f32)>,
    was_focused: bool,
) -> MenuOutcome {
    let duration_label = if show_duration {
        "hide duration"
    } else {
        "show duration"
    };
    let frame_label = if decorated {
        "hide window frame"
    } else {
        "show window frame"
    };

    // label, action, enabled, tooltip, closes the menu
    let entries: [(&str, MenuAction, bool, &str, bool); 8] = [
        (
            "select",
            MenuAction::OpenStack,
            true,
            "Show the task stack and switch to another task",
            true,
        ),
        (
            "timer",
            MenuAction::OpenTimer,
            true,
            "Set how long each task may run per day before it beeps",
            true,
        ),
        (
            "groom",
            MenuAction::OpenGroom,
            true,
            "Finish several open tasks at once",
            true,
        ),
        (
            "end day",
            MenuAction::OpenEndDay,
            true,
            "Today's report; closing the day saves and quits",
            true,
        ),
        (
            "week",
            MenuAction::OpenWeek,
            true,
            "One row per task, one column per weekday",
            true,
        ),
        (
            "revive",
            MenuAction::OpenRevive,
            can_revive,
            "Put a finished task back on the stack",
            true,
        ),
        (
            duration_label,
            MenuAction::ToggleDuration,
            true,
            "Show or hide the running clock on the bar",
            true,
        ),
        (
            frame_label,
            MenuAction::ToggleDecorations,
            true,
            "Give the window its normal title bar, so any desktop can move it.\nTakes \
             effect after a restart",
            false,
        ),
    ];

    let height = entries.len() as f32 * 26.0 + if notice.is_some() { 56.0 } else { 20.0 };

    let mut action = MenuAction::None;
    let mut keep_open = true;
    let mut was_focused = was_focused;

    let mut builder = ViewportBuilder::default()
        .with_title("WipTracker menu")
        .with_inner_size([MENU_WIDTH, height])
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top();
    if let Some((x, y)) = below {
        builder = builder.with_position(egui::pos2(x, y + theme::BAR_SIZE.y));
    }

    ctx.show_viewport_immediate(ViewportId::from_hash_of("menu"), builder, |ctx, _class| {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::new()
                    .fill(theme::BACKGROUND)
                    .stroke(egui::Stroke::new(1.0, theme::BORDER))
                    .inner_margin(8),
            )
            .show(ctx, |ui| {
                ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                    for (label, picked, enabled, hint, closes) in entries {
                        let color = if enabled {
                            theme::TEXT
                        } else {
                            theme::TEXT_DIM
                        };
                        let clicked = crate::ui::widgets::tooltip(
                            ui.add_enabled(
                                enabled,
                                egui::Button::new(RichText::new(label).color(color)),
                            ),
                            hint,
                        )
                        .clicked();
                        if clicked {
                            action = picked;
                            keep_open = !closes;
                        }
                    }

                    if let Some(notice) = notice {
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(notice)
                                .color(egui::Color32::from_rgb(0xE0, 0xB0, 0x5A))
                                .small(),
                        );
                    }
                });
            });

        if ctx.input(|i| i.key_pressed(egui::Key::Escape))
            || ctx.input(|i| i.viewport().close_requested())
        {
            keep_open = false;
        }
        // Clicking anywhere else takes the focus away, which is how menus are dismissed.
        // Only once the menu has actually had the focus, or it would close on the very
        // frame it opens.
        match ctx.input(|i| i.viewport().focused) {
            Some(true) => was_focused = true,
            Some(false) if was_focused => keep_open = false,
            _ => {}
        }
    });

    MenuOutcome {
        action,
        keep_open,
        was_focused,
    }
}
