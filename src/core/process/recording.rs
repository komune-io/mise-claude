//! Test-only [`CommandRunner`] that records calls and returns scripted
//! results.
//!
//! Two modes:
//!
//! - Default — every call returns `Ok(())` and is recorded for later
//!   assertion via [`RecordingCommandRunner::calls`].
//! - Scripted — pre-seed a queue of `Result<(), CommandError>` via
//!   [`RecordingCommandRunner::with_results`] to inject specific
//!   failures at specific positions in the call sequence.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};

use super::{CommandError, CommandRunner};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecordedCall {
    pub cmd: String,
    pub args: Vec<String>,
    pub cwd: PathBuf,
    pub env: Vec<(String, String)>,
}

pub struct RecordingCommandRunner {
    calls: RefCell<Vec<RecordedCall>>,
    results: RefCell<VecDeque<Result<(), CommandError>>>,
    stdout_results: RefCell<VecDeque<Result<String, CommandError>>>,
}

impl RecordingCommandRunner {
    /// Empty runner — every call returns `Ok(())`.
    pub fn new() -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            results: RefCell::new(VecDeque::new()),
            stdout_results: RefCell::new(VecDeque::new()),
        }
    }

    /// Construct with a queue of pre-determined results. Once the queue
    /// drains, subsequent calls return `Ok(())`.
    pub fn with_results(results: Vec<Result<(), CommandError>>) -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            results: RefCell::new(results.into()),
            stdout_results: RefCell::new(VecDeque::new()),
        }
    }

    /// Construct with a queue of scripted stdout results for `run_capture`.
    /// Once drained, subsequent captures return `Ok(String::new())`.
    pub fn with_stdout(stdout_results: Vec<Result<String, CommandError>>) -> Self {
        Self {
            calls: RefCell::new(Vec::new()),
            results: RefCell::new(VecDeque::new()),
            stdout_results: RefCell::new(stdout_results.into()),
        }
    }

    /// Snapshot of every call this runner has seen, in invocation order.
    pub fn calls(&self) -> Vec<RecordedCall> {
        self.calls.borrow().clone()
    }
}

impl Default for RecordingCommandRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl CommandRunner for RecordingCommandRunner {
    fn run(
        &self,
        cmd: &str,
        args: &[&str],
        cwd: &Path,
        env: &[(&str, &str)],
    ) -> Result<(), CommandError> {
        self.calls.borrow_mut().push(RecordedCall {
            cmd: cmd.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: cwd.to_path_buf(),
            env: env
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        });
        self.results.borrow_mut().pop_front().unwrap_or(Ok(()))
    }

    fn run_capture(
        &self,
        cmd: &str,
        args: &[&str],
        cwd: &Path,
        env: &[(&str, &str)],
    ) -> Result<String, CommandError> {
        self.calls.borrow_mut().push(RecordedCall {
            cmd: cmd.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: cwd.to_path_buf(),
            env: env
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        });
        self.stdout_results
            .borrow_mut()
            .pop_front()
            .unwrap_or_else(|| Ok(String::new()))
    }
}
