//! Cross-platform helpers for spawning child processes without surfacing a stray
//! console window on Windows.
//!
//! The launcher itself ships as a `windows_subsystem = "windows"` binary, so it has
//! no console of its own. Whenever it spawns a console-subsystem child (Java, mostly)
//! Windows allocates a fresh console window for that child unless `CREATE_NO_WINDOW`
//! is set. We always pipe stdout/stderr, so suppressing the window costs nothing.
//!
//! Usage:
//!
//! ```ignore
//! let mut cmd = std::process::Command::new(java);
//! crate::process::no_window(&mut cmd);
//! cmd.spawn()?;
//! ```

use std::process::Command;

/// `CREATE_NO_WINDOW` from `winbase.h`. Inlined to avoid dragging in `windows-sys`
/// for a single constant.
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// Mark this command so Windows won't allocate a console window for the child. No-op
/// on non-Windows platforms.
#[cfg(windows)]
pub fn no_window(cmd: &mut Command) -> &mut Command {
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(CREATE_NO_WINDOW)
}

#[cfg(not(windows))]
pub fn no_window(cmd: &mut Command) -> &mut Command {
    cmd
}
