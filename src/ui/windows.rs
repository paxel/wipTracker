//! The windows behind the menu: the task stack, timers, groom, end day, week and revive.
//!
//! Each is a normal decorated window of its own, so it can be moved, resized and closed
//! like any other; the bar keeps working while they are open.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use chrono::{DateTime, Datelike as _, Days, Local, NaiveDate, NaiveTime, TimeZone as _};
use egui::{Context, RichText, ViewportBuilder, ViewportId};

use crate::domain::export;
use crate::domain::task::TaskId;
use crate::domain::tracker::Tracker;
use crate::theme;
use crate::ui::format;
use crate::ui::place::Placement;
use crate::ui::widgets;

/// How far back the revive window looks.
pub const REVIVE_DAYS: i64 = 30;

/// Which report windows are open, and the little bit of state they each remember.
#[derive(Default)]
pub struct OpenWindows {
    pub stack: bool,
    /// The one-time offer to add WipTracker to the application menu.
    pub launcher_offer: bool,
    /// The pause reminder asking for a pause.
    pub pause_reminder: bool,
    pub timer: bool,
    pub groom: bool,
    pub end_day: bool,
    /// Ending the focused task's span earlier than now.
    pub end_earlier: bool,
    /// Whether the end-earlier window was opened by the app's own offer, which gives
    /// it a "not now" button, rather than from the menu.
    pub end_earlier_offered: bool,
    pub week: bool,
    pub revive: bool,
    groom_selection: BTreeSet<TaskId>,
    /// Any day inside the week the week view is showing.
    week_anchor: Option<NaiveDate>,
    /// What the user typed into the week view's date field.
    week_input: String,
    /// Which row of the timer window has its duration picker open. `None` is the default
    /// timer's row.
    timer_editing: Option<TimerTarget>,
    /// What the user typed into the pause reminder's time field.
    reminder_input: String,
    /// Whether the reminder's time field has been given the focus since it opened.
    reminder_focused: bool,
    /// Whether each window is showing its "copied to clipboard" confirmation.
    end_day_copied: bool,
    week_copied: bool,
    /// The end-earlier fields: the day and the clock time of the cut, as typed, and the
    /// span start they were prefilled from, so a new span prefills them again.
    cut_date: String,
    cut_time: String,
    cut_prefilled: Option<DateTime<Local>>,
    /// Where the bar is and where each window went; see [`Placing`].
    pub placing: Placing,
}

/// What placing a window takes: where the bar and its screen are this frame — the app
/// refreshes that before drawing — and where each open window was put when it opened.
/// The latter is remembered because recomputing it every frame would re-issue
/// `with_position`, which drags an open report window along with the bar.
#[derive(Default)]
pub struct Placing {
    pub placement: Placement,
    placed: BTreeMap<&'static str, egui::Pos2>,
}

/// What the user decided in the launcher offer window.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum LauncherChoice {
    #[default]
    None,
    /// Write the entry and the icons into the user's own XDG directories.
    Install,
    /// Never offer again.
    Dismiss,
}

/// The one-time offer to put WipTracker into the application menu.
///
/// Closing the window without choosing leaves the offer for the next start; only the
/// explicit "don't ask again" is remembered.
pub fn launcher_offer(ctx: &Context, open: &mut OpenWindows) -> LauncherChoice {
    let mut placing = std::mem::take(&mut open.placing);
    let mut choice = LauncherChoice::None;
    let mut still_open = true;

    window(
        ctx,
        "launcher_offer",
        "WipTracker — application menu",
        [420.0, 230.0],
        &mut placing,
        &mut still_open,
        |ui| {
            ui.heading("Add WipTracker to the application menu?");
            ui.add_space(4.0);
            ui.label(
                "No menu on this machine can see WipTracker right now. Adding it writes \
                 a launcher entry and the icons into your own ~/.local/share, so it can \
                 be found, pinned and started like any other application.",
            );
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Uninstalling stays clean: the entry hides itself as soon as the \
                     binary is gone, and `wiptracker --remove-launcher` deletes it.",
                )
                .color(theme::current().text_dim)
                .small(),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if ui.button("Add to the menu").clicked() {
                    choice = LauncherChoice::Install;
                }
                if ui.button("Don't ask again").clicked() {
                    choice = LauncherChoice::Dismiss;
                }
            });
        },
    );

    open.placing = placing;
    open.launcher_offer = still_open && choice == LauncherChoice::None;
    choice
}

/// What the user answered the pause reminder.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ReminderChoice {
    #[default]
    None,
    /// Start the pause now.
    PauseNow,
    /// Ask again in a quarter of an hour.
    NotNow,
}

/// The pause reminder: a small window asking whether it is time for a pause.
///
/// Closing the window is "not now" as well: the question comes back, and only the pause
/// itself ends it.
pub fn pause_reminder(
    ctx: &Context,
    open: &mut OpenWindows,
    at: Option<NaiveTime>,
) -> ReminderChoice {
    let mut placing = std::mem::take(&mut open.placing);
    let mut choice = ReminderChoice::None;
    let mut still_open = true;

    window(
        ctx,
        "pause_reminder",
        "WipTracker — pause",
        [360.0, 200.0],
        &mut placing,
        &mut still_open,
        |ui| {
            ui.heading("Time for a pause?");
            ui.add_space(4.0);
            let when = at.map_or_else(|| "now".to_owned(), format::clock_time);
            ui.label(format!(
                "You asked to be reminded at {when}. Starting the pause stops the clock \
                 on the focused task from this moment."
            ));
            ui.add_space(4.0);
            ui.label(
                RichText::new("Not now asks again in a quarter of an hour, until you pause.")
                    .color(theme::current().text_dim)
                    .small(),
            );
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                if widgets::action_button(ui, "Pause now", true).clicked() {
                    choice = ReminderChoice::PauseNow;
                }
                if widgets::action_button(ui, "Not now", false).clicked() {
                    choice = ReminderChoice::NotNow;
                }
            });
        },
    );

    open.placing = placing;
    if choice == ReminderChoice::None && !still_open {
        choice = ReminderChoice::NotNow;
    }
    open.pause_reminder = choice == ReminderChoice::None;
    choice
}

/// What the windows did this frame.
#[derive(Clone, Default)]
pub struct WindowOutcome {
    /// Something changed that is worth storing.
    pub changed: bool,
    /// The user closed the day, which ends the session.
    pub day_closed: bool,
    /// JSON the user asked to have on the clipboard.
    pub copy: Option<String>,
    /// The user ended the focused task's span earlier.
    pub ended_earlier: Option<EndedEarlier>,
}

/// What an end-earlier cut did, for the journal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EndedEarlier {
    pub task: String,
    pub cut: DateTime<Local>,
    pub removed: Duration,
}

/// Draws every open window.
pub fn show_all(
    ctx: &Context,
    open: &mut OpenWindows,
    tracker: &mut Tracker,
    now: DateTime<Local>,
) -> WindowOutcome {
    let mut outcome = WindowOutcome::default();
    if open.stack {
        outcome.changed |= stack(ctx, open, tracker, now);
    }
    if open.timer {
        outcome.changed |= timer(ctx, open, tracker);
    }
    if open.groom {
        outcome.changed |= groom(ctx, open, tracker, now);
    }
    if open.end_day {
        let (day_closed, copy) = end_day(ctx, open, tracker, now);
        outcome.day_closed = day_closed;
        outcome.changed |= day_closed;
        outcome.copy = outcome.copy.or(copy);
    }
    if open.end_earlier {
        outcome.ended_earlier = end_earlier(ctx, open, tracker, now);
        outcome.changed |= outcome.ended_earlier.is_some();
    }
    if open.week {
        outcome.copy = outcome.copy.or(week(ctx, open, tracker, now));
    }
    if open.revive {
        outcome.changed |= revive(ctx, open, tracker, now);
    }
    outcome
}

/// An "export" button that yields `days` as JSON, plus the confirmation that it worked.
/// The flag lives in the caller so the message survives a repaint.
///
/// The JSON is handed back rather than copied here: this runs inside an immediate
/// viewport, whose output never reaches the platform, so a `copy_text` call from this
/// context would light up the label without touching the clipboard. The root context does
/// the copying.
fn export_button(
    ui: &mut egui::Ui,
    tracker: &Tracker,
    days: &[NaiveDate],
    copied: &mut bool,
    payload: &mut Option<String>,
) {
    ui.horizontal(|ui| {
        if widgets::action_button(ui, "export", false)
            .on_hover_text("Copy this data as JSON, one row per task per day")
            .clicked()
        {
            *payload = Some(export::to_json(tracker, days));
            *copied = true;
        }
        if *copied {
            ui.label(
                RichText::new("copied to clipboard")
                    .color(theme::current().text_dim)
                    .small(),
            );
        }
    });
}

/// A grid cell with its text against the right edge, so durations line up on the digits.
fn right_aligned(ui: &mut egui::Ui, text: RichText) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        ui.label(text);
    });
}

fn window(
    ctx: &Context,
    id: &'static str,
    title: &str,
    size: [f32; 2],
    placing: &mut Placing,
    open: &mut bool,
    contents: impl FnOnce(&mut egui::Ui),
) {
    let mut builder = ViewportBuilder::default()
        .with_title(title)
        .with_inner_size(size)
        .with_min_inner_size([320.0, 200.0]);
    // Decided once, when the window opens: the bar can be dragged afterwards, and a fresh
    // position on every frame would drag the report window along with it. Without a
    // position the window manager decides, and on Wayland it always does.
    let position = placing.placed.get(id).copied().or_else(|| {
        placing
            .placement
            .window(egui::vec2(size[0], size[1]))
            .inspect(|position| {
                placing.placed.insert(id, *position);
            })
    });
    if let Some(position) = position {
        builder = builder.with_position(position);
    }

    let mut contents = Some(contents);
    ctx.show_viewport_immediate(ViewportId::from_hash_of(id), builder, |ctx, _class| {
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::both().show(ui, |ui| {
                if let Some(contents) = contents.take() {
                    contents(ui);
                }
            });
        });
        if ctx.input(|i| i.viewport().close_requested()) {
            *open = false;
        }
    });

    if !*open {
        // Reopening should place it again rather than restore wherever it last sat.
        placing.placed.remove(id);
    }
}

/// The task stack: the focused task on top, every other open task under it, the pause
/// task last. Clicking a task brings it back to the top.
fn stack(
    ctx: &Context,
    open: &mut OpenWindows,
    tracker: &mut Tracker,
    now: DateTime<Local>,
) -> bool {
    let mut placing = std::mem::take(&mut open.placing);
    let today = now.date_naive();
    let focused = tracker.focused_id();
    let rows: Vec<(TaskId, String, Duration, Duration)> = tracker
        .open_tasks_top_first()
        .iter()
        .map(|task| {
            (
                task.id,
                task.name.clone(),
                tracker.duration_on(task.id, today),
                task.total,
            )
        })
        .collect();

    let mut pick: Option<TaskId> = None;
    let mut still_open = true;

    window(
        ctx,
        "stack",
        "WipTracker — task stack",
        [420.0, 320.0],
        &mut placing,
        &mut still_open,
        |ui| {
            ui.heading("Task stack");
            ui.label(
                RichText::new("Top of the stack first. Click a task to work on it again.")
                    .color(theme::current().text_dim)
                    .small(),
            );
            ui.add_space(6.0);

            // The top of the stack carries the visual weight: the top row is bigger, and
            // the names fade towards the dim tone the deeper they sit.
            let palette = theme::current();
            egui::Grid::new("stack_rows")
                .num_columns(3)
                .striped(true)
                .min_col_width(120.0)
                .show(ui, |ui| {
                    for (depth, (id, name, today_time, total)) in rows.iter().enumerate() {
                        let current = *id == focused;
                        let emphasis = theme::stack_text(&palette, depth, rows.len());
                        let mut text = RichText::new(if current {
                            format!("● {name}")
                        } else {
                            name.clone()
                        })
                        .color(emphasis);
                        if depth == 0 {
                            text = text.size(15.0);
                        }
                        if current {
                            ui.label(text.strong())
                                .on_hover_text("The task you are working on right now");
                        } else if ui
                            .button(text)
                            .on_hover_text("Left click: work on this task")
                            .clicked()
                        {
                            pick = Some(*id);
                        }
                        ui.label(
                            RichText::new(format!("today {}", format::coarse(*today_time)))
                                .color(theme::current().text_dim),
                        );
                        ui.label(
                            RichText::new(format!("total {}", format::coarse(*total)))
                                .color(theme::current().text_dim),
                        );
                        ui.end_row();
                    }
                });
        },
    );

    open.placing = placing;
    open.stack = still_open;
    if let Some(id) = pick {
        let _ = tracker.select(id, now);
        open.stack = false;
        return true;
    }
    false
}

/// What one row of the timer window sets.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TimerTarget {
    /// The whole day's work, pause excluded.
    Day,
    /// The timer new tasks inherit.
    Default,
    Task(TaskId),
    /// Auto-pause once the user has been idle this long. Off by default.
    IdlePause,
    /// The clock time the pause reminder asks at.
    PauseReminder,
}

/// The timer window: the whole day, the default new tasks inherit, and one row per task.
fn timer(ctx: &Context, open: &mut OpenWindows, tracker: &mut Tracker) -> bool {
    let mut placing = std::mem::take(&mut open.placing);
    let duration_label = |timer: Duration| {
        if timer.is_zero() {
            "off".to_owned()
        } else {
            format::coarse(timer)
        }
    };
    let rows: Vec<(TimerTarget, String, String)> = [
        (
            TimerTarget::Day,
            "the whole day".to_owned(),
            duration_label(tracker.day_timer()),
        ),
        (
            TimerTarget::IdlePause,
            "auto-pause when idle".to_owned(),
            duration_label(tracker.idle_pause()),
        ),
        (
            TimerTarget::PauseReminder,
            "pause reminder at".to_owned(),
            tracker
                .pause_reminder()
                .map_or_else(|| "off".to_owned(), format::clock_time),
        ),
        (
            TimerTarget::Default,
            "default for new tasks".to_owned(),
            duration_label(tracker.default_timer()),
        ),
    ]
    .into_iter()
    .chain(tracker.open_tasks_top_first().iter().map(|task| {
        (
            TimerTarget::Task(task.id),
            task.name.clone(),
            duration_label(task.timer),
        )
    }))
    .collect();

    let mut editing = open.timer_editing;
    let mut chosen: Option<(TimerTarget, Duration)> = None;
    // `Some(None)` switches the reminder off.
    let mut reminder: Option<Option<NaiveTime>> = None;
    let mut typed = open.reminder_input.clone();
    let mut typed_focused = open.reminder_focused;
    let mut still_open = true;

    window(
        ctx,
        "timer",
        "WipTracker — timers",
        [420.0, 340.0],
        &mut placing,
        &mut still_open,
        |ui| {
            ui.heading("Daily timers");
            ui.label(
                RichText::new(
                    "When a task has been worked on this long today, WipTracker beeps once. \
                     The whole day's timer counts every task together, pauses excluded, \
                     sounds its own noise, and turns the bar clock red. Auto-pause \
                     starts the pause by itself once keyboard and mouse have been quiet \
                     that long, and takes the quiet minutes off the task; off means \
                     WipTracker never watches your input. The pause reminder asks \
                     for a pause at that clock time, once a day. Zero means no alarm.",
                )
                .color(theme::current().text_dim)
                .small(),
            );
            ui.add_space(6.0);

            for (id, name, label) in &rows {
                ui.horizontal(|ui| {
                    let hover = if *id == TimerTarget::PauseReminder {
                        "Left click: choose the clock time to be asked for a pause at"
                    } else {
                        "Left click: choose how long this may run per day"
                    };
                    if ui
                        .button(
                            RichText::new(format!("{name}   {label}")).color(theme::current().text),
                        )
                        .on_hover_text(hover)
                        .clicked()
                    {
                        editing = if editing == Some(*id) {
                            None
                        } else {
                            Some(*id)
                        };
                        if editing == Some(TimerTarget::PauseReminder) {
                            typed = tracker
                                .pause_reminder()
                                .map(format::clock_time)
                                .unwrap_or_default();
                            typed_focused = false;
                        }
                    }
                });
                if editing == Some(TimerTarget::PauseReminder) && *id == TimerTarget::PauseReminder
                {
                    ui.horizontal(|ui| {
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut typed)
                                .hint_text("HH:MM")
                                .desired_width(70.0),
                        );
                        if !typed_focused {
                            response.request_focus();
                            typed_focused = true;
                        }
                        let parsed = format::parse_clock(&typed);
                        let submitted =
                            response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if (submitted || widgets::action_button(ui, "set", true).clicked())
                            && parsed.is_some()
                        {
                            reminder = Some(parsed);
                        }
                        if widgets::action_button(ui, "off", false).clicked() {
                            reminder = Some(None);
                        }
                        if !typed.trim().is_empty() && parsed.is_none() {
                            ui.label(
                                RichText::new("not a time")
                                    .color(egui::Color32::from_rgb(0xE8, 0x7A, 0x7A))
                                    .small(),
                            );
                        }
                    });
                    ui.add_space(4.0);
                } else if editing == Some(*id) {
                    // Idle spans are minutes, timers are hours; each row offers what
                    // makes sense for it.
                    let choices: &[(&str, u64)] = if *id == TimerTarget::IdlePause {
                        &[
                            ("off", 0),
                            ("3m", 3),
                            ("5m", 5),
                            ("10m", 10),
                            ("15m", 15),
                            ("30m", 30),
                        ]
                    } else {
                        &[
                            ("off", 0),
                            ("15m", 15),
                            ("30m", 30),
                            ("1h", 60),
                            ("2h", 120),
                            ("4h", 240),
                            ("8h", 480),
                        ]
                    };
                    ui.horizontal_wrapped(|ui| {
                        for (label, minutes) in choices.iter().copied() {
                            if ui.button(label).clicked() {
                                chosen =
                                    Some((*id, Duration::from_secs(minutes.saturating_mul(60))));
                            }
                        }
                    });
                    ui.add_space(4.0);
                }
            }
        },
    );

    open.placing = placing;
    open.timer = still_open;
    open.timer_editing = editing;
    open.reminder_input = typed;
    open.reminder_focused = typed_focused;
    if let Some(at) = reminder {
        tracker.set_pause_reminder(at);
        open.timer_editing = None;
        return true;
    }
    if let Some((target, duration)) = chosen {
        match target {
            TimerTarget::Day => tracker.set_day_timer(duration),
            TimerTarget::IdlePause => tracker.set_idle_pause(duration),
            // Picked through its own field, never from the duration choices.
            TimerTarget::PauseReminder => {}
            TimerTarget::Default => tracker.set_default_timer(duration),
            TimerTarget::Task(id) => {
                let _ = tracker.set_timer(id, duration);
            }
        }
        open.timer_editing = None;
        return true;
    }
    false
}

fn groom(
    ctx: &Context,
    open: &mut OpenWindows,
    tracker: &mut Tracker,
    now: DateTime<Local>,
) -> bool {
    let mut placing = std::mem::take(&mut open.placing);
    let mut finish_now = false;
    let mut still_open = true;
    let tasks: Vec<(TaskId, String, Duration)> = tracker
        .open_tasks_top_first()
        .iter()
        .filter(|task| !task.is_pause())
        .map(|task| (task.id, task.name.clone(), task.total))
        .collect();
    let selection = &mut open.groom_selection;

    window(
        ctx,
        "groom",
        "WipTracker — groom",
        [420.0, 320.0],
        &mut placing,
        &mut still_open,
        |ui| {
            ui.heading("Open tasks");
            ui.add_space(4.0);
            if tasks.is_empty() {
                ui.label("Nothing open but the pause task.");
                return;
            }
            for (id, name, total) in &tasks {
                let mut checked = selection.contains(id);
                ui.horizontal(|ui| {
                    let box_response = ui.checkbox(
                        &mut checked,
                        RichText::new(name).color(theme::current().text),
                    );
                    if box_response.changed() {
                        if checked {
                            selection.insert(*id);
                        } else {
                            selection.remove(id);
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            RichText::new(format::coarse(*total)).color(theme::current().text_dim),
                        );
                    });
                });
            }
            ui.add_space(8.0);
            let count = selection.len();
            ui.add_enabled_ui(count > 0, |ui| {
                if ui.button(format!("Finish selected ({count})")).clicked() {
                    finish_now = true;
                }
            });
        },
    );

    open.placing = placing;
    open.groom = still_open;
    if finish_now {
        let ids: Vec<TaskId> = open.groom_selection.iter().copied().collect();
        tracker.finish_all(&ids, now);
        open.groom_selection.clear();
        return true;
    }
    false
}

/// Returns whether the day was closed, and any JSON the user asked to copy.
fn end_day(
    ctx: &Context,
    open: &mut OpenWindows,
    tracker: &mut Tracker,
    now: DateTime<Local>,
) -> (bool, Option<String>) {
    let mut placing = std::mem::take(&mut open.placing);
    let today = now.date_naive();
    let record = tracker.day(today).cloned().unwrap_or_default();
    let rows: Vec<(String, Duration)> = tracker
        .tasks_active_on(today)
        .into_iter()
        .map(|(task, duration)| (task.name.clone(), duration))
        .collect();
    let mut close_day = false;
    let mut still_open = true;
    let mut copied = open.end_day_copied;
    let mut payload = None;

    window(
        ctx,
        "end_day",
        "WipTracker — end day",
        [460.0, 360.0],
        &mut placing,
        &mut still_open,
        |ui| {
            let palette = theme::current();
            ui.heading(today.format("%A, %-d %B %Y").to_string());
            let started = record
                .started_at
                .map_or("—".to_owned(), |time| time.format("%H:%M").to_string());
            let ended = record.ended_at.map_or_else(
                || now.format("%H:%M").to_string(),
                |time| time.format("%H:%M").to_string(),
            );
            ui.label(
                RichText::new(format!("started {started}, last activity {ended}"))
                    .color(palette.text_dim),
            );
            ui.add_space(10.0);

            // The number the window is about, set larger than everything around it.
            let worked = tracker.worked_on(today);
            let day_timer = tracker.day_timer();
            let color = if tracker.day_over(today) {
                palette.day_over
            } else {
                palette.text
            };
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format::coarse(worked))
                        .size(26.0)
                        .color(color),
                );
                let rest = if day_timer.is_zero() {
                    "worked, pauses not counted".to_owned()
                } else {
                    format!(
                        "of {} worked, pauses not counted",
                        format::coarse(day_timer)
                    )
                };
                ui.label(RichText::new(rest).color(palette.text_dim));
            });
            if record.closed {
                ui.label(RichText::new("This day is closed.").color(palette.text_dim));
            }
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            if rows.is_empty() {
                ui.label(RichText::new("No time collected today yet.").color(palette.text_dim));
            } else {
                egui::Grid::new("end_day_rows")
                    .num_columns(2)
                    .striped(true)
                    .min_col_width(180.0)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("task").color(palette.text_dim).small());
                        right_aligned(ui, RichText::new("today").color(palette.text_dim).small());
                        ui.end_row();
                        for (name, duration) in &rows {
                            ui.label(RichText::new(name).color(palette.text));
                            right_aligned(
                                ui,
                                RichText::new(format::coarse(*duration)).color(palette.text),
                            );
                            ui.end_row();
                        }
                        ui.label(RichText::new("total").strong().color(palette.text));
                        right_aligned(
                            ui,
                            RichText::new(format::coarse(record.total()))
                                .strong()
                                .color(palette.text),
                        );
                        ui.end_row();
                    });
            }

            ui.add_space(14.0);
            export_button(ui, tracker, &[today], &mut copied, &mut payload);
            ui.add_space(10.0);
            if widgets::action_button(ui, "Close day", true).clicked() {
                close_day = true;
            }
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "Closing the day stamps its end time, saves, and quits. Open tasks stay \
                     on the stack for tomorrow.",
                )
                .color(palette.text_dim)
                .small(),
            );
        },
    );

    open.placing = placing;
    open.end_day = still_open;
    open.end_day_copied = copied && still_open;
    if close_day {
        tracker.close_day(now);
        return (true, payload);
    }
    (false, payload)
}

/// The end-earlier window: since when the focused task has been focused, a day and a
/// clock time for the cut, and what the cut would take off. Returns what it took once
/// the user cut; the window closes with it.
fn end_earlier(
    ctx: &Context,
    open: &mut OpenWindows,
    tracker: &mut Tracker,
    now: DateTime<Local>,
) -> Option<EndedEarlier> {
    let mut placing = std::mem::take(&mut open.placing);
    let since = tracker.focused_since();
    let name = tracker.focused_name().to_owned();
    let crosses_days = since.date_naive() < now.date_naive();
    if open.cut_prefilled != Some(since) {
        open.cut_date = since.format("%Y-%m-%d").to_string();
        open.cut_time = format::clock_time(since.time());
        open.cut_prefilled = Some(since);
    }
    let mut date = open.cut_date.clone();
    let mut time = open.cut_time.clone();
    let offered = open.end_earlier_offered;
    let mut cut_now: Option<DateTime<Local>> = None;
    let mut not_now = false;
    let mut still_open = true;

    window(
        ctx,
        "end_earlier",
        "WipTracker — end earlier",
        [460.0, 300.0],
        &mut placing,
        &mut still_open,
        |ui| {
            let palette = theme::current();
            ui.heading("End earlier");
            ui.add_space(4.0);
            let since_text = if crosses_days {
                since.format("%A, %-d %B at %H:%M").to_string()
            } else {
                since.format("%H:%M").to_string()
            };
            ui.label(format!("'{name}' has been focused since {since_text}."));
            ui.label(
                RichText::new(
                    "When did you really stop? Everything after that comes off the task; \
                     it keeps running from now.",
                )
                .color(palette.text_dim),
            );
            ui.add_space(8.0);

            let date_ok = if crosses_days {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("day").color(palette.text_dim));
                    ui.add(
                        egui::TextEdit::singleline(&mut date)
                            .hint_text("YYYY-MM-DD")
                            .desired_width(110.0),
                    );
                    ui.label(RichText::new("time").color(palette.text_dim));
                    ui.add(
                        egui::TextEdit::singleline(&mut time)
                            .hint_text("HH:MM")
                            .desired_width(70.0),
                    );
                });
                NaiveDate::parse_from_str(date.trim(), "%Y-%m-%d").ok()
            } else {
                ui.horizontal(|ui| {
                    ui.label(RichText::new("time").color(palette.text_dim));
                    ui.add(
                        egui::TextEdit::singleline(&mut time)
                            .hint_text("HH:MM")
                            .desired_width(70.0),
                    );
                });
                Some(now.date_naive())
            };
            let cut = date_ok
                .zip(format::parse_clock(&time))
                .and_then(|(day, at)| Local.from_local_datetime(&day.and_time(at)).earliest());
            ui.add_space(6.0);

            let verdict = match cut {
                None => Err("not a day and a time".to_owned()),
                Some(cut) => tracker
                    .end_earlier_preview(cut, now)
                    .map_err(|error| error.to_string())
                    .map(|removal| (cut, removal)),
            };
            let ready = match &verdict {
                Ok((_, removal)) => {
                    let total = removal
                        .iter()
                        .fold(Duration::ZERO, |sum, (_, taken)| sum.saturating_add(*taken));
                    let days = removal
                        .iter()
                        .map(|(day, taken)| {
                            format!("{} {}", day.format("%a"), format::coarse(*taken))
                        })
                        .collect::<Vec<_>>()
                        .join(", ");
                    if total.is_zero() {
                        ui.label(RichText::new("Nothing to take off.").color(palette.text_dim));
                        false
                    } else {
                        ui.label(format!(
                            "Takes {} off '{name}': {days}.",
                            format::coarse(total)
                        ));
                        true
                    }
                }
                Err(reason) => {
                    ui.label(
                        RichText::new(reason)
                            .color(egui::Color32::from_rgb(0xE8, 0x7A, 0x7A))
                            .small(),
                    );
                    false
                }
            };
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if widgets::action_button(ui, "End there", true).clicked()
                    && ready
                    && let Ok((cut, _)) = verdict
                {
                    cut_now = Some(cut);
                }
                if offered && widgets::action_button(ui, "Not now", false).clicked() {
                    not_now = true;
                }
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(
                    "The day of the cut ends there and is closed; days in between that \
                     held nothing else disappear.",
                )
                .color(palette.text_dim)
                .small(),
            );
        },
    );

    open.placing = placing;
    open.cut_date = date;
    open.cut_time = time;
    let mut ended = None;
    if let Some(cut) = cut_now
        && let Ok(removed) = tracker.end_earlier(cut, now)
    {
        ended = Some(EndedEarlier {
            task: name,
            cut,
            removed,
        });
        still_open = false;
    }
    if not_now {
        still_open = false;
    }
    open.end_earlier = still_open;
    if !still_open {
        open.end_earlier_offered = false;
    }
    ended
}

/// Returns any JSON the user asked to copy.
fn week(
    ctx: &Context,
    open: &mut OpenWindows,
    tracker: &Tracker,
    now: DateTime<Local>,
) -> Option<String> {
    let mut placing = std::mem::take(&mut open.placing);
    let anchor = open.week_anchor.unwrap_or_else(|| now.date_naive());
    let monday = monday_of(anchor);
    let days: Vec<NaiveDate> = (0..7)
        .filter_map(|offset| monday.checked_add_days(Days::new(offset)))
        .collect();

    let mut task_rows: Vec<(String, Vec<Duration>, Duration)> = Vec::new();
    let mut seen: BTreeSet<TaskId> = BTreeSet::new();
    for day in &days {
        if let Some(record) = tracker.day(*day) {
            seen.extend(record.per_task.keys().copied());
        }
    }
    for id in &seen {
        let Some(task) = tracker.task(*id) else {
            continue;
        };
        let per_day: Vec<Duration> = days
            .iter()
            .map(|day| tracker.duration_on(*id, *day))
            .collect();
        let total = per_day
            .iter()
            .fold(Duration::ZERO, |sum, d| sum.saturating_add(*d));
        task_rows.push((task.name.clone(), per_day, total));
    }
    task_rows.sort_by_key(|(_, _, total)| std::cmp::Reverse(*total));

    let mut anchor_change: Option<NaiveDate> = None;
    let mut still_open = true;
    let mut typed = open.week_input.clone();
    let mut copied = open.week_copied;
    let mut payload = None;
    let week_days = days.clone();

    window(
        ctx,
        "week",
        "WipTracker — week",
        [720.0, 400.0],
        &mut placing,
        &mut still_open,
        |ui| {
            let palette = theme::current();
            ui.heading(format!("Week {}", monday.iso_week().week()));
            ui.label(
                RichText::new(format!(
                    "{} to {}",
                    monday.format("%A, %-d %B %Y"),
                    days.last()
                        .map_or_else(String::new, |day| day.format("%A, %-d %B %Y").to_string())
                ))
                .color(palette.text_dim),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if widgets::action_button(ui, "previous week", false).clicked() {
                    anchor_change = monday.checked_sub_days(Days::new(7));
                }
                if widgets::action_button(ui, "this week", false).clicked() {
                    anchor_change = Some(now.date_naive());
                }
                if widgets::action_button(ui, "next week", false).clicked() {
                    anchor_change = monday.checked_add_days(Days::new(7));
                }
                ui.add_space(12.0);
                export_button(ui, tracker, &week_days, &mut copied, &mut payload);
            });
            ui.add_space(6.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("jump to date").color(palette.text_dim));
                let response = ui.add(
                    egui::TextEdit::singleline(&mut typed)
                        .hint_text("YYYY-MM-DD")
                        .desired_width(110.0),
                );
                let submitted =
                    response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let picked = NaiveDate::parse_from_str(typed.trim(), "%Y-%m-%d");
                if submitted && let Ok(date) = picked {
                    anchor_change = Some(date);
                }
                if !typed.trim().is_empty() && picked.is_err() {
                    ui.label(
                        RichText::new("not a date")
                            .color(egui::Color32::from_rgb(0xE8, 0x7A, 0x7A))
                            .small(),
                    );
                }
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(6.0);

            if task_rows.is_empty() {
                ui.label(
                    RichText::new("Nothing was tracked in this week.").color(palette.text_dim),
                );
                return;
            }

            let today = now.date_naive();
            egui::Grid::new("week_grid")
                .num_columns(9)
                .striped(true)
                .min_col_width(64.0)
                .spacing([16.0, 6.0])
                .show(ui, |ui| {
                    ui.label(RichText::new("task").color(palette.text_dim).small());
                    for day in &days {
                        // Today is the column the eye looks for first.
                        let header = RichText::new(day.format("%a %-d").to_string()).small();
                        let header = if *day == today {
                            header.strong().color(palette.text)
                        } else {
                            header.color(palette.text_dim)
                        };
                        right_aligned(ui, header);
                    }
                    right_aligned(ui, RichText::new("total").color(palette.text_dim).small());
                    ui.end_row();

                    for (name, per_day, total) in &task_rows {
                        ui.label(RichText::new(name).color(palette.text));
                        for duration in per_day {
                            right_aligned(
                                ui,
                                if duration.is_zero() {
                                    RichText::new("·").color(palette.text_dim)
                                } else {
                                    RichText::new(format::coarse(*duration)).color(palette.text)
                                },
                            );
                        }
                        right_aligned(
                            ui,
                            RichText::new(format::coarse(*total))
                                .strong()
                                .color(palette.text),
                        );
                        ui.end_row();
                    }

                    ui.label(RichText::new("total").strong().color(palette.text));
                    let mut week_total = Duration::ZERO;
                    for (index, _) in days.iter().enumerate() {
                        let column = task_rows
                            .iter()
                            .filter_map(|(_, per_day, _)| per_day.get(index).copied())
                            .fold(Duration::ZERO, Duration::saturating_add);
                        week_total = week_total.saturating_add(column);
                        right_aligned(
                            ui,
                            RichText::new(format::coarse(column))
                                .strong()
                                .color(palette.text),
                        );
                    }
                    right_aligned(
                        ui,
                        RichText::new(format::coarse(week_total))
                            .strong()
                            .color(palette.text),
                    );
                    ui.end_row();
                });
        },
    );

    open.placing = placing;
    open.week = still_open;
    open.week_input = typed;
    open.week_copied = copied && still_open;
    if let Some(new_anchor) = anchor_change {
        open.week_anchor = Some(new_anchor);
    }
    payload
}

fn revive(
    ctx: &Context,
    open: &mut OpenWindows,
    tracker: &mut Tracker,
    now: DateTime<Local>,
) -> bool {
    let mut placing = std::mem::take(&mut open.placing);
    let finished: Vec<(TaskId, String, Duration, String)> = tracker
        .recently_finished(REVIVE_DAYS, now)
        .iter()
        .map(|task| {
            (
                task.id,
                task.name.clone(),
                task.total,
                task.finished_at.map_or_else(String::new, |time| {
                    time.format("%Y-%m-%d %H:%M").to_string()
                }),
            )
        })
        .collect();

    let mut revive_id: Option<TaskId> = None;
    let mut still_open = true;

    window(
        ctx,
        "revive",
        "WipTracker — revive",
        [460.0, 340.0],
        &mut placing,
        &mut still_open,
        |ui| {
            ui.heading("Finished tasks");
            ui.label(
                RichText::new(format!(
                    "The last {REVIVE_DAYS} days. Older tasks stay in the week overview."
                ))
                .color(theme::current().text_dim)
                .small(),
            );
            ui.add_space(4.0);
            if finished.is_empty() {
                ui.label(format!(
                    "Nothing has been finished in the last {REVIVE_DAYS} days."
                ));
                return;
            }
            egui::Grid::new("revive_rows")
                .num_columns(3)
                .striped(true)
                .min_col_width(120.0)
                .show(ui, |ui| {
                    for (id, name, total, finished_at) in &finished {
                        if ui
                            .button(RichText::new(name).color(theme::current().text))
                            .clicked()
                        {
                            revive_id = Some(*id);
                        }
                        ui.label(
                            RichText::new(format::coarse(*total)).color(theme::current().text_dim),
                        );
                        ui.label(RichText::new(finished_at).color(theme::current().text_dim));
                        ui.end_row();
                    }
                });
        },
    );

    open.placing = placing;
    open.revive = still_open;
    if let Some(id) = revive_id {
        let _ = tracker.revive(id, now);
        // Nothing left to bring back, so the window has done its job.
        open.revive = !tracker.recently_finished(REVIVE_DAYS, now).is_empty();
        return true;
    }
    false
}

fn monday_of(day: NaiveDate) -> NaiveDate {
    let offset = day.weekday().num_days_from_monday();
    day.checked_sub_days(Days::new(u64::from(offset)))
        .unwrap_or(day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn monday_of_a_sunday_is_six_days_earlier() {
        let sunday = NaiveDate::from_ymd_opt(2026, 8, 16).expect("valid date");
        let monday = NaiveDate::from_ymd_opt(2026, 8, 10).expect("valid date");
        assert_eq!(monday_of(sunday), monday);
        assert_eq!(monday_of(monday), monday);
    }
}
