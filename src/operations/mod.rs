//! Shared operation primitives for the `chord` CLI and TUI.
//!
//! Each submodule exposes one verb (`add`, `remove`, `install`, `scope`).
//! Both `main.rs` and the TUI handler dispatch through this module so the
//! two surfaces never drift in semantics.

use std::path::Path;

use thiserror::Error;

use crate::error::InstallError;

pub mod add;
pub mod install;
pub mod remove;
pub mod scope;

/// Shared context for all operations. Replaces the ad-hoc `&Path` arguments
/// previously threaded through `main.rs`.
pub struct OpContext<'a> {
    pub project_root: &'a Path,
    pub home_dir: &'a Path,
    pub packages_dir: &'a Path,
    pub verbose: bool,
}

/// All error variants returned by `operations::*` functions.
#[derive(Debug, Error)]
pub enum OperationError {
    #[error("failed to read chord.toml: {0}")]
    ConfigRead(std::io::Error),

    #[error("failed to parse chord.toml: {0}")]
    ConfigParse(#[from] toml::de::Error),

    #[error("failed to write chord.toml: {0}")]
    ConfigWrite(std::io::Error),

    #[error("failed to write chord.lock: {0}")]
    LockfileWrite(std::io::Error),

    #[error("tool '{0}' not found in chord.toml")]
    NotFound(String),

    #[error("tool '{0}' is already declared in chord.toml")]
    Duplicate(String),

    #[error("invalid spec: {0}")]
    Parse(String),

    #[error("install failed: {0}")]
    Install(#[from] InstallError),

    #[error("failed to update settings.json: {0}")]
    Settings(std::io::Error),

    #[error("failed to update .mcp.json: {0}")]
    McpConfig(std::io::Error),
}
