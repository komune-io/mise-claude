//! Production adapters for [`ConfigStore`] / [`LockfileStore`] — read
//! and write the canonical files at `<project_root>/chord.toml` and
//! `<project_root>/chord.lock`.

use std::path::{Path, PathBuf};

use crate::core::config::Config;
use crate::core::error::ConfigError;
use crate::core::lockfile::Lockfile;

use super::{ConfigStore, ConfigStoreError, LockfileStore, LockfileStoreError, Snapshot};

/// Reads and writes `chord.toml` under a project root.
pub struct FileConfigStore {
    config_path: PathBuf,
}

impl FileConfigStore {
    /// Construct a store that reads/writes `<project_root>/chord.toml`.
    pub fn new(project_root: &Path) -> Self {
        Self {
            config_path: project_root.join("chord.toml"),
        }
    }
}

impl ConfigStore for FileConfigStore {
    fn load(&self) -> Result<Config, ConfigStoreError> {
        match Config::from_file(&self.config_path) {
            Ok(config) => Ok(config),
            Err(ConfigError::Io(e)) if e.kind() == std::io::ErrorKind::NotFound => {
                // Treat a missing chord.toml as an empty config. Mirrors the
                // semantics of `Lockfile::from_file` and lets `chord install`
                // run from a clean checkout without first creating the file.
                Ok(Config::default())
            }
            Err(ConfigError::Io(e)) => Err(ConfigStoreError::Read(e)),
            Err(ConfigError::Parse(e)) => Err(ConfigStoreError::Parse(e)),
        }
    }

    fn save(&self, config: &Config) -> Result<(), ConfigStoreError> {
        let body = toml::to_string_pretty(config).map_err(ConfigStoreError::Serialize)?;
        std::fs::write(&self.config_path, body).map_err(ConfigStoreError::Write)
    }

    fn snapshot(&self) -> Result<Snapshot, ConfigStoreError> {
        match std::fs::read(&self.config_path) {
            Ok(bytes) => Ok(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(ConfigStoreError::Read(e)),
        }
    }

    fn restore(&self, snap: &[u8]) -> Result<(), ConfigStoreError> {
        std::fs::write(&self.config_path, snap).map_err(ConfigStoreError::Write)
    }
}

/// Reads and writes `chord.lock` under a project root.
pub struct FileLockfileStore {
    lock_path: PathBuf,
}

impl FileLockfileStore {
    /// Construct a store that reads/writes `<project_root>/chord.lock`.
    pub fn new(project_root: &Path) -> Self {
        Self {
            lock_path: project_root.join("chord.lock"),
        }
    }
}

impl LockfileStore for FileLockfileStore {
    fn load(&self) -> Result<Lockfile, LockfileStoreError> {
        // `Lockfile::from_file` already handles NotFound by returning the
        // default empty lockfile. It collapses other I/O errors into
        // `toml::de::Error::custom(...)`, which is why we can't distinguish
        // a Read failure from a Parse failure here — the underlying API
        // doesn't preserve that distinction.
        Lockfile::from_file(&self.lock_path).map_err(LockfileStoreError::Parse)
    }

    fn save(&self, lockfile: &Lockfile) -> Result<(), LockfileStoreError> {
        lockfile
            .write_to_file(&self.lock_path)
            .map_err(LockfileStoreError::Write)
    }

    fn snapshot(&self) -> Result<Snapshot, LockfileStoreError> {
        match std::fs::read(&self.lock_path) {
            Ok(bytes) => Ok(bytes),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
            Err(e) => Err(LockfileStoreError::Read(e)),
        }
    }

    fn restore(&self, snap: &[u8]) -> Result<(), LockfileStoreError> {
        std::fs::write(&self.lock_path, snap).map_err(LockfileStoreError::Write)
    }
}
