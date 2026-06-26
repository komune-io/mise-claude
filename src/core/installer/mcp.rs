use std::path::Path;

use crate::core::error::InstallError;
use crate::core::mcp_config::{self, McpEntry};
use crate::core::registry::Registry;
use crate::core::resolver::PlannedAction;

use super::{run_npm_install, InstallContext, InstallResult, Installer};

#[derive(Default)]
pub struct McpInstaller {
    registry: Registry,
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

        Ok(InstallResult {
            integrity: None,
            commit: String::new(),
            materialized: Vec::new(),
        })
    }
}

fn detect_binary(
    bin_dir: &Path,
    package: &str,
    registry: &Registry,
) -> Result<String, InstallError> {
    if let Some(ov) = registry.get_override(package) {
        if let Some(ref name) = ov.bin_name {
            return Ok(name.clone());
        }
    }

    let entries = std::fs::read_dir(bin_dir).map_err(|e| {
        InstallError::Command(
            "detect_binary".to_string(),
            format!("cannot read {}: {}", bin_dir.display(), e),
        )
    })?;

    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.'))
        .collect();

    pick_first_binary(&mut names).ok_or_else(|| {
        InstallError::Command(
            "detect_binary".to_string(),
            format!("no binary found in {}", bin_dir.display()),
        )
    })
}

/// Sort the candidate binary names lexically and return the first.
///
/// Public (not `pub(crate)`) so the external integration-test crate can
/// import it; the deterministic-ordering behavior is unit-tested without
/// a filesystem fixture.
pub fn pick_first_binary(names: &mut [String]) -> Option<String> {
    names.sort();
    names.first().cloned()
}
