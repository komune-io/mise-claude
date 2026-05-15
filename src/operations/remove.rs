//! `chord remove` core. Used by the CLI and the TUI.

use crate::config::Config;
use crate::lockfile::Lockfile;
use crate::mcp_config;

use super::{OpContext, OperationError};

/// Outcome of a remove operation.
#[derive(Debug)]
pub struct RemoveOutcome {
    pub section: &'static str,
}

/// Remove a tool from chord.toml + lockfile + .mcp.json + filesystem.
///
/// Returns [`OperationError::NotFound`] if the tool is not in any section.
/// On `.mcp.json` write failure, chord.toml is restored from an in-memory
/// backup so the user is not left in a half-state.
pub fn remove(name: &str, ctx: &OpContext) -> Result<RemoveOutcome, OperationError> {
    let config_path = ctx.project_root.join("chord.toml");

    let original_toml =
        std::fs::read_to_string(&config_path).map_err(OperationError::ConfigRead)?;
    let mut config: Config = toml::from_str(&original_toml)?;

    let section: &'static str = if config.mcp.contains_key(name) {
        "mcp"
    } else if config.cli.contains_key(name) {
        "cli"
    } else if config.skills.contains_key(name) {
        "skills"
    } else if config.plugins.contains_key(name) {
        "plugins"
    } else {
        return Err(OperationError::NotFound(name.to_string()));
    };

    match section {
        "mcp" => {
            config.mcp.remove(name);
        }
        "cli" => {
            config.cli.remove(name);
        }
        "skills" => {
            config.skills.remove(name);
        }
        "plugins" => {
            config.plugins.remove(name);
        }
        _ => unreachable!(),
    }

    let new_toml = toml::to_string_pretty(&config)
        .map_err(|e| OperationError::ConfigWrite(std::io::Error::other(e)))?;
    std::fs::write(&config_path, &new_toml).map_err(OperationError::ConfigWrite)?;

    // 2. .mcp.json (rollback-on-failure)
    if section == "mcp" {
        if let Err(e) = mcp_config::remove_server(ctx.project_root, name) {
            // Restore chord.toml before reporting the failure.
            if let Err(rb_err) = std::fs::write(&config_path, &original_toml) {
                eprintln!("warning: rollback of chord.toml also failed: {rb_err}");
                eprintln!("         chord.toml may be missing '{name}' — manual restore required");
            }
            return Err(OperationError::McpConfig(e));
        }
    }

    // 3. Package directory (best-effort).
    let pkg_dir = ctx.packages_dir.join(name);
    match std::fs::remove_dir_all(&pkg_dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => eprintln!("warning: failed to remove package directory: {e}"),
    }

    // 4. Lockfile (best-effort write).
    let lock_path = ctx.project_root.join("chord.lock");
    let mut lockfile = Lockfile::from_file(&lock_path).unwrap_or_default();
    lockfile.remove(section, name);
    if let Err(e) = lockfile.write_to_file(&lock_path) {
        eprintln!("warning: failed to write lockfile: {e}");
    }

    Ok(RemoveOutcome { section })
}
