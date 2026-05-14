//! `chord add` core. Parses `<section>:<name>@<version>` and writes the
//! entry to chord.toml before delegating to `install_one`.

use crate::config::Config;

use super::{OpContext, OperationError};

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Section {
    Mcp,
    Cli,
    Skills,
    Plugins,
}

impl Section {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "mcp" => Some(Section::Mcp),
            "cli" => Some(Section::Cli),
            "skills" => Some(Section::Skills),
            "plugins" => Some(Section::Plugins),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Section::Mcp => "mcp",
            Section::Cli => "cli",
            Section::Skills => "skills",
            Section::Plugins => "plugins",
        }
    }
}

/// Parsed `<section>:<name>@<version>` triple.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct AddSpec {
    pub section: Section,
    pub name: String,
    pub version: String,
}

impl AddSpec {
    /// Parse a tool spec.
    ///
    /// Rules:
    /// 1. Split on the FIRST `:` → `(section, rest)`. Section must be
    ///    `mcp` | `cli` | `skills` | `plugins`.
    /// 2. Section-aware `@` split for the version suffix:
    ///    - Plugin names contain `@marketplace` by convention, so for
    ///      `Section::Plugins` a version is recognized only when `rest`
    ///      has 2+ `@` characters (split on the last one).
    ///    - For every other section the first `@` is the version separator.
    ///    - When no version separator is present, `version` defaults to
    ///      `"latest"` and `name = rest` (preserving any inner `@`).
    /// 3. Reject empty `name`. Reject empty `version` if a version
    ///    separator was found but the suffix is empty.
    pub fn parse(input: &str) -> Result<Self, OperationError> {
        let (section_str, rest) = input
            .split_once(':')
            .ok_or_else(|| OperationError::Parse(format!("missing ':' in spec: '{input}'")))?;

        let section = Section::parse(section_str)
            .ok_or_else(|| OperationError::Parse(format!("unknown section: '{section_str}'")))?;

        // For the `plugins` section the name already contains an `@`
        // (the marketplace qualifier, e.g. `owner/repo/plugin@marketplace`).
        // A version suffix is therefore only present when there are at least
        // two `@` characters in `rest` — the last one being the version.
        // For every other section the first (and only expected) `@` is the
        // version separator.
        let at_count = rest.chars().filter(|&c| c == '@').count();
        let split_on_last_at = match section {
            Section::Plugins => at_count >= 2,
            _ => at_count >= 1,
        };

        let (name, version) = if split_on_last_at {
            let idx = rest.rfind('@').unwrap();
            let name = &rest[..idx];
            let version = &rest[idx + 1..];
            if version.is_empty() {
                return Err(OperationError::Parse(format!(
                    "empty version after '@' in spec: '{input}'"
                )));
            }
            (name.to_string(), version.to_string())
        } else {
            (rest.to_string(), "latest".to_string())
        };

        if name.is_empty() {
            return Err(OperationError::Parse(format!(
                "empty name in spec: '{input}'"
            )));
        }

        Ok(AddSpec {
            section,
            name,
            version,
        })
    }
}

/// Insert the spec into chord.toml. Returns [`OperationError::Duplicate`]
/// if `spec.name` already appears in any section.
///
/// Exposed for unit testing in isolation from the installer call chain.
pub fn write_toml_entry(spec: &AddSpec, ctx: &OpContext) -> Result<(), OperationError> {
    let config_path = ctx.project_root.join("chord.toml");

    let original_toml = std::fs::read_to_string(&config_path).or_else(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            Ok(String::new())
        } else {
            Err(OperationError::ConfigRead(e))
        }
    })?;

    let mut config: Config = toml::from_str(&original_toml).map_err(OperationError::ConfigParse)?;

    let already_present = config.mcp.contains_key(&spec.name)
        || config.cli.contains_key(&spec.name)
        || config.skills.contains_key(&spec.name)
        || config.plugins.contains_key(&spec.name);

    if already_present {
        return Err(OperationError::Duplicate(spec.name.clone()));
    }

    let target = match spec.section {
        Section::Mcp => &mut config.mcp,
        Section::Cli => &mut config.cli,
        Section::Skills => &mut config.skills,
        Section::Plugins => &mut config.plugins,
    };
    target.insert(spec.name.clone(), spec.version.clone());

    let new_toml = toml::to_string_pretty(&config)
        .map_err(|e| OperationError::ConfigWrite(std::io::Error::other(e)))?;
    std::fs::write(&config_path, new_toml).map_err(OperationError::ConfigWrite)?;
    Ok(())
}

/// Write the entry, then install the single tool.
pub fn add(
    spec: &AddSpec,
    ctx: &OpContext,
) -> Result<super::install::InstallOutcome, OperationError> {
    write_toml_entry(spec, ctx)?;
    super::install::install_one(&spec.name, ctx, false)
}
