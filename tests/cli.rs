//! Drives the real binary's command-line surface, which nothing in-process can reach.

use std::process::Command;

/// The compiled binary under test.
fn wiptracker() -> Command {
    Command::new(env!("CARGO_BIN_EXE_wiptracker"))
}

#[test]
fn version_names_itself() {
    let output = wiptracker().arg("--version").output().expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains(env!("CARGO_PKG_VERSION")));
    assert!(stdout.contains("wiptracker"));
}

#[test]
fn help_describes_every_flag() {
    let output = wiptracker().arg("--help").output().expect("run");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    for flag in [
        "--version",
        "--help",
        "--reset-position",
        "--foreground",
        "--install-launcher",
        "--remove-launcher",
    ] {
        assert!(stdout.contains(flag), "usage is missing {flag}");
    }
}

#[test]
fn an_unknown_argument_is_refused_with_the_usage() {
    let output = wiptracker().arg("--frobnicate").output().expect("run");
    assert_eq!(output.status.code(), Some(2));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--frobnicate"));
    assert!(stderr.contains("Usage:"));
}

/// The launcher flags, end to end against a scratch home: install writes a valid entry
/// with the binary named absolutely, remove leaves nothing behind.
#[test]
fn the_launcher_flags_install_and_remove() {
    let scratch = tempfile::tempdir().expect("tempdir");
    let home = scratch.path();
    let run = |flag: &str| {
        wiptracker()
            .arg(flag)
            .env("HOME", home)
            .env("XDG_DATA_HOME", home.join("share"))
            .env("XDG_CONFIG_HOME", home.join("config"))
            // The cache refreshers would otherwise touch the real user's caches.
            .env("PATH", "")
            .output()
            .expect("run")
    };

    let output = run("--install-launcher");
    assert!(
        output.status.success(),
        "install failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let entry = home.join("share/applications/wiptracker.desktop");
    let written = std::fs::read_to_string(&entry).expect("the entry was written");
    assert!(written.contains(&format!("Exec={}", env!("CARGO_BIN_EXE_wiptracker"))));
    assert!(written.contains("TryExec="));
    assert!(
        home.join("share/icons/hicolor/256x256/apps/wiptracker.png")
            .exists()
    );

    let output = run("--remove-launcher");
    assert!(output.status.success());
    assert!(!entry.exists());
    assert!(
        !home
            .join("share/icons/hicolor/256x256/apps/wiptracker.png")
            .exists()
    );
}
