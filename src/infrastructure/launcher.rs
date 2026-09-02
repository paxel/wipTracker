//! Installing the launcher entry and icons into the user's own XDG directories.
//!
//! A desktop only lists applications whose `.desktop` entry sits in a directory on the
//! session's `XDG_DATA_DIRS` — and a Homebrew prefix never is, so a brew-installed
//! WipTracker is invisible to every menu until its entry is copied into
//! `~/.local/share`. A package manager must not write into `$HOME`, but the app itself,
//! asked by its user, may.
//!
//! The entry names the binary by absolute path in `Exec` and in `TryExec` — by its
//! stable name where there is one: a Homebrew binary lives in a Cellar directory that
//! carries the version and vanishes with every upgrade, while the `bin` symlink to it is
//! repointed, so the symlink is what the entry must say (see [`stable_exe`]).
//!
//! `TryExec` asks the desktop to hide the entry once the binary is gone, but not every
//! menu re-checks it, so it is a courtesy, not the mechanism. What actually keeps the
//! entry alive across upgrades is [`repair_stale_entries`], run at startup: an entry
//! this app once wrote and that no longer names the running binary is rewritten.

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
    data_dirs_with(data_home(), std::env::var("XDG_DATA_DIRS").ok())
}

/// [`data_dirs`] against named inputs, so both the set and the unset shape are exercised
/// on every machine — which branch the environment picks must not move the coverage.
fn data_dirs_with(data_home: Option<PathBuf>, system: Option<String>) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = data_home.into_iter().collect();
    let system = system.unwrap_or_default();
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

/// The path the entries should name for the running binary.
///
/// The binary itself, unless a directory on `$PATH` holds a name that resolves to it —
/// then that name: it is the stable one. A Homebrew install runs from
/// `Cellar/wiptracker/<version>/bin/`, which an upgrade deletes, while the
/// `bin/wiptracker` symlink pointing at it is repointed to the new version.
pub fn stable_exe() -> PathBuf {
    let exe = std::env::current_exe().unwrap_or_default();
    stable_exe_from(&exe, std::env::var_os("PATH"))
}

/// [`stable_exe`] against named inputs, so tests can lay out their own worlds.
fn stable_exe_from(exe: &Path, path: Option<std::ffi::OsString>) -> PathBuf {
    let Ok(real) = exe.canonicalize() else {
        return exe.to_path_buf();
    };
    let Some(name) = exe.file_name() else {
        return exe.to_path_buf();
    };
    let path = path.unwrap_or_default();
    for dir in std::env::split_paths(&path) {
        if dir.as_os_str().is_empty() {
            continue;
        }
        let candidate = dir.join(name);
        if candidate.canonicalize().is_ok_and(|found| found == real) {
            return candidate;
        }
    }
    exe.to_path_buf()
}

/// Rewrites entries this app once wrote when they no longer say what they would be
/// written with today — after an upgrade moved the binary, they would otherwise go on
/// pointing at the removed version. Entries that already match, and machines that never
/// installed any, are left alone. Says whether anything was rewritten, so the caller
/// knows to refresh the menu caches.
pub fn repair_stale_entries(
    data_home: &Path,
    config_home: Option<&Path>,
    exe: &Path,
) -> io::Result<bool> {
    let wanted = entry_for(exe);
    let mut repaired = false;

    let entry = data_home.join("applications/wiptracker.desktop");
    if std::fs::read_to_string(&entry).is_ok_and(|found| found != wanted) {
        install_into(data_home, exe)?;
        repaired = true;
    }
    if let Some(config) = config_home {
        let autostart = config.join("autostart/wiptracker.desktop");
        if std::fs::read_to_string(&autostart).is_ok_and(|found| found != wanted) {
            set_autostart_in(config, true, exe)?;
            repaired = true;
        }
    }
    Ok(repaired)
}

/// The entry as it is written: the shipped one, with the binary named absolutely.
fn entry_for(exe: &Path) -> String {
    let exe = exe.display();
    let mut written = String::new();
    for line in ENTRY.lines() {
        if let Some(rest) = line.strip_prefix("Exec=") {
            let _ = rest;
            written.push_str(&format!("Exec={exe}\n"));
            // Asks menus that honor it to hide the entry once the binary is gone.
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
        Some(launch_agents_dir()?.join(LAUNCH_AGENT).exists())
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
        let dir = launch_agents_dir().ok_or_else(|| io::Error::other("no home directory"))?;
        set_launch_agent_in(&dir, enabled, exe)
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

/// The launchd agent's file name under `~/Library/LaunchAgents`, which is also its label.
pub const LAUNCH_AGENT: &str = "dev.paxel.wiptracker.plist";

/// Where launchd reads a user's own agents from.
#[cfg(target_os = "macos")]
fn launch_agents_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join("Library/LaunchAgents"))
}

/// The launchd agent that starts the binary at login.
///
/// A plain file, like the Linux autostart entry: launchd reads every agent in the
/// directory when the session starts, so writing it is the whole job and deleting it is
/// the whole undo. Nothing is loaded or unloaded on the spot — loading a `RunAtLoad`
/// agent would start a second bar right away, and unloading one would stop the running
/// one. A login item would need `osascript` and an automation prompt, and macOS opens a
/// bare binary named by a login item through Terminal.app; launchd starts it directly.
pub fn launch_agent_for(exe: &Path) -> String {
    let exe = exe.display();
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
         \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n\
         <dict>\n\
         \t<key>Label</key>\n\
         \t<string>dev.paxel.wiptracker</string>\n\
         \t<key>ProgramArguments</key>\n\
         \t<array>\n\
         \t\t<string>{exe}</string>\n\
         \t</array>\n\
         \t<key>RunAtLoad</key>\n\
         \t<true/>\n\
         </dict>\n\
         </plist>\n"
    )
}

/// The macOS half of [`set_autostart`], against a named directory, so tests can aim it
/// at a scratch one on any platform.
pub fn set_launch_agent_in(dir: &Path, enabled: bool, exe: &Path) -> io::Result<()> {
    let target = dir.join(LAUNCH_AGENT);
    if enabled {
        std::fs::create_dir_all(dir)?;
        std::fs::write(target, launch_agent_for(exe))
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

    /// The launchd agent names the binary, runs at login, and comes and goes with the
    /// switch — off twice is fine, like the Linux entry.
    #[test]
    fn the_launch_agent_switches_on_and_off() {
        let scratch = tempfile::tempdir().expect("tempdir");
        let dir = scratch.path().join("LaunchAgents");
        let target = dir.join(LAUNCH_AGENT);

        set_launch_agent_in(&dir, true, Path::new("/opt/homebrew/bin/wiptracker")).expect("on");
        let written = std::fs::read_to_string(&target).expect("agent written");
        // Byte for byte: launchd ignores a plist it cannot parse without a word.
        assert_eq!(
            written,
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
             <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \
             \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
             <plist version=\"1.0\">\n\
             <dict>\n\
             \t<key>Label</key>\n\
             \t<string>dev.paxel.wiptracker</string>\n\
             \t<key>ProgramArguments</key>\n\
             \t<array>\n\
             \t\t<string>/opt/homebrew/bin/wiptracker</string>\n\
             \t</array>\n\
             \t<key>RunAtLoad</key>\n\
             \t<true/>\n\
             </dict>\n\
             </plist>\n"
        );

        set_launch_agent_in(&dir, false, Path::new("/opt/homebrew/bin/wiptracker")).expect("off");
        assert!(!target.exists());
        set_launch_agent_in(&dir, false, Path::new("/opt/homebrew/bin/wiptracker"))
            .expect("off again is fine");
    }

    /// Both shapes of `XDG_DATA_DIRS`, independent of what this machine's session says:
    /// unset falls back to the two system prefixes, set is split with empties dropped.
    #[test]
    fn the_data_dirs_cover_both_environment_shapes() {
        let unset = data_dirs_with(Some(PathBuf::from("/home/u/.local/share")), None);
        assert_eq!(
            unset,
            vec![
                PathBuf::from("/home/u/.local/share"),
                PathBuf::from("/usr/local/share"),
                PathBuf::from("/usr/share"),
            ]
        );

        let set = data_dirs_with(None, Some("/opt/share::/var/share".to_owned()));
        assert_eq!(
            set,
            vec![PathBuf::from("/opt/share"), PathBuf::from("/var/share")],
            "split on colons, empty entries dropped"
        );
    }

    /// The environment readers answer on any machine with a home directory, and the
    /// visibility check is callable whatever the machine looks like.
    #[test]
    fn the_environment_readers_answer() {
        if std::env::var_os("HOME").is_some() {
            assert!(data_home().is_some());
            assert!(config_home().is_some());
        }
        let _ = is_visible();
        let _ = autostart_enabled();
    }

    /// Removing on a machine that never installed is not an error.
    #[test]
    fn remove_is_content_with_nothing_to_remove() {
        let scratch = tempfile::tempdir().expect("tempdir");
        remove_from(&scratch.path().join("a"), &scratch.path().join("b")).expect("remove");
    }

    /// The Homebrew shape: the binary in a versioned Cellar directory, a `bin` symlink
    /// on `$PATH` pointing at it. The entry must name the symlink, which survives the
    /// upgrade that deletes the Cellar directory.
    #[test]
    #[cfg(unix)]
    fn the_stable_name_is_the_path_symlink_not_the_cellar() {
        let scratch = tempfile::tempdir().expect("tempdir");
        let cellar = scratch.path().join("Cellar/wiptracker/0.6.0/bin");
        std::fs::create_dir_all(&cellar).expect("cellar");
        let real = cellar.join("wiptracker");
        std::fs::write(&real, "").expect("binary");
        let bin = scratch.path().join("bin");
        std::fs::create_dir_all(&bin).expect("bin");
        let link = bin.join("wiptracker");
        std::os::unix::fs::symlink(&real, &link).expect("symlink");

        let path = std::env::join_paths([&bin]).expect("join");
        assert_eq!(stable_exe_from(&real, Some(path)), link);

        // Without a matching name on PATH the binary keeps its own path.
        let elsewhere = std::env::join_paths([scratch.path()]).expect("join");
        assert_eq!(stable_exe_from(&real, Some(elsewhere)), real);
        assert_eq!(stable_exe_from(&real, None), real);
    }

    /// After an upgrade the written entries name the removed version; the repair
    /// rewrites both, and an entry that is already right is left untouched.
    #[test]
    fn stale_entries_are_repaired_and_fresh_ones_left_alone() {
        let scratch = tempfile::tempdir().expect("tempdir");
        let data = scratch.path().join("share");
        let config = scratch.path().join("config");
        let old = Path::new("/opt/cellar/0.6.0/wiptracker");
        let new = Path::new("/opt/bin/wiptracker");

        install_into(&data, old).expect("install");
        set_autostart_in(&config, true, old).expect("autostart");

        assert!(repair_stale_entries(&data, Some(&config), new).expect("repair"));
        let entry =
            std::fs::read_to_string(data.join("applications/wiptracker.desktop")).expect("entry");
        assert!(entry.contains("Exec=/opt/bin/wiptracker\n"));
        let autostart = std::fs::read_to_string(config.join("autostart/wiptracker.desktop"))
            .expect("autostart entry");
        assert!(autostart.contains("Exec=/opt/bin/wiptracker\n"));

        assert!(
            !repair_stale_entries(&data, Some(&config), new).expect("repair again"),
            "a second pass finds nothing to do"
        );
    }

    /// A machine that never installed anything has nothing to repair — and, most
    /// importantly, nothing gets written where nothing was.
    #[test]
    fn repair_writes_nothing_where_nothing_was_installed() {
        let scratch = tempfile::tempdir().expect("tempdir");
        let data = scratch.path().join("share");
        let config = scratch.path().join("config");
        assert!(
            !repair_stale_entries(&data, Some(&config), Path::new("/opt/wiptracker"))
                .expect("repair")
        );
        assert!(!data.exists());
        assert!(!config.exists());
    }
}
