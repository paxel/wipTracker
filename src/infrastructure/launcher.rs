//! Installing the launcher entry and icons into the user's own XDG directories.
//!
//! A desktop only lists applications whose `.desktop` entry sits in a directory on the
//! session's `XDG_DATA_DIRS` — and a Homebrew prefix never is, so a brew-installed
//! WipTracker is invisible to every menu until its entry is copied into
//! `~/.local/share`. A package manager must not write into `$HOME`, but the app itself,
//! asked by its user, may.
//!
//! The entry names the binary by absolute path in `Exec` and in `TryExec`. `TryExec` is
//! what makes uninstalling clean without any hook: a desktop hides an entry whose
//! `TryExec` no longer resolves, so removing the binary removes the menu entry from
//! sight, wherever the entry file itself came from.

use std::io;
use std::path::{Path, PathBuf};

/// The icon in every size a menu draws at, embedded so the installer needs no files
/// around it. `make_icon.py` writes these; the 512 pixel one keeps its plain name.
const ICONS: &[(u32, &[u8])] = &[
    (32, include_bytes!("../../assets/icon-32.png")),
    (48, include_bytes!("../../assets/icon-48.png")),
    (64, include_bytes!("../../assets/icon-64.png")),
    (128, include_bytes!("../../assets/icon-128.png")),
    (256, include_bytes!("../../assets/icon-256.png")),
    (512, include_bytes!("../../assets/icon.png")),
];

/// The entry the packages ship, kept as the single source of what it says.
const ENTRY: &str = include_str!("../../packaging/wiptracker.desktop");

/// Where the user's own XDG data lives: `$XDG_DATA_HOME`, or `~/.local/share`.
pub fn data_home() -> Option<PathBuf> {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
}

/// Every directory the session finds applications in: the user's own, then
/// `$XDG_DATA_DIRS` or its default.
fn data_dirs() -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = data_home().into_iter().collect();
    let system = std::env::var("XDG_DATA_DIRS").unwrap_or_default();
    if system.is_empty() {
        dirs.push(PathBuf::from("/usr/local/share"));
        dirs.push(PathBuf::from("/usr/share"));
    } else {
        dirs.extend(
            system
                .split(':')
                .filter(|d| !d.is_empty())
                .map(PathBuf::from),
        );
    }
    dirs
}

/// Whether any menu on this session can see a WipTracker entry.
pub fn is_visible() -> bool {
    data_dirs()
        .iter()
        .any(|dir| dir.join("applications/wiptracker.desktop").exists())
}

/// The entry as it is written: the shipped one, with the binary named absolutely.
fn entry_for(exe: &Path) -> String {
    let exe = exe.display();
    let mut written = String::new();
    for line in ENTRY.lines() {
        if let Some(rest) = line.strip_prefix("Exec=") {
            let _ = rest;
            written.push_str(&format!("Exec={exe}\n"));
            // Hidden automatically once the binary is gone — see the module notes.
            written.push_str(&format!("TryExec={exe}\n"));
        } else {
            written.push_str(line);
            written.push('\n');
        }
    }
    written
}

/// Writes the launcher entry and the icons under `data_home`.
pub fn install_into(data_home: &Path, exe: &Path) -> io::Result<()> {
    let applications = data_home.join("applications");
    std::fs::create_dir_all(&applications)?;
    std::fs::write(applications.join("wiptracker.desktop"), entry_for(exe))?;
    for (size, bytes) in ICONS {
        let dir = data_home.join(format!("icons/hicolor/{size}x{size}/apps"));
        std::fs::create_dir_all(&dir)?;
        std::fs::write(dir.join("wiptracker.png"), bytes)?;
    }
    Ok(())
}

/// Removes everything [`install_into`] wrote, plus an autostart copy if one exists.
/// Missing files are fine: removing what is already gone is success.
pub fn remove_from(data_home: &Path, config_home: &Path) -> io::Result<()> {
    let mut targets = vec![
        data_home.join("applications/wiptracker.desktop"),
        config_home.join("autostart/wiptracker.desktop"),
    ];
    for (size, _) in ICONS {
        targets.push(data_home.join(format!("icons/hicolor/{size}x{size}/apps/wiptracker.png")));
    }
    for target in targets {
        match std::fs::remove_file(&target) {
            Ok(()) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
    }
    Ok(())
}

/// `$XDG_CONFIG_HOME`, or `~/.config`.
pub fn config_home() -> Option<PathBuf> {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
}

/// Whether WipTracker starts with the session.
///
/// `None` where this cannot be known, which disables the menu toggle.
pub fn autostart_enabled() -> Option<bool> {
    #[cfg(target_os = "linux")]
    {
        Some(config_home()?.join("autostart/wiptracker.desktop").exists())
    }
    #[cfg(target_os = "windows")]
    {
        let shortcut = startup_dir()?.join("WipTracker.cmd");
        Some(shortcut.exists())
    }
    #[cfg(target_os = "macos")]
    {
        let output = std::process::Command::new("osascript")
            .args([
                "-e",
                "tell application \"System Events\" to get the name of every login item",
            ])
            .output()
            .ok()?;
        Some(String::from_utf8_lossy(&output.stdout).contains("WipTracker"))
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        None
    }
}

/// Switches starting with the session on or off.
pub fn set_autostart(enabled: bool, exe: &Path) -> io::Result<()> {
    #[cfg(target_os = "linux")]
    {
        let config = config_home().ok_or_else(|| io::Error::other("no home directory"))?;
        set_autostart_in(&config, enabled, exe)
    }
    #[cfg(target_os = "windows")]
    {
        // A .cmd in the Startup folder: no COM, no registry, removable by hand.
        let target = startup_dir()
            .ok_or_else(|| io::Error::other("no Startup folder"))?
            .join("WipTracker.cmd");
        if enabled {
            std::fs::write(target, format!("@start \"\" \"{}\"\r\n", exe.display()))
        } else {
            match std::fs::remove_file(target) {
                Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
                _ => Ok(()),
            }
        }
    }
    #[cfg(target_os = "macos")]
    {
        let script = if enabled {
            format!(
                "tell application \"System Events\" to make login item at end with properties \
                 {{path:\"{}\", hidden:false, name:\"WipTracker\"}}",
                exe.display()
            )
        } else {
            "tell application \"System Events\" to delete (every login item whose name is \
             \"WipTracker\")"
                .to_owned()
        };
        let status = std::process::Command::new("osascript")
            .args(["-e", &script])
            .status()?;
        if status.success() {
            Ok(())
        } else {
            Err(io::Error::other("osascript refused"))
        }
    }
    #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
    {
        let _ = (enabled, exe);
        Err(io::Error::other("not supported here"))
    }
}

/// The Linux half of [`set_autostart`], against a named config directory: an autostart
/// entry is the launcher entry in `autostart/`. Public so tests and the app can aim it
/// at a scratch directory.
pub fn set_autostart_in(config_home: &Path, enabled: bool, exe: &Path) -> io::Result<()> {
    let target = config_home.join("autostart/wiptracker.desktop");
    if enabled {
        std::fs::create_dir_all(target.parent().expect("autostart has a parent"))?;
        std::fs::write(target, entry_for(exe))
    } else {
        match std::fs::remove_file(target) {
            Err(error) if error.kind() != io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }
}

#[cfg(target_os = "windows")]
fn startup_dir() -> Option<PathBuf> {
    std::env::var_os("APPDATA")
        .map(|appdata| PathBuf::from(appdata).join("Microsoft/Windows/Start Menu/Programs/Startup"))
}

/// Best-effort cache refresh so the entry appears without a re-login where possible.
/// Every tool is optional; a desktop that has none picks the files up on its own.
pub fn refresh_caches(data_home: &Path) {
    let applications = data_home.join("applications");
    let _ = std::process::Command::new("update-desktop-database")
        .arg(&applications)
        .status();
    let _ = std::process::Command::new("kbuildsycoca6").status();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_entry_names_the_binary_absolutely_and_hides_with_it() {
        let entry = entry_for(Path::new("/opt/somewhere/wiptracker"));
        assert!(entry.contains("Exec=/opt/somewhere/wiptracker\n"));
        assert!(
            entry.contains("TryExec=/opt/somewhere/wiptracker\n"),
            "TryExec is what hides the entry once the binary is uninstalled"
        );
        assert!(
            !entry.contains("Exec=wiptracker\n"),
            "no bare name survives"
        );
        assert!(entry.starts_with("[Desktop Entry]"));
        // Everything else the shipped entry says is kept.
        assert!(entry.contains("Icon=wiptracker"));
        assert!(entry.contains("StartupWMClass=wiptracker"));
    }

    #[test]
    fn install_writes_the_entry_and_every_icon_and_remove_takes_them_back() {
        let scratch = tempfile::tempdir().expect("tempdir");
        let data = scratch.path().join("share");
        let config = scratch.path().join("config");

        install_into(&data, Path::new("/opt/wiptracker")).expect("install");
        assert!(data.join("applications/wiptracker.desktop").exists());
        for (size, _) in ICONS {
            assert!(
                data.join(format!("icons/hicolor/{size}x{size}/apps/wiptracker.png"))
                    .exists(),
                "icon {size} missing"
            );
        }

        remove_from(&data, &config).expect("remove");
        assert!(!data.join("applications/wiptracker.desktop").exists());
        for (size, _) in ICONS {
            assert!(
                !data
                    .join(format!("icons/hicolor/{size}x{size}/apps/wiptracker.png"))
                    .exists()
            );
        }
    }

    /// Switching autostart on writes the entry, off removes it, and off twice is fine.
    #[test]
    fn autostart_switches_on_and_off() {
        let scratch = tempfile::tempdir().expect("tempdir");
        let target = scratch.path().join("autostart/wiptracker.desktop");

        set_autostart_in(scratch.path(), true, Path::new("/opt/wiptracker")).expect("on");
        let written = std::fs::read_to_string(&target).expect("entry written");
        assert!(written.contains("Exec=/opt/wiptracker"));

        set_autostart_in(scratch.path(), false, Path::new("/opt/wiptracker")).expect("off");
        assert!(!target.exists());
        set_autostart_in(scratch.path(), false, Path::new("/opt/wiptracker"))
            .expect("off again is fine");
    }

    /// Removing on a machine that never installed is not an error.
    #[test]
    fn remove_is_content_with_nothing_to_remove() {
        let scratch = tempfile::tempdir().expect("tempdir");
        remove_from(&scratch.path().join("a"), &scratch.path().join("b")).expect("remove");
    }
}
