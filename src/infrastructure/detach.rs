//! Handing the terminal back on macOS.
//!
//! Homebrew ships the bare binary, and a binary started from a shell is a foreground
//! child of that shell: the prompt does not come back until the bar is closed, and
//! closing the terminal takes the bar with it. On Linux that is what `&` is for; on
//! macOS, where a graphical program is expected to behave like `open`, the app starts
//! itself again as a session of its own and lets the parent return.
//!
//! Only a start from a terminal detaches. Finder, `open` and launchd hand the process a
//! null stdin, and those starts must stay the process the system launched — that is the
//! one it puts the bundle's `LSUIElement` and login-item bookkeeping on.

use std::io;
use std::path::Path;

/// The flag that keeps the process attached to its terminal, for debugging.
pub const FOREGROUND: &str = "--foreground";

/// Whether this start should hand the terminal back.
pub fn wanted(args: &[String]) -> bool {
    use std::io::IsTerminal as _;
    wanted_on(cfg!(target_os = "macos"), io::stdin().is_terminal(), args)
}

/// [`wanted`] against named inputs, so the rule is testable on every platform: macOS
/// only, only from a terminal, and never when the foreground was asked for.
fn wanted_on(macos: bool, from_terminal: bool, args: &[String]) -> bool {
    macos && from_terminal && !args.iter().any(|arg| arg == FOREGROUND)
}

/// Starts `exe` again with the same arguments plus [`FOREGROUND`], detached from the
/// terminal. The caller exits once this returns.
#[cfg(target_os = "macos")]
#[allow(unsafe_code)]
pub fn detach(exe: &Path, args: &[String]) -> io::Result<()> {
    use std::os::unix::process::CommandExt as _;
    use std::process::{Command, Stdio};

    let mut command = Command::new(exe);
    command
        .args(args)
        .arg(FOREGROUND)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    // SAFETY: `pre_exec` runs between fork and exec, where only async-signal-safe calls
    // are allowed. `setsid` is one, and it touches no memory of the parent: it gives the
    // child a session of its own, which is what takes the controlling terminal away.
    unsafe {
        command.pre_exec(|| {
            if libc::setsid() == -1 {
                return Err(io::Error::last_os_error());
            }
            Ok(())
        });
    }
    command.spawn()?;
    Ok(())
}

#[cfg(not(target_os = "macos"))]
pub fn detach(_exe: &Path, _args: &[String]) -> io::Result<()> {
    Err(io::Error::other("only macOS detaches"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(list: &[&str]) -> Vec<String> {
        list.iter().map(|arg| (*arg).to_owned()).collect()
    }

    #[test]
    fn only_a_terminal_start_on_macos_detaches() {
        assert!(wanted_on(true, true, &[]));
        assert!(
            !wanted_on(false, true, &[]),
            "Linux and Windows never detach"
        );
        assert!(
            !wanted_on(true, false, &[]),
            "Finder and launchd starts stay put"
        );
    }

    #[test]
    fn the_foreground_flag_keeps_the_terminal() {
        assert!(!wanted_on(true, true, &args(&[FOREGROUND])));
        assert!(!wanted_on(
            true,
            true,
            &args(&["--reset-position", FOREGROUND])
        ));
        assert!(wanted_on(true, true, &args(&["--reset-position"])));
    }

    /// Whatever stdin is on this machine, the answer off macOS is no.
    #[cfg(not(target_os = "macos"))]
    #[test]
    fn the_real_check_says_no_here() {
        assert!(!wanted(&[]));
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn detaching_is_refused_elsewhere() {
        assert!(detach(Path::new("/nowhere"), &[]).is_err());
    }
}
