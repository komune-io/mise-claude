pub mod cli_tool;
pub mod mcp;
pub mod plugin;
pub mod skill;

use crate::error::InstallError;
use crate::process::CommandRunner;
use crate::registry::Registry;
use crate::resolver::PlannedAction;
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

pub struct InstallResult {
    pub integrity: Option<String>,
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
