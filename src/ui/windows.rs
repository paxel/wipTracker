//! The windows behind the menu: the task stack, timers, groom, end day, week and revive.
//!
//! Each is a normal decorated window of its own, so it can be moved, resized and closed
//! like any other; the bar keeps working while they are open.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

use chrono::{DateTime, Datelike as _, Days, Local, NaiveDate};
use egui::{Context, RichText, ViewportBuilder, ViewportId};

use crate::domain::export;
use crate::domain::task::TaskId;
use crate::domain::tracker::Tracker;
use crate::theme;
use crate::ui::format;

/// How far back the revive window looks.
pub const REVIVE_DAYS: i64 = 30;

/// Which report windows are open, and the little bit of state they each remember.
#[derive(Default)]
pub struct OpenWindows {
    pub stack: bool,
    pub timer: bool,
    pub groom: bool,
    pub end_day: bool,
    pub week: bool,
    pub revive: bool,
    groom_selection: BTreeSet<TaskId>,
    /// Any day inside the week the week view is showing.
    week_anchor: Option<NaiveDate>,
    /// What the user typed into the week view's date field.
    week_input: String,
    /// Which row of the timer window has its duration picker open. `None` is the default
    /// timer's row.
    timer_editing: Option<Option<TaskId>>,
    /// Whether each window is showing its "copied to clipboard" confirmation.
    end_day_copied: bool,
    week_copied: bool,
    /// Where each window was placed when it opened. Recomputing this every frame would
    /// re-issue `with_position`, which drags an open report window along with the bar.
    placed: BTreeMap<&'static str, egui::Pos2>,
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
        if ui
            .button("export")
            .on_hover_text("Copy this data as JSON, one row per task per day")
            .clicked()
        {
            *payload = Some(export::to_json(tracker, days));
            *copied = true;
        }
        if *copied {
            ui.label(
                RichText::new("copied to clipboard")
                    .color(theme::TEXT_DIM)
                    .small(),
            );
        }
    });
}

/// Where a report window should open.
///
/// Without a position the window manager decides, and it parks every window in the
/// top-left corner. Near the bar is the useful answer when the bar's own position is
/// known; centred on the screen is the fallback. On Wayland neither is possible — a client
/// there can neither read nor set a window position — so the compositor keeps deciding.
fn placement(ctx: &Context, size: [f32; 2]) -> Option<egui::Pos2> {
    let bar = ctx.input(|i| i.viewport().outer_rect);
    if let Some(bar) = bar {
        // Below the bar, left edges aligned, nudged left if that would run off the screen.
        let x = bar.min.x;
        let y = bar.max.y + 8.0;
        return Some(egui::pos2(x, y));
    }
    let monitor = ctx.input(|i| i.viewport().monitor_size)?;
    Some(egui::pos2(
        (monitor.x - size[0]).max(0.0) / 2.0,
        (monitor.y - size[1]).max(0.0) / 2.0,
    ))
}

fn window(
    ctx: &Context,
    id: &'static str,
    title: &str,
    size: [f32; 2],
    placed: &mut BTreeMap<&'static str, egui::Pos2>,
    open: &mut bool,
    contents: impl FnOnce(&mut egui::Ui),
) {
    let mut builder = ViewportBuilder::default()
        .with_title(title)
        .with_inner_size(size)
        .with_min_inner_size([320.0, 200.0]);
    // Decided once, when the window opens: the bar can be dragged afterwards, and a fresh
    // position on every frame would drag the report window along with it.
    let position = placed.get(id).copied().or_else(|| {
        placement(ctx, size).inspect(|position| {
            placed.insert(id, *position);
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
        placed.remove(id);
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
    let mut placed = std::mem::take(&mut open.placed);
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
        &mut placed,
        &mut still_open,
        |ui| {
            ui.heading("Task stack");
            ui.label(
                RichText::new("Top of the stack first. Click a task to work on it again.")
                    .color(theme::TEXT_DIM)
                    .small(),
            );
            ui.add_space(6.0);

            egui::Grid::new("stack_rows")
                .num_columns(3)
                .striped(true)
                .min_col_width(120.0)
                .show(ui, |ui| {
                    for (id, name, today_time, total) in &rows {
                        let current = *id == focused;
                        if current {
                            ui.label(
                                RichText::new(format!("● {name}"))
                                    .color(theme::TEXT)
                                    .strong(),
                            )
                            .on_hover_text("The task you are working on right now");
                        } else if ui
                            .button(RichText::new(name).color(theme::TEXT))
                            .on_hover_text("Left click: work on this task")
                            .clicked()
                        {
                            pick = Some(*id);
                        }
                        ui.label(
                            RichText::new(format!("today {}", format::coarse(*today_time)))
                                .color(theme::TEXT_DIM),
                        );
                        ui.label(
                            RichText::new(format!("total {}", format::coarse(*total)))
                                .color(theme::TEXT_DIM),
                        );
                        ui.end_row();
                    }
                });
        },
    );

    open.placed = placed;
    open.stack = still_open;
    if let Some(id) = pick {
        let _ = tracker.select(id, now);
        open.stack = false;
        return true;
    }
    false
}

/// The timer window: one row per open task plus the default that new tasks inherit.
fn timer(ctx: &Context, open: &mut OpenWindows, tracker: &mut Tracker) -> bool {
    let mut placed = std::mem::take(&mut open.placed);
    let default_timer = tracker.default_timer();
    let rows: Vec<(Option<TaskId>, String, Duration)> =
        std::iter::once((None, "default for new tasks".to_owned(), default_timer))
            .chain(
                tracker
                    .open_tasks_top_first()
                    .iter()
                    .map(|task| (Some(task.id), task.name.clone(), task.timer)),
            )
            .collect();

    let mut editing = open.timer_editing;
    let mut chosen: Option<(Option<TaskId>, Duration)> = None;
    let mut still_open = true;

    window(
        ctx,
        "timer",
        "WipTracker — timers",
        [420.0, 340.0],
        &mut placed,
        &mut still_open,
        |ui| {
            ui.heading("Daily timers");
            ui.label(
                RichText::new(
                    "When a task has been worked on this long today, WipTracker beeps once. \
                     Zero means no alarm.",
                )
                .color(theme::TEXT_DIM)
                .small(),
            );
            ui.add_space(6.0);

            for (id, name, timer) in &rows {
                ui.horizontal(|ui| {
                    let label = if timer.is_zero() {
                        "off".to_owned()
                    } else {
                        format::coarse(*timer)
                    };
                    if ui
                        .button(RichText::new(format!("{name}   {label}")).color(theme::TEXT))
                        .on_hover_text("Left click: choose how long this may run per day")
                        .clicked()
                    {
                        editing = if editing == Some(*id) {
                            None
                        } else {
                            Some(*id)
                        };
                    }
                });
                if editing == Some(*id) {
                    ui.horizontal_wrapped(|ui| {
                        for (label, minutes) in [
                            ("off", 0_u64),
                            ("15m", 15),
                            ("30m", 30),
                            ("1h", 60),
                            ("2h", 120),
                            ("4h", 240),
                            ("8h", 480),
                        ] {
                            if ui.button(label).clicked() {
                                chosen = Some((*id, Duration::from_secs(minutes * 60)));
                            }
                        }
                    });
                    ui.add_space(4.0);
                }
            }
        },
    );

    open.placed = placed;
    open.timer = still_open;
    open.timer_editing = editing;
    if let Some((id, duration)) = chosen {
        match id {
            Some(id) => {
                let _ = tracker.set_timer(id, duration);
            }
            None => tracker.set_default_timer(duration),
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
    let mut placed = std::mem::take(&mut open.placed);
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
        &mut placed,
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
                    let box_response =
                        ui.checkbox(&mut checked, RichText::new(name).color(theme::TEXT));
                    if box_response.changed() {
                        if checked {
                            selection.insert(*id);
                        } else {
                            selection.remove(id);
                        }
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(RichText::new(format::coarse(*total)).color(theme::TEXT_DIM));
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

    open.placed = placed;
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
    let mut placed = std::mem::take(&mut open.placed);
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
        &mut placed,
        &mut still_open,
        |ui| {
            ui.heading(format!("{today}"));
            ui.add_space(4.0);
            let started = record
                .started_at
                .map_or("—".to_owned(), |time| time.format("%H:%M").to_string());
            let ended = record.ended_at.map_or_else(
                || now.format("%H:%M").to_string(),
                |time| time.format("%H:%M").to_string(),
            );
            ui.label(format!("Day started {started}, last activity {ended}"));
            if record.closed {
                ui.label(RichText::new("This day is closed.").color(theme::TEXT_DIM));
            }
            ui.separator();

            if rows.is_empty() {
                ui.label("No time collected today yet.");
            } else {
                egui::Grid::new("end_day_rows")
                    .num_columns(2)
                    .striped(true)
                    .min_col_width(160.0)
                    .show(ui, |ui| {
                        for (name, duration) in &rows {
                            ui.label(RichText::new(name).color(theme::TEXT));
                            ui.label(
                                RichText::new(format::coarse(*duration)).color(theme::TEXT_DIM),
                            );
                            ui.end_row();
                        }
                        ui.label(RichText::new("total").strong());
                        ui.label(RichText::new(format::coarse(record.total())).strong());
                        ui.end_row();
                    });
            }

            ui.add_space(8.0);
            export_button(ui, tracker, &[today], &mut copied, &mut payload);
            ui.add_space(8.0);
            if ui.button("Close day").clicked() {
                close_day = true;
            }
            ui.label(
                RichText::new(
                    "Closing the day stamps its end time, saves, and quits. Open tasks stay \
                     on the stack for tomorrow.",
                )
                .color(theme::TEXT_DIM)
                .small(),
            );
        },
    );

    open.placed = placed;
    open.end_day = still_open;
    open.end_day_copied = copied && still_open;
    if close_day {
        tracker.close_day(now);
        return (true, payload);
    }
    (false, payload)
}

/// Returns any JSON the user asked to copy.
fn week(
    ctx: &Context,
    open: &mut OpenWindows,
    tracker: &Tracker,
    now: DateTime<Local>,
) -> Option<String> {
    let mut placed = std::mem::take(&mut open.placed);
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
        let total = per_day.iter().sum();
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
        &mut placed,
        &mut still_open,
        |ui| {
            ui.horizontal(|ui| {
                if ui.button("◀ previous").clicked() {
                    anchor_change = monday.checked_sub_days(Days::new(7));
                }
                if ui.button("today").clicked() {
                    anchor_change = Some(now.date_naive());
                }
                if ui.button("next ▶").clicked() {
                    anchor_change = monday.checked_add_days(Days::new(7));
                }
                ui.label(
                    RichText::new(format!(
                        "week of {monday} (calendar week {})",
                        monday.iso_week().week()
                    ))
                    .color(theme::TEXT_DIM),
                );
            });
            ui.horizontal(|ui| {
                ui.label(RichText::new("jump to date").color(theme::TEXT_DIM));
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
            export_button(ui, tracker, &week_days, &mut copied, &mut payload);
            ui.separator();

            if task_rows.is_empty() {
                ui.label("Nothing was tracked in this week.");
                return;
            }

            egui::Grid::new("week_grid")
                .num_columns(9)
                .striped(true)
                .min_col_width(64.0)
                .show(ui, |ui| {
                    ui.label(RichText::new("task").strong());
                    for day in &days {
                        ui.label(RichText::new(day.format("%a %d").to_string()).strong());
                    }
                    ui.label(RichText::new("total").strong());
                    ui.end_row();

                    for (name, per_day, total) in &task_rows {
                        ui.label(RichText::new(name).color(theme::TEXT));
                        for duration in per_day {
                            ui.label(if duration.is_zero() {
                                RichText::new("·").color(theme::TEXT_DIM)
                            } else {
                                RichText::new(format::coarse(*duration)).color(theme::TEXT)
                            });
                        }
                        ui.label(RichText::new(format::coarse(*total)).strong());
                        ui.end_row();
                    }

                    ui.label(RichText::new("total").strong());
                    let mut week_total = Duration::ZERO;
                    for (index, _) in days.iter().enumerate() {
                        let column: Duration =
                            task_rows.iter().map(|(_, per_day, _)| per_day[index]).sum();
                        week_total += column;
                        ui.label(RichText::new(format::coarse(column)).strong());
                    }
                    ui.label(RichText::new(format::coarse(week_total)).strong());
                    ui.end_row();
                });
        },
    );

    open.placed = placed;
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
    let mut placed = std::mem::take(&mut open.placed);
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
        &mut placed,
        &mut still_open,
        |ui| {
            ui.heading("Finished tasks");
            ui.label(
                RichText::new(format!(
                    "The last {REVIVE_DAYS} days. Older tasks stay in the week overview."
                ))
                .color(theme::TEXT_DIM)
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
                        if ui.button(RichText::new(name).color(theme::TEXT)).clicked() {
                            revive_id = Some(*id);
                        }
                        ui.label(RichText::new(format::coarse(*total)).color(theme::TEXT_DIM));
                        ui.label(RichText::new(finished_at).color(theme::TEXT_DIM));
                        ui.end_row();
                    }
                });
        },
    );

    open.placed = placed;
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
