//! Storage ports — the seam between Operations and persistent state.
//!
//! Two traits live here: [`ConfigStore`] (manages `chord.toml`) and
//! [`LockfileStore`] (manages `chord.lock`). Operations call into these
//! traits instead of touching the filesystem directly so that tests can
//! substitute in-memory adapters.
//!
//! See `docs/superpowers/specs/2026-05-15-store-ports-design.md` (TODO)
//! for the design rationale.

use thiserror::Error;

use crate::config::Config;
use crate::lockfile::Lockfile;

pub mod file;
pub mod memory;

pub use file::{FileConfigStore, FileLockfileStore};
pub use memory::{InMemoryConfigStore, InMemoryLockfileStore};

/// Opaque snapshot bytes produced by [`ConfigStore::snapshot`] or
/// [`LockfileStore::snapshot`]. Pass to the matching `restore` to return
/// the store to the snapshotted state.
///
/// The byte representation is adapter-specific. The `FileConfigStore`
/// returns the raw file contents (preserving exact formatting); the
/// in-memory adapter returns a re-serialized form (logically equivalent
/// but possibly differently formatted).
pub type Snapshot = Vec<u8>;

/// Manages persistence of `chord.toml`.
///
/// Implementations are free-threaded (`&self`); the in-memory adapter
/// uses interior mutability so callers don't have to thread `&mut`
/// references through Operation signatures.
pub trait ConfigStore {
    /// Load the current `Config`. Returns the default empty config if the
    /// underlying file does not exist (matches today's `Config::from_file`
    /// behavior).
    fn load(&self) -> Result<Config, ConfigStoreError>;

    /// Persist `config`, overwriting any previous content.
    fn save(&self, config: &Config) -> Result<(), ConfigStoreError>;

    /// Capture the current state as an opaque snapshot. Used by
    /// rollback flows (e.g. `operations::remove`) that need to revert
    /// a write if a subsequent step fails.
    fn snapshot(&self) -> Result<Snapshot, ConfigStoreError>;

    /// Restore the store to a previously captured snapshot. After this
    /// call, `load` returns the same `Config` it would have returned
    /// just before the corresponding `snapshot` call.
    fn restore(&self, snap: &[u8]) -> Result<(), ConfigStoreError>;
}

/// Manages persistence of `chord.lock`.
pub trait LockfileStore {
    /// Load the current `Lockfile`. Returns an empty lockfile if the
    /// underlying file does not exist.
    fn load(&self) -> Result<Lockfile, LockfileStoreError>;

    /// Persist `lockfile`, overwriting any previous content.
    fn save(&self, lockfile: &Lockfile) -> Result<(), LockfileStoreError>;

    /// Capture the current state as an opaque snapshot.
    fn snapshot(&self) -> Result<Snapshot, LockfileStoreError>;

    /// Restore the store to a previously captured snapshot.
    fn restore(&self, snap: &[u8]) -> Result<(), LockfileStoreError>;
}

/// Error returned by [`ConfigStore`] operations.
#[derive(Debug, Error)]
pub enum ConfigStoreError {
    #[error("failed to read chord.toml: {0}")]
    Read(std::io::Error),

    #[error("failed to parse chord.toml: {0}")]
    Parse(toml::de::Error),

    #[error("failed to serialize chord.toml: {0}")]
    Serialize(toml::ser::Error),

    #[error("failed to write chord.toml: {0}")]
    Write(std::io::Error),
}

/// Error returned by [`LockfileStore`] operations.
#[derive(Debug, Error)]
pub enum LockfileStoreError {
    #[error("failed to read chord.lock: {0}")]
    Read(std::io::Error),

    #[error("failed to parse chord.lock: {0}")]
    Parse(toml::de::Error),

    #[error("failed to write chord.lock: {0}")]
    Write(std::io::Error),
}
