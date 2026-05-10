use std::path::Path;

use crate::config::Config;

/// Parse `.mise.toml` `claude:*` tool entries and produce a `Config`
/// suitable for writing as `chord.toml`.
///
/// Returns an error if `.mise.toml` is missing, unparseable, or contains
/// no `claude:` entries.
pub fn migrate(project_dir: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let path = project_dir.join(".mise.toml");
    let content = std::fs::read_to_string(&path)
        .map_err(|e| format!("failed to read .mise.toml: {e}"))?;

    let raw: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("failed to parse .mise.toml: {e}"))?;

    let tools = raw
        .get("tools")
        .and_then(|t| t.as_table())
        .ok_or("no [tools] section in .mise.toml")?;

    let mut config = Config::default();

    for (key, value) in tools {
        let version = match value {
            toml::Value::String(s) => s.clone(),
            _ => continue,
        };

        if let Some(rest) = key.strip_prefix("claude:mcp/") {
            config.mcp.insert(rest.to_string(), version);
        } else if let Some(rest) = key.strip_prefix("claude:skills.sh/") {
            config.skills.insert(rest.to_string(), version);
        } else if let Some(rest) = key.strip_prefix("claude:plugin/") {
            config.plugins.insert(rest.to_string(), version);
        } else if let Some(rest) = key.strip_prefix("claude:spec/") {
            config.cli.insert(rest.to_string(), version);
        } else if let Some(rest) = key.strip_prefix("claude:cli/") {
            config.cli.insert(rest.to_string(), version);
        }
    }

    let total = config.mcp.len() + config.skills.len() + config.plugins.len() + config.cli.len();
    if total == 0 {
        return Err("no claude: tool entries found in .mise.toml [tools]".into());
    }

    Ok(config)
}

/// Serialize `config` and write it to `<project_dir>/chord.toml`.
pub fn write_chord_toml(
    config: &Config,
    project_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = project_dir.join("chord.toml");
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}
