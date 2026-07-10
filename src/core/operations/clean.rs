//! `chord clean` core — reverse everything chord installed.
//!
//! Default mode removes only chord-owned artifacts (driven by chord.lock) and
//! is reversible via `chord install`. `--all` additionally wipes foreign
//! artifacts and all project Claude config (destructive). `chord.toml` is never
//! touched.

use std::path::Path;

use crate::core::mcp_config;
use crate::core::process::{CommandRunner, SystemCommandRunner};

use super::{OpContext, OperationError};

/// Tally of what `clean` removed.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct CleanOutcome {
    pub skills: u32,
    pub mcp: u32,
    pub cli: u32,
    pub plugins: u32,
    pub plugins_failed: u32,
    /// Foreign artifacts wiped under `--all`. 0 in default mode.
    pub extra_removed: u32,
}

/// The argv (after the `claude` program name) to uninstall the plugin named by
/// a chord.lock plugin key `<owner>/<repo>/<plugin>@<marketplace>`.
pub fn plugin_uninstall_argv(key: &str) -> Vec<String> {
    let plugin_at_market = key.rsplit('/').next().unwrap_or(key);
    vec![
        "plugin".into(),
        "uninstall".into(),
        plugin_at_market.into(),
        "--scope".into(),
        "project".into(),
    ]
}

pub fn clean(ctx: &OpContext, all: bool) -> Result<CleanOutcome, OperationError> {
    let lockfile = ctx.lockfile_store.load()?;
    let mut out = CleanOutcome::default();
    let root = ctx.project_root;

    // Skills: remove chord-owned symlinks (those resolving into .chord). The
    // .chord stores themselves are removed wholesale below.
    for (key, tool) in lockfile.section_entries("skills") {
        // `owner/repo` store namespace: the wildcard key verbatim, or the
        // named key with its leaf skill segment stripped.
        let owner_repo: &str = match &tool.skills {
            Some(_) => &key,
            None => key.rsplit_once('/').map(|(o, _)| o).unwrap_or(&key),
        };
        let flat_names: Vec<String> = match &tool.skills {
            Some(subs) => subs.iter().map(|s| s.name.clone()).collect(),
            None => vec![key.rsplit('/').next().unwrap_or(&key).to_string()],
        };
        for name in &flat_names {
            let link_name = crate::core::skills::materialize::link_name(owner_repo, name);
            let link = root.join(".claude").join("skills").join(link_name);
            if crate::core::skills::materialize::symlink_points_into_chord(root, &link) {
                let _ = std::fs::remove_file(&link);
            }
        }
        out.skills += 1;
    }

    // MCP: drop the server entry from .mcp.json and the installed package dir.
    for (name, _) in lockfile.section_entries("mcp") {
        if let Err(e) = mcp_config::remove_server(root, &name) {
            eprintln!("warning: failed to remove mcp server '{name}': {e}");
        }
        remove_dir_best_effort(&ctx.packages_dir.join(&name));
        out.mcp += 1;
    }

    // CLI: drop the installed package dir.
    for (name, _) in lockfile.section_entries("cli") {
        remove_dir_best_effort(&ctx.packages_dir.join(&name));
        out.cli += 1;
    }

    // Plugins: uninstall via the claude CLI.
    let runner = SystemCommandRunner::new(ctx.verbose);
    for (key, _) in lockfile.section_entries("plugins") {
        let argv = plugin_uninstall_argv(&key);
        let args: Vec<&str> = argv.iter().map(String::as_str).collect();
        match runner.run("claude", &args, root, &[]) {
            Ok(()) => out.plugins += 1,
            Err(e) => {
                eprintln!("warning: failed to uninstall plugin from '{key}': {e}");
                out.plugins_failed += 1;
            }
        }
    }

    // chord-owned store + lockfile.
    remove_dir_best_effort(&root.join(".chord"));
    remove_file_best_effort(&root.join("chord.lock"));

    if all {
        // Full project reset: foreign artifacts + all project Claude config.
        if remove_path(&root.join(".claude").join("skills")) {
            out.extra_removed += 1;
        }
        if remove_path(&root.join(".agents")) {
            out.extra_removed += 1;
        }
        if remove_path(&root.join("skills-lock.json")) {
            out.extra_removed += 1;
        }
        if remove_path(&root.join(".mcp.json")) {
            out.extra_removed += 1;
        }
        if remove_path(&root.join(".claude").join("settings.json")) {
            out.extra_removed += 1;
        }
    }

    Ok(out)
}

/// Remove a path (dir or file/symlink). Returns whether it existed beforehand.
fn remove_path(p: &Path) -> bool {
    let existed = p.symlink_metadata().is_ok();
    if p.is_dir() {
        let _ = std::fs::remove_dir_all(p);
    } else {
        let _ = std::fs::remove_file(p);
    }
    existed
}

fn remove_dir_best_effort(p: &Path) {
    match std::fs::remove_dir_all(p) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => eprintln!("warning: failed to remove {}: {e}", p.display()),
    }
}

fn remove_file_best_effort(p: &Path) {
    match std::fs::remove_file(p) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Err(e) => eprintln!("warning: failed to remove {}: {e}", p.display()),
    }
}
