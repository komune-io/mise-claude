use std::path::Path;

use crate::error::InstallError;
use crate::mcp_config::{self, McpEntry};
use crate::registry::Registry;
use crate::resolver::PlannedAction;

use super::{run_npm_install, InstallContext, InstallResult, Installer};

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
        let install_dir = run_npm_install(action, ctx, &self.registry)?;

        let bin_dir = install_dir.join("node_modules").join(".bin");
        let bin_name = detect_binary(&bin_dir, &action.package, &self.registry)?;

        let bin_path = bin_dir.join(&bin_name);
        let entry = McpEntry {
            command: bin_path.to_string_lossy().into_owned(),
            args: vec![],
        };

        mcp_config::ensure_server(ctx.project_root, &action.name, &entry)
            .map_err(|e| InstallError::Config(".mcp.json".to_string(), e.to_string()))?;

        Ok(InstallResult { integrity: None })
    }
}

fn detect_binary(bin_dir: &Path, package: &str, registry: &Registry) -> Result<String, InstallError> {
    if let Some(ov) = registry.get_override(package) {
        if let Some(ref name) = ov.bin_name {
            return Ok(name.clone());
        }
    }

    let entries = std::fs::read_dir(bin_dir).map_err(|e| {
        InstallError::Command("detect_binary".to_string(), format!("cannot read {}: {}", bin_dir.display(), e))
    })?;

    for entry in entries.flatten() {
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if !name_str.starts_with('.') {
            return Ok(name_str.into_owned());
        }
    }

    Err(InstallError::Command("detect_binary".to_string(), format!("no binary found in {}", bin_dir.display())))
}
