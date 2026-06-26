//! `chord remove` core. Used by the CLI and the TUI.

use crate::core::mcp_config;

use super::{OpContext, OperationError};

/// Outcome of a remove operation.
#[derive(Debug)]
pub struct RemoveOutcome {
    pub section: &'static str,
}

/// Remove a tool from chord.toml + lockfile + .mcp.json + filesystem.
///
/// Returns [`OperationError::NotFound`] if the tool is not in any section.
/// On `.mcp.json` write failure, chord.toml is restored from an opaque
/// snapshot held by the [`ConfigStore`](crate::core::store::ConfigStore) so the
/// user is not left in a half-state.
pub fn remove(name: &str, ctx: &OpContext) -> Result<RemoveOutcome, OperationError> {
    // Capture the pre-mutation bytes before loading the parsed Config.
    // The file adapter preserves byte-identity through restore; the
    // in-memory adapter restores logical equivalence.
    let snapshot = ctx.config_store.snapshot()?;
    let mut config = ctx.config_store.load()?;

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

    ctx.config_store.save(&config)?;

    // 2. .mcp.json (rollback-on-failure)
    if section == "mcp" {
        if let Err(e) = mcp_config::remove_server(ctx.project_root, name) {
            // Restore chord.toml before reporting the failure.
            if let Err(rb_err) = ctx.config_store.restore(&snapshot) {
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

    // 3b. Skills: remove the .chord store(s) + .claude/skills symlinks.
    if section == "skills" {
        let lf = ctx.lockfile_store.load().unwrap_or_default();
        if let Some(entry) = lf.get("skills", name) {
            let owner_repo = if entry.skills.is_some() {
                name.to_string() // wildcard key is owner/repo
            } else {
                name.rsplit_once('/')
                    .map(|(owner_repo, _)| owner_repo)
                    .unwrap_or(name)
                    .to_string()
            };
            let store_root = {
                let mut p = ctx.project_root.join(".chord");
                for seg in owner_repo.split('/') {
                    p = p.join(seg);
                }
                p
            };
            let flat_names: Vec<String> = match &entry.skills {
                Some(subs) => subs.iter().map(|s| s.name.clone()).collect(),
                None => vec![name.rsplit('/').next().unwrap_or(name).to_string()],
            };
            for flat in &flat_names {
                let link = ctx.project_root.join(".claude").join("skills").join(flat);
                let _ = std::fs::remove_file(&link);
                let _ = std::fs::remove_dir_all(store_root.join(flat));
            }
        }
    }

    // 4. Lockfile (best-effort write).
    let mut lockfile = ctx.lockfile_store.load().unwrap_or_default();
    if section == "skills" {
        lockfile.remove_prefix("skills", name);
    } else {
        lockfile.remove(section, name);
    }
    if let Err(e) = ctx.lockfile_store.save(&lockfile) {
        eprintln!("warning: failed to write lockfile: {e}");
    }

    Ok(RemoveOutcome { section })
}
