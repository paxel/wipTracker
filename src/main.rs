#![forbid(unsafe_code)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use wiptracker::app::{
    Backend, WAYLAND_NOTICE, WipTracker, backend_to_force, choose_backend, prefers_decorations,
};
use wiptracker::domain::ports::Store as _;
use wiptracker::infrastructure::launcher;
use wiptracker::infrastructure::redb_store::RedbStore;
use wiptracker::theme;

const USAGE: &str = "\
WipTracker — a one-line always-on-top bar showing the task you are focused on.

Usage: wiptracker [--version] [--help] [--reset-position]
                  [--install-launcher] [--remove-launcher]

  --reset-position    Open where the window manager wants to, and forget the stored
                      position. Use it when the bar is remembered somewhere you cannot
                      see it — a monitor that has been unplugged, or a layout that has
                      changed.
  --install-launcher  Write the launcher entry and icons into ~/.local/share, so every
                      application menu can find WipTracker, and exit. The app also
                      offers this on its own when no menu can see it.
  --remove-launcher   Delete that entry, the icons and an autostart copy, and exit.
                      After an uninstall the entry hides itself anyway: it names the
                      binary in TryExec, and menus drop entries whose binary is gone.

Everything else happens on the bar itself. See https://github.com/paxel/wipTracker for
what the clicks do.";

/// The taskbar icon: a small cat reading, drawn by `packaging/make_icon.py`.
///
/// It is embedded as raw RGBA rather than PNG so the app needs no image decoder; the
/// script writes exactly 64x64 pixels.
fn icon() -> egui::IconData {
    const SIDE: u32 = 64;
    let rgba = include_bytes!("../assets/icon.rgba").to_vec();
    debug_assert_eq!(rgba.len(), (SIDE * SIDE * 4) as usize);
    egui::IconData {
        rgba,
        width: SIDE,
        height: SIDE,
    }
}

/// Asks winit for a specific backend.
///
/// Without this it would pick Wayland whenever a Wayland session is running, even with
/// XWayland available — and on Wayland the bar cannot be kept above other windows at all.
#[cfg(target_os = "linux")]
fn ask_for_backend(options: &mut eframe::NativeOptions, backend: Backend) {
    use winit::platform::wayland::EventLoopBuilderExtWayland as _;
    use winit::platform::x11::EventLoopBuilderExtX11 as _;

    options.event_loop_builder = Some(Box::new(move |builder| {
        match backend {
            Backend::X11 => builder.with_x11(),
            Backend::Wayland => builder.with_wayland(),
        };
    }));
}

#[cfg(not(target_os = "linux"))]
fn ask_for_backend(_options: &mut eframe::NativeOptions, _backend: Backend) {}

fn main() -> eframe::Result<()> {
    let mut reset_position = false;
    if let Some(argument) = std::env::args().nth(1) {
        match argument.as_str() {
            "--version" | "-V" => {
                println!("wiptracker {}", env!("CARGO_PKG_VERSION"));
                return Ok(());
            }
            "--help" | "-h" => {
                println!("{USAGE}");
                return Ok(());
            }
            "--reset-position" => reset_position = true,
            "--install-launcher" => {
                let Some(data_home) = launcher::data_home() else {
                    eprintln!("wiptracker: no home directory to install into");
                    std::process::exit(1);
                };
                let exe = launcher::stable_exe();
                if let Err(error) = launcher::install_into(&data_home, &exe) {
                    eprintln!("wiptracker: {error}");
                    std::process::exit(1);
                }
                launcher::refresh_caches(&data_home);
                println!("Added to the application menu.");
                return Ok(());
            }
            "--remove-launcher" => {
                let (Some(data_home), Some(config_home)) =
                    (launcher::data_home(), launcher::config_home())
                else {
                    eprintln!("wiptracker: no home directory to remove from");
                    std::process::exit(1);
                };
                if let Err(error) = launcher::remove_from(&data_home, &config_home) {
                    eprintln!("wiptracker: {error}");
                    std::process::exit(1);
                }
                launcher::refresh_caches(&data_home);
                println!("Removed from the application menu.");
                return Ok(());
            }
            other => {
                eprintln!("wiptracker: unknown argument {other:?}\n\n{USAGE}");
                std::process::exit(2);
            }
        }
    }

    // Said once at startup as well as in the menu: someone starting the app from a
    // terminal on Wayland should not have to wonder why the bar keeps disappearing behind
    // other windows.
    let backend = choose_backend();
    if backend == Backend::Wayland {
        eprintln!("wiptracker: {WAYLAND_NOTICE}");
    }

    // The store is opened before the window so the bar can be placed where it was last
    // seen, and so an unreadable database is reported instead of being written over.
    let path = RedbStore::default_path();
    let opened = RedbStore::open(&path).and_then(|store| {
        let snapshot = store.load()?;
        Ok((store, snapshot))
    });

    let decorated = opened
        .as_ref()
        .ok()
        .and_then(|(_, snapshot)| snapshot.as_ref())
        .and_then(|snapshot| snapshot.decorated)
        .unwrap_or_else(prefers_decorations);
    let taskbar = opened
        .as_ref()
        .ok()
        .and_then(|(_, snapshot)| snapshot.as_ref())
        .and_then(|snapshot| snapshot.taskbar)
        .unwrap_or(true);

    // The bar is narrower when it wears a window frame, because the grip that drags an
    // undecorated window is not needed then.
    let size = theme::bar_size(decorated);
    let mut viewport = egui::ViewportBuilder::default()
        .with_title("WipTracker")
        .with_icon(icon())
        .with_app_id("wiptracker")
        .with_inner_size(size)
        .with_min_inner_size(size)
        .with_max_inner_size(size)
        .with_decorations(decorated)
        .with_always_on_top()
        .with_resizable(false);
    if !taskbar {
        // Windows honors this at creation. On X11 the app asks the window manager for
        // SKIP_TASKBAR itself once running — see `xdesk` — because winit has no API for
        // it. Native Wayland knows no taskbar states at all.
        viewport = viewport.with_taskbar(false);
    }
    if let Ok((_, Some(snapshot))) = &opened
        && let Some((x, y)) = snapshot.window_pos
        && !reset_position
    {
        viewport = viewport.with_position(egui::pos2(x, y));
    }

    let mut options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };
    if let Some(forced) = backend_to_force() {
        ask_for_backend(&mut options, forced);
    }

    eframe::run_native(
        "WipTracker",
        options,
        Box::new(move |cc| {
            let app = match opened {
                Ok((store, snapshot)) => WipTracker::start(cc, Box::new(store), snapshot),
                Err(error) => {
                    eprintln!("wiptracker: {path:?}: {error}");
                    WipTracker::broken(cc, &error)
                }
            };
            Ok(Box::new(app))
        }),
    )
}
