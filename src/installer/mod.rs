pub mod cli_tool;
pub mod mcp;
pub mod plugin;
pub mod skill;

use crate::error::InstallError;
use crate::process::CommandRunner;
use crate::registry::Registry;
use crate::resolver::{PlannedAction, ToolType};
use std::path::{Path, PathBuf};

pub struct InstallContext<'a> {
    pub project_root: &'a Path,
    pub packages_dir: &'a Path,
    pub runner: &'a dyn CommandRunner,
}

pub trait Installer {
    fn install(
        &self,
        action: &PlannedAction,
        ctx: &InstallContext,
    ) -> Result<InstallResult, InstallError>;
}

#[derive(Debug)]
pub struct InstallResult {
    pub integrity: Option<String>,
}

/// A complete set of Installers, one per `ToolType`. Pure dispatch
/// abstraction: `operations::install::execute_action` calls
/// `installers.for_tool(&action.tool_type).install(...)` instead of
/// constructing concrete adapters inline.
///
/// Construct from concrete adapter references — typically via
/// [`DefaultInstallers::as_set`] in production code, or by building the
/// set literal with mock impls in tests.
pub struct InstallerSet<'a> {
    pub mcp: &'a dyn Installer,
    pub cli: &'a dyn Installer,
    pub skill: &'a dyn Installer,
    pub plugin: &'a dyn Installer,
}

impl<'a> InstallerSet<'a> {
    /// Pick the Installer for a given Tool kind. The match is
    /// exhaustive so a new `ToolType` variant is a compile error here.
    pub fn for_tool(&self, tool_type: &ToolType) -> &dyn Installer {
        match tool_type {
            ToolType::Mcp => self.mcp,
            ToolType::Cli => self.cli,
            ToolType::Skill => self.skill,
            ToolType::Plugin => self.plugin,
        }
    }
}

/// Owning bundle of the four production Installer adapters. Construct
/// once per CLI/TUI entry point and borrow as an [`InstallerSet`] via
/// [`DefaultInstallers::as_set`]. Keeps the wire-up at each call site
/// down to two lines.
pub struct DefaultInstallers {
    pub mcp: mcp::McpInstaller,
    pub cli: cli_tool::CliToolInstaller,
    pub skill: skill::SkillInstaller,
    pub plugin: plugin::PluginInstaller,
}

impl DefaultInstallers {
    pub fn new() -> Self {
        Self {
            mcp: mcp::McpInstaller::default(),
            cli: cli_tool::CliToolInstaller::default(),
            skill: skill::SkillInstaller,
            plugin: plugin::PluginInstaller,
        }
    }

    pub fn as_set(&self) -> InstallerSet<'_> {
        InstallerSet {
            mcp: &self.mcp,
            cli: &self.cli,
            skill: &self.skill,
            plugin: &self.plugin,
        }
    }
}

impl Default for DefaultInstallers {
    fn default() -> Self {
        Self::new()
    }
}

/// Run `npm install <pkg>@<ver> --prefix <dir> --no-save` with optional extra_deps.
/// Returns the install directory path.
pub fn run_npm_install(
    action: &PlannedAction,
    ctx: &InstallContext,
    registry: &Registry,
) -> Result<PathBuf, InstallError> {
    let install_dir = ctx.packages_dir.join(&action.name);
    let pkg_version = format!("{}@{}", action.package, action.version);

    let install_dir_str = install_dir.to_string_lossy().into_owned();
    let mut args: Vec<&str> = vec![
        "install",
        &pkg_version,
        "--prefix",
        &install_dir_str,
        "--no-save",
    ];

    let override_deps: Vec<&str> = registry
        .get_override(&action.package)
        .map(|ov| ov.extra_deps.iter().map(String::as_str).collect())
        .unwrap_or_default();
    args.extend(override_deps);

    ctx.runner
        .run("npm", &args, ctx.project_root, &[])
        .map_err(|e| InstallError::Command("npm install".to_string(), e.to_string()))?;

    Ok(install_dir)
}
