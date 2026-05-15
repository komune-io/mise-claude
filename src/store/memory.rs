//! Test-only in-memory adapters for [`ConfigStore`] / [`LockfileStore`].
//!
//! Useful for Operation tests that don't need a real filesystem. The
//! `RefCell` interior mutation lets the trait methods stay `&self` so
//! the call-site ergonomics match the production adapter.

use std::cell::RefCell;

use crate::config::Config;
use crate::lockfile::Lockfile;

use super::{ConfigStore, ConfigStoreError, LockfileStore, LockfileStoreError, Snapshot};

/// In-memory [`ConfigStore`] backed by a single `RefCell<Config>`.
///
/// `snapshot()` serializes the current `Config` to TOML bytes; `restore()`
/// deserializes those bytes back. Byte-identity is not preserved across
/// snapshot+restore (re-serialization may pick different formatting), but
/// the round-trip is `Config`-equivalent.
pub struct InMemoryConfigStore {
    state: RefCell<Config>,
}

impl InMemoryConfigStore {
    /// Construct a store pre-populated with `config`.
    pub fn new(config: Config) -> Self {
        Self {
            state: RefCell::new(config),
        }
    }

    /// Convenience: empty store.
    pub fn empty() -> Self {
        Self::new(Config::default())
    }
}

impl ConfigStore for InMemoryConfigStore {
    fn load(&self) -> Result<Config, ConfigStoreError> {
        Ok(self.state.borrow().clone())
    }

    fn save(&self, config: &Config) -> Result<(), ConfigStoreError> {
        *self.state.borrow_mut() = config.clone();
        Ok(())
    }

    fn snapshot(&self) -> Result<Snapshot, ConfigStoreError> {
        let body =
            toml::to_string_pretty(&*self.state.borrow()).map_err(ConfigStoreError::Serialize)?;
        Ok(body.into_bytes())
    }

    fn restore(&self, snap: &[u8]) -> Result<(), ConfigStoreError> {
        let text = std::str::from_utf8(snap).map_err(|e| {
            ConfigStoreError::Read(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;
        let config: Config = toml::from_str(text).map_err(ConfigStoreError::Parse)?;
        *self.state.borrow_mut() = config;
        Ok(())
    }
}

/// In-memory [`LockfileStore`] backed by a single `RefCell<Lockfile>`.
pub struct InMemoryLockfileStore {
    state: RefCell<Lockfile>,
}

impl InMemoryLockfileStore {
    pub fn new(lockfile: Lockfile) -> Self {
        Self {
            state: RefCell::new(lockfile),
        }
    }

    pub fn empty() -> Self {
        Self::new(Lockfile::new())
    }
}

impl LockfileStore for InMemoryLockfileStore {
    fn load(&self) -> Result<Lockfile, LockfileStoreError> {
        Ok(self.state.borrow().clone())
    }

    fn save(&self, lockfile: &Lockfile) -> Result<(), LockfileStoreError> {
        *self.state.borrow_mut() = lockfile.clone();
        Ok(())
    }

    fn snapshot(&self) -> Result<Snapshot, LockfileStoreError> {
        Ok(self.state.borrow().serialize().into_bytes())
    }

    fn restore(&self, snap: &[u8]) -> Result<(), LockfileStoreError> {
        let text = std::str::from_utf8(snap).map_err(|e| {
            LockfileStoreError::Read(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?;
        let lockfile = Lockfile::parse(text).map_err(LockfileStoreError::Parse)?;
        *self.state.borrow_mut() = lockfile;
        Ok(())
    }
}
