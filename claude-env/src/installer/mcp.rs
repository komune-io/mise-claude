use std::path::Path;
use std::process::Command;

use crate::error::InstallError;
use crate::mcp_config::{self, McpEntry};
use crate::registry::Registry;
use crate::resolver::PlannedAction;

use super::{InstallContext, InstallResult, Installer};

pub struct McpInstaller {
    registry: Registry,
}

impl Default for McpInstaller {
    fn default() -> Self {
        Self {
            registry: Registry::default(),
        }
    }
}

impl Installer for McpInstaller {
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

        // Detect the binary.
        let bin_dir = install_dir.join("node_modules").join(".bin");
        let bin_name = self.detect_binary(&bin_dir, &action.package)?;

        // Write .mcp.json entry.
        let bin_path = bin_dir.join(&bin_name);
        let entry = McpEntry {
            command: bin_path.to_string_lossy().into_owned(),
            args: vec![],
        };

        mcp_config::ensure_server(ctx.project_root, &action.name, &entry)
            .map_err(|e| InstallError::Config(".mcp.json".to_string(), e.to_string()))?;

        Ok(InstallResult {
            installed: true,
            integrity: None,
        })
    }
}

impl McpInstaller {
    /// Detect the binary name inside `bin_dir`.
    ///
    /// Uses the `bin_name` override from the registry if present; otherwise
    /// picks the first non-dot file in the directory.
    fn detect_binary(&self, bin_dir: &Path, package: &str) -> Result<String, InstallError> {
        // Check for a registry override first.
        if let Some(ov) = self.registry.get_override(package) {
            if let Some(ref name) = ov.bin_name {
                return Ok(name.clone());
            }
        }

        // Fall back to the first non-dot entry in bin_dir.
        let entries = std::fs::read_dir(bin_dir).map_err(|e| {
            InstallError::Command(
                "detect_binary".to_string(),
                format!("cannot read {}: {}", bin_dir.display(), e),
            )
        })?;

        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if !name_str.starts_with('.') {
                return Ok(name_str.into_owned());
            }
        }

        Err(InstallError::Command(
            "detect_binary".to_string(),
            format!("no binary found in {}", bin_dir.display()),
        ))
    }
}
