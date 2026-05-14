//! `chord install` core. Used by the CLI and the TUI.

use std::path::PathBuf;

use crate::config::Config;
use crate::installer::cli_tool::CliToolInstaller;
use crate::installer::mcp::McpInstaller;
use crate::installer::plugin::PluginInstaller;
use crate::installer::skill::SkillInstaller;
use crate::installer::{InstallContext, Installer};
use crate::lockfile::{LockedTool, Lockfile};
use crate::output::Reporter;
use crate::resolver::{self, Action, PlannedAction, ToolType};

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
    let config_path = ctx.project_root.join("chord.toml");
    let config = Config::from_file(&config_path).map_err(|e| match e {
        crate::error::ConfigError::Io(io) => OperationError::ConfigRead(io),
        crate::error::ConfigError::Parse(p) => OperationError::ConfigParse(p),
    })?;

    let lock_path = ctx.project_root.join("chord.lock");
    let mut lockfile = Lockfile::from_file(&lock_path).map_err(OperationError::LockfileParse)?;

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

    let install_ctx = InstallContext {
        project_root: ctx.project_root,
        packages_dir: &packages_dir,
        verbose: ctx.verbose,
    };

    for action in &plan.actions {
        execute_action(action, &install_ctx, &mut lockfile, &mut reporter);
    }

    // Only persist the lockfile when at least one action succeeded:
    // `execute_action` increments `reporter.installed` on the same code path
    // that mutates `lockfile`. Skipping the write when nothing changed keeps
    // the file mtime stable.
    if reporter.installed > 0 {
        lockfile
            .write_to_file(&lock_path)
            .map_err(OperationError::LockfileWrite)?;
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
    lockfile: &mut Lockfile,
    reporter: &mut Reporter,
) {
    let mcp_installer = McpInstaller::default();
    let cli_installer = CliToolInstaller::default();
    let skill_installer = SkillInstaller;
    let plugin_installer = PluginInstaller;

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

            let result = match action.tool_type {
                ToolType::Mcp => mcp_installer.install(action, ctx),
                ToolType::Cli => cli_installer.install(action, ctx),
                ToolType::Skill => skill_installer.install(action, ctx),
                ToolType::Plugin => plugin_installer.install(action, ctx),
            };

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
                        }
                    } else {
                        LockedTool {
                            package: Some(action.package.clone()),
                            version: action.version.clone(),
                            integrity: install_result.integrity,
                            resolved_at: None,
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
pub fn install_one(name: &str, ctx: &OpContext) -> Result<InstallOutcome, OperationError> {
    let config_path = ctx.project_root.join("chord.toml");
    let config = Config::from_file(&config_path).map_err(|e| match e {
        crate::error::ConfigError::Io(io) => OperationError::ConfigRead(io),
        crate::error::ConfigError::Parse(p) => OperationError::ConfigParse(p),
    })?;

    if !config.mcp.contains_key(name)
        && !config.cli.contains_key(name)
        && !config.skills.contains_key(name)
        && !config.plugins.contains_key(name)
    {
        return Err(OperationError::NotFound(name.to_string()));
    }

    let lock_path = ctx.project_root.join("chord.lock");
    let mut lockfile = Lockfile::from_file(&lock_path).map_err(OperationError::LockfileParse)?;

    let packages_dir = ctx.packages_dir.to_path_buf();
    let is_installed = |section: &str, n: &str| -> bool {
        packages_dir.join(n).join("node_modules").exists()
            && lockfile.get(section, n).is_some()
    };
    let plan = resolver::resolve(&config, &lockfile, &is_installed);

    let mut reporter = Reporter::new();
    let install_ctx = InstallContext {
        project_root: ctx.project_root,
        packages_dir: &packages_dir,
        verbose: ctx.verbose,
    };

    for action in plan.actions.iter().filter(|a| a.name == name) {
        execute_action(action, &install_ctx, &mut lockfile, &mut reporter);
    }

    // See `install_all` for the invariant rationale behind this guard.
    if reporter.installed > 0 {
        lockfile
            .write_to_file(&lock_path)
            .map_err(OperationError::LockfileWrite)?;
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
