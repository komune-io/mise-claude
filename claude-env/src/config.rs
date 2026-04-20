use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;

use crate::error::ConfigError;

/// Top-level `claude-env.toml` configuration.
///
/// Each section is a flat `BTreeMap<String, String>` so that individual tool
/// entries can carry arbitrary version / options strings without a fixed schema.
/// Unknown top-level keys are rejected via `#[serde(deny_unknown_fields)]`.
#[derive(Debug, Default, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// MCP server declarations (`[mcp]` table).
    #[serde(default)]
    pub mcp: BTreeMap<String, String>,

    /// Skill declarations (`[skills]` table).
    #[serde(default)]
    pub skills: BTreeMap<String, String>,

    /// Plugin declarations (`[plugins]` table).
    #[serde(default)]
    pub plugins: BTreeMap<String, String>,

    /// CLI tool declarations (`[cli]` table).
    #[serde(default)]
    pub cli: BTreeMap<String, String>,
}

impl Config {
    /// Load and parse a `claude-env.toml` file from `path`.
    ///
    /// Returns a `ConfigError::Io` if the file cannot be read, or a
    /// `ConfigError::Parse` if the TOML is malformed or contains unknown fields.
    pub fn from_file(path: &Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)?;
        Self::parse(&content)
    }

    /// Parse configuration from a TOML string.
    pub fn parse(content: &str) -> Result<Self, ConfigError> {
        let config: Config = toml::from_str(content)?;
        Ok(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_config_parses_to_defaults() {
        let cfg = Config::parse("").unwrap();
        assert_eq!(cfg, Config::default());
    }

    #[test]
    fn minimal_config_only_mcp() {
        let toml = r#"
            [mcp]
            context7 = "latest"
        "#;
        let cfg = Config::parse(toml).unwrap();
        assert_eq!(cfg.mcp.get("context7").map(String::as_str), Some("latest"));
        assert!(cfg.skills.is_empty());
        assert!(cfg.plugins.is_empty());
        assert!(cfg.cli.is_empty());
    }

    #[test]
    fn full_config_all_sections() {
        let toml = r#"
            [mcp]
            context7 = "latest"
            memory = "^1.0"

            [skills]
            my-skill = "latest"

            [plugins]
            my-plugin = "^2.3"

            [cli]
            some-tool = "1.2.3"
        "#;
        let cfg = Config::parse(toml).unwrap();
        assert_eq!(cfg.mcp.len(), 2);
        assert_eq!(cfg.skills.len(), 1);
        assert_eq!(cfg.plugins.len(), 1);
        assert_eq!(cfg.cli.len(), 1);
        assert_eq!(cfg.mcp.get("context7").map(String::as_str), Some("latest"));
        assert_eq!(cfg.mcp.get("memory").map(String::as_str), Some("^1.0"));
        assert_eq!(cfg.skills.get("my-skill").map(String::as_str), Some("latest"));
        assert_eq!(cfg.plugins.get("my-plugin").map(String::as_str), Some("^2.3"));
        assert_eq!(cfg.cli.get("some-tool").map(String::as_str), Some("1.2.3"));
    }

    #[test]
    fn invalid_toml_returns_parse_error() {
        let bad_toml = "this is [not valid toml {{";
        let err = Config::parse(bad_toml).unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)));
    }

    #[test]
    fn unknown_section_returns_parse_error() {
        let toml = r#"
            [unknown_section]
            foo = "bar"
        "#;
        let err = Config::parse(toml).unwrap_err();
        assert!(matches!(err, ConfigError::Parse(_)));
    }
}
