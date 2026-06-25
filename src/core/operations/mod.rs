//! Shared operation primitives for the `chord` CLI and TUI.
//!
//! Each submodule exposes one verb (`add`, `remove`, `install`, `scope`).
//! Both `main.rs` and the TUI handler dispatch through this module so the
//! two surfaces never drift in semantics.

use std::path::Path;

use thiserror::Error;

use crate::core::error::InstallError;
use crate::core::installer::InstallerSet;
use crate::core::store::{ConfigStore, ConfigStoreError, LockfileStore, LockfileStoreError};

pub mod add;
pub mod install;
pub mod remove;
pub mod scope;

/// Shared context for all operations. Carries trait-object references to
/// the persistent-state stores, the installer set used by Install (all)
/// and Install (one), plus the path bag the remaining non-store
/// operations (settings.json, .mcp.json, packages dir) still need.
///
/// Construct one per top-level entry point (CLI command arm, TUI runner)
/// from concrete adapters and pass `&OpContext` into each operation. The
/// store trait objects use interior mutability (the in-memory adapter
/// uses `RefCell`; the file adapter doesn't mutate), so callers don't
/// thread `&mut` through.
pub struct OpContext<'a> {
    pub config_store: &'a dyn ConfigStore,
    pub lockfile_store: &'a dyn LockfileStore,
    pub installers: &'a InstallerSet<'a>,
    pub project_root: &'a Path,
    pub home_dir: &'a Path,
    pub packages_dir: &'a Path,
    pub verbose: bool,
}

/// All error variants returned by `operations::*` functions.
#[derive(Debug, Error)]
pub enum OperationError {
    #[error(transparent)]
    Config(#[from] ConfigStoreError),

    #[error(transparent)]
    Lockfile(#[from] LockfileStoreError),

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
