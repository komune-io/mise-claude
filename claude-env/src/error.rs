use thiserror::Error;

/// Errors that can occur when interacting with the npm registry.
#[derive(Error, Debug)]
pub enum NpmError {
    #[error("failed to fetch package '{0}': {1}")]
    Request(String, String),
    #[error("failed to parse metadata for '{0}': {1}")]
    Parse(String, String),
}

/// Errors that can occur when loading or parsing configuration.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// The TOML source could not be parsed or deserialized.
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),

    /// An I/O error occurred while reading the config file.
    #[error("failed to read config file: {0}")]
    Io(#[from] std::io::Error),
}
