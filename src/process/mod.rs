//! Subprocess-execution port — the seam between Installers and the OS.
//!
//! The [`CommandRunner`] trait abstracts "spawn this command with these
//! args in this directory, possibly with env overrides, and check its
//! exit status." Installers call into it instead of constructing
//! `std::process::Command` directly so tests can substitute a recording
//! runner without forking real subprocesses.
//!
//! Two adapters ship in this module:
//!
//! - [`SystemCommandRunner`] (production) — forks a real subprocess
//!   with inherited stdio.
//! - [`RecordingCommandRunner`] (test) — captures each call as a
//!   [`RecordedCall`] and returns configurable results from a queue.

use std::path::Path;
use std::process::ExitStatus;

use thiserror::Error;

pub mod recording;
pub mod system;

pub use recording::{RecordedCall, RecordingCommandRunner};
pub use system::SystemCommandRunner;

/// Spawns one subprocess at a time.
///
/// Contract: `cwd` is the directory the child runs in; `env` entries are
/// added to (or override) the inherited environment per-key (the
/// inherited env is otherwise preserved). Stdio is inherited so the user
/// sees output live — chord's TUI relies on this when it drops to the
/// inline screen for slow installs.
pub trait CommandRunner {
    fn run(
        &self,
        cmd: &str,
        args: &[&str],
        cwd: &Path,
        env: &[(&str, &str)],
    ) -> Result<(), CommandError>;
}

/// Error returned by [`CommandRunner::run`].
#[derive(Debug, Error)]
pub enum CommandError {
    #[error("failed to spawn '{0}': {1}")]
    Spawn(String, std::io::Error),

    #[error("'{0}' exited with status {1}")]
    NonZeroExit(String, ExitStatus),
}
