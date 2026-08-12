//! The burger menu: the full list of commands, each opening a window of its own.
//!
//! The bar is a tiny window, so the menu cannot be drawn inside it: it opens as its own
//! small undecorated window next to the bar.
//!
//! Every gesture on the bar also appears here. The gestures are accelerators, never the
//! only way in — a touchscreen shows no tooltips at all, so a menu entry is the only thing
//! that can be discovered by looking.

use egui::{Align, Context, Layout, RichText, ViewportBuilder, ViewportId};

use crate::theme;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum MenuAction {
    #[default]
    None,
    NewTask,
    Rename,
    Finish,
    Pause,
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

/// What the menu needs to know to draw itself and to place itself on the screen.
pub struct MenuContext<'a> {
    /// Whether there is a finished task that could be put back on the stack.
    pub can_revive: bool,
    /// Whether the focused task is the break, which cannot be renamed or paused again.
    pub paused: bool,
    pub show_duration: bool,
    pub decorated: bool,
    /// A short message shown at the bottom, such as "restart to apply".
    pub notice: Option<&'a str>,
    /// Where the bar sits. Unknown on Wayland, where the compositor places windows.
    pub bar: Option<(f32, f32)>,
    /// How big the monitor the bar is on is, when the platform says.
    pub monitor: Option<egui::Vec2>,
    /// Whether the menu already held the focus, which is what makes "clicking somewhere
    /// else closes the menu" safe: a window that never had the focus is still opening.
    pub was_focused: bool,
}

const MENU_WIDTH: f32 = 270.0;
const ROW_HEIGHT: f32 = 26.0;
const SEPARATOR_HEIGHT: f32 = 6.0;
const PADDING: f32 = 20.0;
const NOTICE_HEIGHT: f32 = 56.0;

struct Item {
    label: String,
    action: MenuAction,
    enabled: bool,
    hint: &'static str,
    /// Whether picking this closes the menu.
    closes: bool,
}

enum Row {
    Separator,
    Item(Item),
}

fn item(label: impl Into<String>, action: MenuAction, enabled: bool, hint: &'static str) -> Row {
    Row::Item(Item {
        label: label.into(),
        action,
        enabled,
        hint,
        closes: true,
    })
}

/// The entries, grouped by what they act on: the current task, the stack, the reports,
/// and the bar's own settings.
fn rows(context: &MenuContext<'_>) -> Vec<Row> {
    let duration_label = if context.show_duration {
        "hide duration"
    } else {
        "show duration"
    };
    let frame_label = if context.decorated {
        "hide window frame"
    } else {
        "show window frame"
    };
    let finish_label = if context.paused {
        "end break"
    } else {
        "finish"
    };

    vec![
        item(
            "new task",
            MenuAction::NewTask,
            true,
            "Start a new task — the plus button on the bar does the same",
        ),
        item(
            "rename",
            MenuAction::Rename,
            !context.paused,
            "Rename this task — clicking its name on the bar does the same",
        ),
        item(
            finish_label,
            MenuAction::Finish,
            true,
            "Finish this task — holding its name on the bar for two seconds does the same",
        ),
        item(
            "pause",
            MenuAction::Pause,
            !context.paused,
            "Take a break — holding the fork button on the bar does the same",
        ),
        Row::Separator,
        item(
            "select",
            MenuAction::OpenStack,
            true,
            "Show the task stack and switch to another task — the fork button does the same",
        ),
        item(
            "revive",
            MenuAction::OpenRevive,
            context.can_revive,
            "Put a finished task back on the stack — holding the plus button does the same",
        ),
        Row::Separator,
        item(
            "timer",
            MenuAction::OpenTimer,
            true,
            "Set how long each task may run per day before it beeps — holding the menu \
             button does the same",
        ),
        item(
            "groom",
            MenuAction::OpenGroom,
            true,
            "Finish several open tasks at once",
        ),
        item(
            "end day",
            MenuAction::OpenEndDay,
            true,
            "Today's report; closing the day saves and quits",
        ),
        item(
            "week",
            MenuAction::OpenWeek,
            true,
            "One row per task, one column per weekday",
        ),
        Row::Separator,
        item(
            duration_label,
            MenuAction::ToggleDuration,
            true,
            "Show or hide the running clock on the bar",
        ),
        Row::Item(Item {
            label: frame_label.to_owned(),
            action: MenuAction::ToggleDecorations,
            enabled: true,
            hint: "Give the window its normal title bar, so any desktop can move it.\nTakes \
                   effect after a restart",
            closes: false,
        }),
    ]
}

fn wanted_height(rows: &[Row], notice: bool) -> f32 {
    let content: f32 = rows
        .iter()
        .map(|row| match row {
            Row::Separator => SEPARATOR_HEIGHT,
            Row::Item(_) => ROW_HEIGHT,
        })
        .sum();
    content + PADDING + if notice { NOTICE_HEIGHT } else { 0.0 }
}

/// Where the menu goes, and how tall it may be.
///
/// The only rule is that the menu has to be fully visible. Below the bar is where it ends
/// up most of the time, simply because that is usually where it fits; a bar near the
/// bottom of the screen gets it above instead. Nothing here reaches the screen on Wayland,
/// where `with_position` is a no-op and the compositor decides.
fn place(bar: (f32, f32), monitor: Option<egui::Vec2>, wanted: f32) -> (egui::Pos2, f32) {
    let (x, y) = bar;
    let Some(monitor) = monitor else {
        return (egui::pos2(x, y + theme::BAR_HEIGHT), wanted);
    };
    let height = wanted.min(monitor.y);
    let below = y + theme::BAR_HEIGHT;
    let top = if below + height <= monitor.y {
        below
    } else {
        (y - height).max(0.0)
    };
    let left = x.min((monitor.x - MENU_WIDTH).max(0.0)).max(0.0);
    (egui::pos2(left, top), height)
}

/// Draws the menu window and reports what was picked.
pub fn show(ctx: &Context, context: &MenuContext<'_>) -> MenuOutcome {
    let rows = rows(context);
    let wanted = wanted_height(&rows, context.notice.is_some());
    let (position, height) = match context.bar {
        Some(bar) => {
            let (position, height) = place(bar, context.monitor, wanted);
            (Some(position), height)
        }
        None => (None, wanted),
    };

    let mut action = MenuAction::None;
    let mut keep_open = true;
    let mut was_focused = context.was_focused;

    let mut builder = ViewportBuilder::default()
        .with_title("WipTracker menu")
        .with_inner_size([MENU_WIDTH, height])
        .with_decorations(false)
        .with_resizable(false)
        .with_always_on_top();
    if let Some(position) = position {
        builder = builder.with_position(position);
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
                // Only scrolls when the screen is too short for the whole list; otherwise
                // the window is exactly as tall as the entries.
                egui::ScrollArea::vertical().show(ui, |ui| {
                    ui.with_layout(Layout::top_down_justified(Align::LEFT), |ui| {
                        for row in &rows {
                            match row {
                                Row::Separator => {
                                    ui.add_space(SEPARATOR_HEIGHT / 2.0);
                                    ui.separator();
                                }
                                Row::Item(entry) => {
                                    let color = if entry.enabled {
                                        theme::TEXT
                                    } else {
                                        theme::TEXT_DIM
                                    };
                                    let clicked = crate::ui::widgets::tooltip(
                                        ui.add_enabled(
                                            entry.enabled,
                                            egui::Button::new(
                                                RichText::new(&entry.label).color(color),
                                            ),
                                        ),
                                        entry.hint,
                                    )
                                    .clicked();
                                    if clicked {
                                        action = entry.action;
                                        keep_open = !entry.closes;
                                    }
                                }
                            }
                        }

                        if let Some(notice) = context.notice {
                            ui.add_space(6.0);
                            ui.label(
                                RichText::new(notice)
                                    .color(egui::Color32::from_rgb(0xE0, 0xB0, 0x5A))
                                    .small(),
                            );
                        }
                    });
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
