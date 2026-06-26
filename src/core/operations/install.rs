//! `chord install` core. Used by the CLI and the TUI.

use std::path::PathBuf;

use crate::core::installer::{InstallContext, InstallerSet};
use crate::core::lockfile::{LockedTool, Lockfile};
use crate::core::output::Reporter;
use crate::core::process::SystemCommandRunner;
use crate::core::resolver::{self, Action, PlannedAction, ToolType};

use super::{OpContext, OperationError};

/// Summary of an install run.
#[derive(Debug)]
pub struct InstallOutcome {
    pub installed: u32,
    pub skipped: u32,
    pub failed: u32,
}

impl InstallOutcome {
    pub fn exit_code(&self) -> i32 {
        if self.failed > 0 {
            1
        } else {
            0
        }
    }
}

/// Install every tool declared in chord.toml. Mirrors the previous
/// `main.rs::run_install` behavior.
pub fn install_all(ctx: &OpContext, quiet: bool) -> Result<InstallOutcome, OperationError> {
    let config = ctx.config_store.load()?;
    let mut lockfile = ctx.lockfile_store.load()?;

    let packages_dir = ctx.packages_dir.to_path_buf();
    let is_installed = |section: &str, name: &str| -> bool {
        packages_dir.join(name).join("node_modules").exists()
            && lockfile.get(section, name).is_some()
    };
    let plan = resolver::resolve(&config, &lockfile, &is_installed);

    let mut reporter = if quiet {
        Reporter::new_quiet()
    } else {
        Reporter::new()
    };

    let runner = SystemCommandRunner::new(ctx.verbose);
    let install_ctx = InstallContext {
        project_root: ctx.project_root,
        packages_dir: &packages_dir,
        runner: &runner,
    };

    for action in &plan.actions {
        execute_action(
            action,
            &install_ctx,
            ctx.installers,
            &mut lockfile,
            &mut reporter,
        );
    }

    // Only persist the lockfile when at least one action succeeded:
    // `execute_action` increments `reporter.installed` on the same code path
    // that mutates `lockfile`. Skipping the write when nothing changed keeps
    // the file mtime stable.
    if reporter.installed > 0 {
        ctx.lockfile_store.save(&lockfile)?;
    }

    reporter.summary();

    Ok(InstallOutcome {
        installed: reporter.installed,
        skipped: reporter.skipped,
        failed: reporter.failed,
    })
}

/// Execute a single planned action and update lockfile/reporter accordingly.
fn execute_action(
    action: &PlannedAction,
    ctx: &InstallContext,
    installers: &InstallerSet,
    lockfile: &mut Lockfile,
    reporter: &mut Reporter,
) {
    match &action.action {
        Action::Skip => {
            reporter.skip(&action.name, &action.version);
        }
        Action::Install | Action::Upgrade => {
            let detail = match &action.action {
                Action::Install => "installed",
                Action::Upgrade => "upgraded",
                _ => unreachable!(),
            };

            let result = installers.for_tool(&action.tool_type).install(action, ctx);

            match result {
                Ok(install_result) => {
                    reporter.success(&action.name, &action.version, detail);
                    let section = section_name(&action.tool_type);
                    let locked = if action.tool_type == ToolType::Skill
                        || action.tool_type == ToolType::Plugin
                    {
                        LockedTool {
                            package: None,
                            version: action.version.clone(),
                            integrity: None,
                            resolved_at: Some(chrono::Utc::now().format("%Y-%m-%d").to_string()),
                            skills: None,
                        }
                    } else {
                        LockedTool {
                            package: Some(action.package.clone()),
                            version: action.version.clone(),
                            integrity: install_result.integrity,
                            resolved_at: None,
                            skills: None,
                        }
                    };
                    lockfile.set(section, &action.name, locked);
                }
                Err(e) => {
                    reporter.failure(&action.name, &action.version, &e.to_string());
                }
            }
        }
    }
}

pub(super) fn section_name(tool_type: &ToolType) -> &'static str {
    match tool_type {
        ToolType::Mcp => "mcp",
        ToolType::Cli => "cli",
        ToolType::Skill => "skills",
        ToolType::Plugin => "plugins",
    }
}

/// Install a single tool by name. Looks up the tool in chord.toml, builds
/// a plan, executes only the matching action, and updates the lockfile.
///
/// Returns [`OperationError::NotFound`] if the name is not present in any
/// chord.toml section.
pub fn install_one(
    name: &str,
    ctx: &OpContext,
    quiet: bool,
) -> Result<InstallOutcome, OperationError> {
    let config = ctx.config_store.load()?;

    if !config.mcp.contains_key(name)
        && !config.cli.contains_key(name)
        && !config.skills.contains_key(name)
        && !config.plugins.contains_key(name)
    {
        return Err(OperationError::NotFound(name.to_string()));
    }

    let mut lockfile = ctx.lockfile_store.load()?;

    let packages_dir = ctx.packages_dir.to_path_buf();
    let is_installed = |section: &str, n: &str| -> bool {
        packages_dir.join(n).join("node_modules").exists() && lockfile.get(section, n).is_some()
    };
    let plan = resolver::resolve(&config, &lockfile, &is_installed);

    let mut reporter = if quiet {
        Reporter::new_quiet()
    } else {
        Reporter::new()
    };
    let runner = SystemCommandRunner::new(ctx.verbose);
    let install_ctx = InstallContext {
        project_root: ctx.project_root,
        packages_dir: &packages_dir,
        runner: &runner,
    };

    // Execute only the first plan action matching `name`. A chord.toml can
    // legally declare the same name in two sections, but `install_one` is a
    // per-name surgical operation — we install the first match and stop.
    if let Some(action) = plan.actions.iter().find(|a| a.name == name) {
        execute_action(
            action,
            &install_ctx,
            ctx.installers,
            &mut lockfile,
            &mut reporter,
        );
    }

    // See `install_all` for the invariant rationale behind this guard.
    if reporter.installed > 0 {
        ctx.lockfile_store.save(&lockfile)?;
    }

    reporter.summary();

    Ok(InstallOutcome {
        installed: reporter.installed,
        skipped: reporter.skipped,
        failed: reporter.failed,
    })
}

/// Compute the default packages directory (`$CHORD_HOME` or `~/.chord/packages`).
pub fn default_packages_dir() -> PathBuf {
    std::env::var("CHORD_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".chord")
                .join("packages")
        })
}
