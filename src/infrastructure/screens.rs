//! Where the monitors are, in the coordinates egui reports window positions in.
//!
//! egui tells the app the size of the monitor the bar is on, and nothing about where that
//! monitor begins — while window positions span the whole desktop. With one monitor the
//! two agree; with several, a menu placed by size alone lands on the wrong screen, or
//! behind the bar, or nowhere reachable. So the app asks the platform for every monitor's
//! rectangle itself and converts it into egui's space, which is
//! `physical pixels / pixels_per_point`, the y axis pointing down, the primary monitor's
//! top-left at the origin.
//!
//! - macOS: `NSScreen` frames, in points with the y axis pointing up from the bottom of
//!   the primary screen. Flipped the way winit flips window frames, then divided by the
//!   zoom factor, since points are already `physical / native scale`.
//! - X11: RandR monitors, in physical root-window pixels; divided by `pixels_per_point`.
//! - Windows: `EnumDisplayMonitors`, in physical virtual-screen pixels; the same.
//! - Wayland: nothing, and it does not matter, since a Wayland client can neither read
//!   nor set a window position.
//!
//! Every platform can answer `None`; the placement then falls back to the size-only
//! guess it always made.

use egui::{Rect, pos2, vec2};

/// A monitor's rectangle in the platform's own units: left, top, width, height, y down.
pub type Raw = (f64, f64, f64, f64);

/// Rectangles in whatever unit the platform reports, scaled into egui points.
fn scaled(raw: &[Raw], divisor: f32) -> Vec<Rect> {
    raw.iter()
        .map(|&(x, y, w, h)| {
            Rect::from_min_size(
                pos2(x as f32 / divisor, y as f32 / divisor),
                vec2(w as f32 / divisor, h as f32 / divisor),
            )
        })
        .collect()
}

/// Cocoa screen frames — `(x, y, width, height)` with y pointing up — turned into the
/// y-down space winit reports window positions in. The first frame is the primary
/// screen, whose origin is always `(0, 0)` and whose height is what everything flips
/// against; this is exactly winit's `flip_window_screen_coordinates`.
///
/// Only macOS reports frames this way; the function is kept on every platform so its
/// tests run where the tests run.
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
fn flipped(frames: &[Raw]) -> Vec<Raw> {
    let Some(&(_, _, _, primary_height)) = frames.first() else {
        return Vec::new();
    };
    frames
        .iter()
        .map(|&(x, y, w, h)| (x, primary_height - h - y, w, h))
        .collect()
}

/// The connection to whatever answers the question, opened once and kept.
pub struct Screens {
    #[cfg(target_os = "linux")]
    x11: Option<x11::X11>,
}

impl Screens {
    /// Connects where a connection is needed. Never fails: a platform that cannot answer
    /// simply answers `None` from [`Screens::monitors`].
    pub fn connect() -> Self {
        Self {
            #[cfg(target_os = "linux")]
            x11: x11::X11::connect(),
        }
    }

    /// Every monitor, in egui points, or `None` where the platform does not say.
    ///
    /// `pixels_per_point` is what egui divides physical pixels by; `zoom_factor` is the
    /// part of it that is not the native scale.
    pub fn monitors(&self, pixels_per_point: f32, zoom_factor: f32) -> Option<Vec<Rect>> {
        let raw = self.raw(pixels_per_point, zoom_factor)?;
        Some(scaled(&raw.0, raw.1))
    }

    /// The platform's rectangles together with what to divide them by.
    #[cfg(target_os = "macos")]
    fn raw(&self, _pixels_per_point: f32, zoom_factor: f32) -> Option<(Vec<Raw>, f32)> {
        Some((flipped(&macos::frames()?), zoom_factor))
    }

    #[cfg(target_os = "linux")]
    fn raw(&self, pixels_per_point: f32, _zoom_factor: f32) -> Option<(Vec<Raw>, f32)> {
        Some((self.x11.as_ref()?.monitors()?, pixels_per_point))
    }

    #[cfg(target_os = "windows")]
    fn raw(&self, pixels_per_point: f32, _zoom_factor: f32) -> Option<(Vec<Raw>, f32)> {
        Some((windows::monitors()?, pixels_per_point))
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    fn raw(&self, _pixels_per_point: f32, _zoom_factor: f32) -> Option<(Vec<Raw>, f32)> {
        None
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use super::Raw;
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSScreen;

    /// Every screen's frame as Cocoa reports it, the primary one first. `None` off the
    /// main thread, where AppKit must not be asked.
    pub fn frames() -> Option<Vec<Raw>> {
        let mtm = MainThreadMarker::new()?;
        let frames: Vec<Raw> = NSScreen::screens(mtm)
            .iter()
            .map(|screen| {
                let frame = screen.frame();
                (
                    frame.origin.x,
                    frame.origin.y,
                    frame.size.width,
                    frame.size.height,
                )
            })
            .collect();
        Some(frames)
    }
}

#[cfg(target_os = "linux")]
mod x11 {
    use super::Raw;
    use x11rb::connection::Connection as _;
    use x11rb::protocol::randr::ConnectionExt as _;
    use x11rb::protocol::xproto::Window;
    use x11rb::rust_connection::RustConnection;

    /// RandR monitors need protocol 1.5, which every server since 2015 speaks.
    const RANDR_MAJOR: u32 = 1;
    const RANDR_MINOR: u32 = 5;

    pub struct X11 {
        conn: RustConnection,
        root: Window,
    }

    impl X11 {
        /// `None` where there is no X server, or one too old to list monitors.
        pub fn connect() -> Option<Self> {
            let (conn, screen) = x11rb::connect(None).ok()?;
            let root = conn.setup().roots.get(screen)?.root;
            let version = conn
                .randr_query_version(RANDR_MAJOR, RANDR_MINOR)
                .ok()?
                .reply()
                .ok()?;
            if (version.major_version, version.minor_version) < (RANDR_MAJOR, RANDR_MINOR) {
                return None;
            }
            Some(Self { conn, root })
        }

        /// The active monitors, in physical root-window pixels.
        pub fn monitors(&self) -> Option<Vec<Raw>> {
            let reply = self
                .conn
                .randr_get_monitors(self.root, true)
                .ok()?
                .reply()
                .ok()?;
            let monitors: Vec<Raw> = reply
                .monitors
                .iter()
                .map(|monitor| {
                    (
                        f64::from(monitor.x),
                        f64::from(monitor.y),
                        f64::from(monitor.width),
                        f64::from(monitor.height),
                    )
                })
                .collect();
            (!monitors.is_empty()).then_some(monitors)
        }
    }
}

#[cfg(target_os = "windows")]
#[allow(unsafe_code)]
mod windows {
    use super::Raw;
    use windows_sys::Win32::Foundation::{LPARAM, RECT};
    use windows_sys::Win32::Graphics::Gdi::{EnumDisplayMonitors, HDC, HMONITOR};

    /// Every monitor's rectangle in virtual-screen pixels — the space `GetWindowRect`,
    /// and so winit, reports window positions in.
    pub fn monitors() -> Option<Vec<Raw>> {
        let mut found: Vec<Raw> = Vec::new();
        // SAFETY: `EnumDisplayMonitors` calls `collect` synchronously, once per monitor,
        // and hands it back the pointer to `found` passed as `lparam`; nothing else
        // holds that pointer and the vector outlives the call.
        let ok = unsafe {
            EnumDisplayMonitors(
                std::ptr::null_mut(),
                std::ptr::null(),
                Some(collect),
                &mut found as *mut Vec<Raw> as LPARAM,
            )
        };
        (ok != 0 && !found.is_empty()).then_some(found)
    }

    unsafe extern "system" fn collect(
        _monitor: HMONITOR,
        _context: HDC,
        rect: *mut RECT,
        found: LPARAM,
    ) -> i32 {
        // SAFETY: both pointers are the ones `monitors` describes above, valid for the
        // duration of this call.
        unsafe {
            let rect = &*rect;
            let found = &mut *(found as *mut Vec<Raw>);
            found.push((
                f64::from(rect.left),
                f64::from(rect.top),
                f64::from(rect.right - rect.left),
                f64::from(rect.bottom - rect.top),
            ));
        }
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The three-monitor stack that started this: the primary at the bottom, two more
    /// above it. In Cocoa the ones above have positive y; in winit's space, negative.
    #[test]
    fn cocoa_frames_are_flipped_against_the_primary_screen() {
        let frames = [
            (0.0, 0.0, 2560.0, 1440.0),
            (0.0, 1440.0, 2560.0, 1440.0),
            (0.0, 2880.0, 1920.0, 1080.0),
        ];
        assert_eq!(
            flipped(&frames),
            vec![
                (0.0, 0.0, 2560.0, 1440.0),
                (0.0, -1440.0, 2560.0, 1440.0),
                (0.0, -2520.0, 1920.0, 1080.0),
            ]
        );
    }

    /// A screen below the primary one — a laptop under an external display — has a
    /// negative y in Cocoa and a positive one after the flip.
    #[test]
    fn a_screen_below_the_primary_lands_below_it() {
        let frames = [(0.0, 0.0, 1920.0, 1080.0), (300.0, -900.0, 1440.0, 900.0)];
        assert_eq!(flipped(&frames)[1], (300.0, 1080.0, 1440.0, 900.0));
    }

    #[test]
    fn no_frames_flip_to_nothing() {
        assert!(flipped(&[]).is_empty());
    }

    /// Physical pixels on a 2x display become half as many points.
    #[test]
    fn raw_rectangles_are_scaled_into_points() {
        let rects = scaled(&[(3840.0, -2160.0, 3840.0, 2160.0)], 2.0);
        assert_eq!(
            rects,
            vec![Rect::from_min_size(
                pos2(1920.0, -1080.0),
                vec2(1920.0, 1080.0)
            )]
        );
    }

    /// On a machine with an X server the real answer has at least one monitor of some
    /// size; anywhere else the question is answered with a clean `None`.
    #[test]
    fn the_platform_answers_or_declines() {
        let screens = Screens::connect();
        if let Some(monitors) = screens.monitors(1.0, 1.0) {
            assert!(!monitors.is_empty());
            for monitor in monitors {
                assert!(monitor.width() > 0.0 && monitor.height() > 0.0);
            }
        }
    }
}
