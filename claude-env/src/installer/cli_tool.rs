use std::process::Command;

use crate::error::InstallError;
use crate::registry::Registry;
use crate::resolver::PlannedAction;

use super::{InstallContext, InstallResult, Installer};

pub struct CliToolInstaller {
    registry: Registry,
}

impl Default for CliToolInstaller {
    fn default() -> Self {
        Self {
            registry: Registry::default(),
        }
    }
}

impl Installer for CliToolInstaller {
    fn install(
        &self,
        action: &PlannedAction,
        ctx: &InstallContext,
    ) -> Result<InstallResult, InstallError> {
        let install_dir = ctx.packages_dir.join(&action.name);
        let pkg_version = format!("{}@{}", action.package, action.version);

        // Build the npm install argument list.
        let mut args = vec![
            "install".to_string(),
            pkg_version,
            "--prefix".to_string(),
            install_dir.to_string_lossy().into_owned(),
            "--no-save".to_string(),
        ];

        // Append extra_deps from registry.
        if let Some(ov) = self.registry.get_override(&action.package) {
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

        // Run post_install if configured (CLI type skips .mcp.json).
        if let Some(ov) = self.registry.get_override(&action.package) {
            let project_root_str = ctx.project_root.to_string_lossy();
            if let Some(cmd) = ov.resolve_post_install(&project_root_str) {
                if ctx.verbose {
                    eprintln!("[verbose] post_install: {cmd}");
                }

                // Prepend node_modules/.bin to PATH so the just-installed binary is available.
                let bin_dir = install_dir.join("node_modules").join(".bin");
                let path_env = std::env::var("PATH").unwrap_or_default();
                let new_path = format!("{}:{}", bin_dir.display(), path_env);

                let status = Command::new("sh")
                    .arg("-c")
                    .arg(&cmd)
                    .current_dir(ctx.project_root)
                    .env("PATH", &new_path)
                    .status()
                    .map_err(|e| InstallError::Command("sh".to_string(), e.to_string()))?;

                if !status.success() {
                    return Err(InstallError::Command(
                        "post_install".to_string(),
                        format!("exited with status {}", status),
                    ));
                }
            }
        }

        Ok(InstallResult {
            installed: true,
            integrity: None,
        })
    }
}
