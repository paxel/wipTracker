//! Where a small window next to the bar goes.
//!
//! The only rule is that it has to be fully visible. Below the bar is where things end up
//! most of the time, simply because that is usually where they fit; a bar near the bottom
//! of the screen gets them above instead. Nothing here reaches the screen on Wayland,
//! where `with_position` is a no-op and the compositor decides.
//!
//! Positions are in desktop coordinates spanning every monitor, so the window has to be
//! kept inside the monitor the bar is actually on. Its rectangle comes from the platform
//! (see `infrastructure::screens`); where the platform does not say, egui's size of the
//! current monitor is used instead, and only when the bar lies inside a rectangle of
//! that size at the origin — see [`fits_one_monitor`]. Treating the monitor as if it
//! began at (0, 0) is what used to drag a menu off a second screen and onto the first.

use egui::{Pos2, Rect, Vec2};

/// The gap between the bar and a report window, so the frame does not touch the bar.
const WINDOW_GAP: f32 = 8.0;

/// Everything known about where the bar is, gathered once per frame.
///
/// `bar` is the bar's whole window in desktop coordinates — frame included, which is the
/// point: a decorated window is taller than the bar it contains, and placing a menu at
/// "top plus the bar's inner height" put it over the bar's own content, with just the
/// title bar peeking out. `None` on Wayland, where a position is unknowable.
///
/// `screen` is the monitor the bar is on, in the same coordinates, when that is known.
#[derive(Clone, Copy, Default, Debug, PartialEq)]
pub struct Placement {
    pub bar: Option<Rect>,
    pub screen: Option<Rect>,
}

/// Whether the monitor's own bounds can be read as desktop coordinates.
///
/// They can only be trusted when the bar lies inside a rectangle of that size at the
/// origin. A bar further right or further down than the monitor is proof of a second
/// monitor next to or above it, and then nothing here knows where anything begins.
fn fits_one_monitor(bar: Rect, monitor: Vec2) -> bool {
    bar.min.x >= 0.0 && bar.min.y >= 0.0 && bar.max.x <= monitor.x && bar.max.y <= monitor.y
}

/// The monitor the bar is on: the one it overlaps most. A bar straddling two goes with
/// the one holding more of it; a bar on none — dragged off every screen — has no monitor.
fn screen_for(bar: Rect, monitors: &[Rect]) -> Option<Rect> {
    monitors
        .iter()
        .copied()
        .map(|monitor| (monitor, monitor.intersect(bar)))
        .filter(|(_, overlap)| overlap.is_positive())
        .max_by(|(_, a), (_, b)| a.area().total_cmp(&b.area()))
        .map(|(monitor, _)| monitor)
}

impl Placement {
    /// Works out the screen from what the platform says, falling back to egui's
    /// size-only report where it says nothing.
    pub fn new(bar: Option<Rect>, monitors: &[Rect], monitor_size: Option<Vec2>) -> Self {
        let at_origin = monitor_size.map(|size| Rect::from_min_size(Pos2::ZERO, size));
        let screen = match bar {
            Some(bar) => screen_for(bar, monitors).or_else(|| {
                at_origin.filter(|_| monitor_size.is_some_and(|size| fits_one_monitor(bar, size)))
            }),
            // Without a bar position nothing is placed against it; the size is still
            // good for centring, which is all a window without a bar can be.
            None => at_origin,
        };
        Self { bar, screen }
    }

    /// Returns where a window of height `wanted` goes, and how tall it may actually be.
    ///
    /// The left edge is always the bar's own, never clamped. A bar half way across a
    /// second monitor still has coordinates smaller than that monitor's width, so no test
    /// can tell it apart from a bar on one wide screen — and clamping threw it back onto
    /// the first screen from that point on.
    ///
    /// The vertical flip stays, because falling off the bottom is the common case, and it
    /// is only applied where the monitor's bounds are known at all. Flipping puts the
    /// window's bottom against the bar's frame top, so it clears the title bar too.
    pub fn near_bar(&self, wanted: f32) -> (Option<Pos2>, f32) {
        let Some(bar) = self.bar else {
            return (None, wanted);
        };
        let below = egui::pos2(bar.min.x, bar.max.y);
        let Some(screen) = self.screen else {
            return (Some(below), wanted);
        };
        let height = wanted.min(screen.height());
        let top = if below.y + height <= screen.max.y {
            below.y
        } else {
            (bar.min.y - height).max(screen.min.y)
        };
        (Some(egui::pos2(bar.min.x, top)), height)
    }

    /// Where a report window of `size` opens: next to the bar with a small gap, or, with
    /// no bar position to go by, in the middle of the screen. `None` where nothing is
    /// known, and the window manager decides.
    pub fn window(&self, size: Vec2) -> Option<Pos2> {
        if let Some(bar) = self.bar {
            let (position, _) = self.near_bar(size.y + WINDOW_GAP);
            let mut position = position?;
            if position.y >= bar.max.y {
                position.y += WINDOW_GAP;
            }
            return Some(position);
        }
        let screen = self.screen?;
        Some(screen.min + ((screen.size() - size) / 2.0).max(Vec2::ZERO))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ONE_SCREEN: Vec2 = Vec2::new(1920.0, 1080.0);

    /// A frameless bar on a machine that reports only the monitor's size: the window is
    /// exactly the bar.
    fn bare(x: f32, y: f32) -> Placement {
        Placement::new(
            Some(Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(304.0, 32.0),
            )),
            &[],
            Some(ONE_SCREEN),
        )
    }

    /// A decorated bar: the frame adds a title bar above the 32 pixel content.
    fn decorated(x: f32, y: f32) -> Placement {
        Placement::new(
            Some(Rect::from_min_size(
                egui::pos2(x, y),
                egui::vec2(294.0, 60.0),
            )),
            &[],
            Some(ONE_SCREEN),
        )
    }

    /// Three monitors stacked: the primary at the bottom, two above it with negative y —
    /// the layout the misplaced report windows were seen on.
    fn stacked() -> Vec<Rect> {
        vec![
            Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(2560.0, 1440.0)),
            Rect::from_min_size(egui::pos2(0.0, -1440.0), egui::vec2(2560.0, 1440.0)),
            Rect::from_min_size(egui::pos2(0.0, -2520.0), egui::vec2(1920.0, 1080.0)),
        ]
    }

    fn bar_at(x: f32, y: f32) -> Rect {
        Rect::from_min_size(egui::pos2(x, y), egui::vec2(304.0, 32.0))
    }

    #[test]
    fn a_window_that_fits_below_goes_below() {
        let (position, height) = bare(100.0, 40.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(100.0, 72.0)));
        assert_eq!(height, 350.0);
    }

    /// The regression the mac report was about: with a window frame, "the bar's top plus
    /// 32" is inside the window, and the menu covered everything but the title bar. The
    /// window's real bottom is what counts.
    #[test]
    fn a_decorated_bar_gets_the_window_below_its_frame() {
        let (position, _) = decorated(100.0, 40.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(100.0, 100.0)));
    }

    #[test]
    fn a_bar_at_the_bottom_gets_the_window_above_it() {
        let (position, _) = bare(100.0, 1000.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(100.0, 650.0)));
    }

    /// Flipping above a decorated bar clears the whole frame, not just the content.
    #[test]
    fn flipping_above_a_decorated_bar_clears_the_title_bar() {
        let (position, _) = decorated(100.0, 1000.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(100.0, 650.0)));
    }

    /// A tall window on a short screen is pulled to the top edge and cut to fit, rather
    /// than being pushed off it.
    #[test]
    fn a_window_taller_than_the_screen_is_clamped() {
        let placement = Placement::new(
            Some(bar_at(0.0, 500.0)),
            &[],
            Some(Vec2::new(1024.0, 600.0)),
        );
        let (position, height) = placement.near_bar(900.0);
        assert_eq!(height, 600.0);
        assert_eq!(position.map(|p| p.y), Some(0.0));
    }

    /// Never pulled sideways. Half way across a second monitor the coordinates still look
    /// like a single wide screen, and clamping there is what moved the menu to the wrong
    /// monitor.
    #[test]
    fn the_left_edge_is_always_the_bar_own() {
        for x in [0.0, 1800.0, 3600.0, 5000.0] {
            let (position, _) = bare(x, 40.0).near_bar(350.0);
            assert_eq!(
                position.map(|p| p.x),
                Some(x),
                "the window follows the bar at x = {x}"
            );
        }
    }

    /// A bar past the right edge of the reported monitor is on a second one, whose origin
    /// is unknowable without the platform's help; the bar's own position is simply kept.
    #[test]
    fn a_bar_on_a_monitor_to_the_right_keeps_its_own_coordinates() {
        let (position, height) = bare(4000.0, 200.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(4000.0, 232.0)));
        assert_eq!(height, 350.0);
    }

    /// The same below the reported monitor: a laptop panel mounted under an external
    /// screen has coordinates larger than its own height.
    #[test]
    fn a_bar_on_a_monitor_below_keeps_its_own_coordinates() {
        let (position, _) = bare(300.0, 1500.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(300.0, 1532.0)));
    }

    /// A negative coordinate means a monitor to the left of the primary one.
    #[test]
    fn a_bar_on_a_monitor_to_the_left_keeps_its_own_coordinates() {
        let (position, _) = bare(-1200.0, 40.0).near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(-1200.0, 72.0)));
    }

    #[test]
    fn no_known_position_means_no_placement() {
        let placement = Placement::new(None, &[], Some(ONE_SCREEN));
        let (position, height) = placement.near_bar(350.0);
        assert_eq!(position, None);
        assert_eq!(height, 350.0);
    }

    /// With the monitors known, a bar on the top screen of the stack is placed against
    /// that screen: below when there is room, above when there is not, and never on the
    /// primary screen's coordinates.
    #[test]
    fn a_bar_on_an_upper_monitor_is_placed_inside_it() {
        let high = Placement::new(Some(bar_at(100.0, -2500.0)), &stacked(), Some(ONE_SCREEN));
        assert_eq!(high.screen, Some(stacked()[2]));
        let (position, _) = high.near_bar(350.0);
        assert_eq!(position, Some(egui::pos2(100.0, -2468.0)));

        let low = Placement::new(Some(bar_at(100.0, -1500.0)), &stacked(), Some(ONE_SCREEN));
        let (position, _) = low.near_bar(350.0);
        assert_eq!(
            position,
            Some(egui::pos2(100.0, -1850.0)),
            "flipped above, still inside the top screen"
        );
    }

    /// Clamping a tall window respects the top of the screen it is on, not y = 0.
    #[test]
    fn a_tall_window_on_an_upper_monitor_is_clamped_to_that_monitor() {
        let placement = Placement::new(Some(bar_at(0.0, -2000.0)), &stacked(), Some(ONE_SCREEN));
        let (position, height) = placement.near_bar(5000.0);
        assert_eq!(height, 1080.0);
        assert_eq!(position, Some(egui::pos2(0.0, -2520.0)));
    }

    /// A bar straddling two monitors goes with the one holding more of it.
    #[test]
    fn a_straddling_bar_belongs_to_the_monitor_holding_most_of_it() {
        let mostly_upper = Placement::new(Some(bar_at(0.0, -20.0)), &stacked(), None);
        assert_eq!(mostly_upper.screen, Some(stacked()[1]));
        let mostly_lower = Placement::new(Some(bar_at(0.0, -10.0)), &stacked(), None);
        assert_eq!(mostly_lower.screen, Some(stacked()[0]));
    }

    /// A bar on none of the monitors has no screen; its own coordinates are kept.
    #[test]
    fn a_bar_off_every_monitor_keeps_its_own_coordinates() {
        let placement = Placement::new(Some(bar_at(9000.0, 9000.0)), &stacked(), None);
        assert_eq!(placement.screen, None);
        assert_eq!(
            placement.near_bar(350.0).0,
            Some(egui::pos2(9000.0, 9032.0))
        );
    }

    /// The platform's answer wins over the size-only guess, even where the guess would
    /// have been accepted.
    #[test]
    fn the_platform_monitors_win_over_the_size_guess() {
        let placement = Placement::new(Some(bar_at(100.0, 100.0)), &stacked(), Some(ONE_SCREEN));
        assert_eq!(placement.screen, Some(stacked()[0]));
    }

    /// A report window sits a gap below the bar, or a gap above it when flipped.
    #[test]
    fn a_report_window_keeps_a_gap_from_the_bar() {
        let size = egui::vec2(420.0, 300.0);
        assert_eq!(
            bare(100.0, 40.0).window(size),
            Some(egui::pos2(100.0, 80.0))
        );
        assert_eq!(
            bare(100.0, 1000.0).window(size),
            Some(egui::pos2(100.0, 692.0)),
            "above: bottom edge a gap over the bar's top"
        );
    }

    /// Without a bar position the window is centred on the screen; without a screen it
    /// is left to the window manager.
    #[test]
    fn a_report_window_without_a_bar_is_centred_or_left_alone() {
        let size = egui::vec2(420.0, 300.0);
        let centred = Placement::new(None, &[], Some(ONE_SCREEN));
        assert_eq!(centred.window(size), Some(egui::pos2(750.0, 390.0)));
        let unknown = Placement::new(None, &[], None);
        assert_eq!(unknown.window(size), None);
    }
}
