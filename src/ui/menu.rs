//! The burger menu.
//!
//! The bar is a tiny window, so the menu cannot be drawn inside it: it opens as its own
//! small undecorated window just below the bar.

use chrono::Local;
use egui::{Align, Context, Layout, RichText, ViewportBuilder, ViewportId};

use crate::domain::task::TaskId;
use crate::domain::tracker::Tracker;
use crate::theme;
use crate::ui::format;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MenuAction {
    #[default]
    None,
    Select(TaskId),
    ToggleDuration,
    ToggleDecorations,
    OpenGroom,
    OpenEndDay,
    OpenWeek,
    OpenRevive,
}

pub struct MenuOutcome {
    pub action: MenuAction,
    pub keep_open: bool,
    /// Whether the menu window has held the keyboard focus at least once.
    pub was_focused: bool,
}

const MENU_WIDTH: f32 = 260.0;
const ROW_HEIGHT: f32 = 22.0;

/// Draws the menu window and reports what was picked.
///
/// `below` is where the bar sits, so the menu can open underneath it; on Wayland the
/// position is unknown and the compositor decides.
/// `was_focused` says whether the menu already held the focus, which is what makes
/// "clicking somewhere else closes the menu" safe: a window that never had the focus is
/// still opening, not being dismissed.
pub fn show(
    ctx: &Context,
    tracker: &Tracker,
    show_duration: bool,
    decorated: bool,
    below: Option<(f32, f32)>,
    was_focused: bool,
) -> MenuOutcome {
    let open_tasks = tracker.open_tasks_top_first();
    let today = Local::now().date_naive();
    let rows = open_tasks.len() as f32 + 7.0;
    let height = (rows * ROW_HEIGHT + 24.0).min(420.0);

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
                    ui.label(RichText::new("switch to").color(theme::TEXT_DIM).small());
                    for task in &open_tasks {
                        let is_current = task.id == tracker.focused_id();
                        let today_time = format::coarse(tracker.duration_on(task.id, today));
                        let text = format!("{}   {today_time}", task.name);
                        if is_current {
                            ui.label(RichText::new(format!("● {text}")).color(theme::TEXT));
                        } else if ui.button(RichText::new(text).color(theme::TEXT)).clicked() {
                            action = MenuAction::Select(task.id);
                            keep_open = false;
                        }
                    }

                    ui.separator();
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
                    let can_revive = !tracker.finished_tasks().is_empty();
                    for (label, picked, enabled) in [
                        (duration_label, MenuAction::ToggleDuration, true),
                        (frame_label, MenuAction::ToggleDecorations, true),
                        ("groom", MenuAction::OpenGroom, true),
                        ("end day", MenuAction::OpenEndDay, true),
                        ("week", MenuAction::OpenWeek, true),
                        ("revive", MenuAction::OpenRevive, can_revive),
                    ] {
                        let color = if enabled {
                            theme::TEXT
                        } else {
                            theme::TEXT_DIM
                        };
                        let clicked = ui
                            .add_enabled(
                                enabled,
                                egui::Button::new(RichText::new(label).color(color)),
                            )
                            .clicked();
                        if clicked {
                            action = picked;
                            keep_open = false;
                        }
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
