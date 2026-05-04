pub mod cli_tool;
pub mod mcp;
pub mod plugin;
pub mod skill;

use crate::error::InstallError;
use crate::registry::Registry;
use crate::resolver::PlannedAction;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct InstallContext<'a> {
    pub project_root: &'a Path,
    pub packages_dir: &'a Path,
    pub verbose: bool,
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

    let mut args = vec![
        "install".to_string(),
        pkg_version,
        "--prefix".to_string(),
        install_dir.to_string_lossy().into_owned(),
        "--no-save".to_string(),
    ];

    if let Some(ov) = registry.get_override(&action.package) {
        for dep in &ov.extra_deps {
            args.push(dep.clone());
        }
    }

    if ctx.verbose {
        eprintln!("[verbose] npm {}", args.join(" "));
    }

    let status = Command::new("npm")
        .args(&args)
        .status()
        .map_err(|e| InstallError::Command("npm".to_string(), e.to_string()))?;

    if !status.success() {
        return Err(InstallError::Command(
            "npm install".to_string(),
            format!("exited with status {}", status),
        ));
    }

    Ok(install_dir)
}
