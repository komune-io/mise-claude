//! Production [`CommandRunner`] — forks real subprocesses.

use std::path::Path;
use std::process::Command;

use super::{CommandError, CommandRunner};

/// Spawns commands via `std::process::Command` with inherited stdio.
///
/// When `verbose` is `true`, emits `[verbose] <cmd> <args...>` to stderr
/// before each spawn. The TUI's drop-to-inline path and the CLI's
/// `--verbose` flag both flow into this single eprintln site, so chord
/// has one canonical verbose subprocess log format.
pub struct SystemCommandRunner {
    pub verbose: bool,
}

impl SystemCommandRunner {
    pub fn new(verbose: bool) -> Self {
        Self { verbose }
    }
}

impl CommandRunner for SystemCommandRunner {
    fn run(
        &self,
        cmd: &str,
        args: &[&str],
        cwd: &Path,
        env: &[(&str, &str)],
    ) -> Result<(), CommandError> {
        if self.verbose {
            eprintln!("[verbose] {} {}", cmd, args.join(" "));
        }

        let mut command = Command::new(cmd);
        command.args(args).current_dir(cwd);
        for (k, v) in env {
            command.env(k, v);
        }

        let status = command
            .status()
            .map_err(|e| CommandError::Spawn(cmd.to_string(), e))?;
        if !status.success() {
            return Err(CommandError::NonZeroExit(cmd.to_string(), status));
        }
        Ok(())
    }

    fn run_capture(
        &self,
        cmd: &str,
        args: &[&str],
        cwd: &Path,
        env: &[(&str, &str)],
    ) -> Result<String, CommandError> {
        if self.verbose {
            eprintln!("[verbose] {} {}", cmd, args.join(" "));
        }

        let mut command = Command::new(cmd);
        command.args(args).current_dir(cwd);
        for (k, v) in env {
            command.env(k, v);
        }

        let output = command
            .output()
            .map_err(|e| CommandError::Spawn(cmd.to_string(), e))?;
        if !output.status.success() {
            return Err(CommandError::NonZeroExit(cmd.to_string(), output.status));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }
}
