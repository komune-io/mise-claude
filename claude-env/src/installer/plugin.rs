use std::process::Command;

use crate::error::InstallError;
use crate::resolver::PlannedAction;

use super::{InstallContext, InstallResult, Installer};

pub struct PluginInstaller;

struct PluginParts {
    owner_repo: String,
    plugin: String,
    marketplace: String,
}

impl PluginInstaller {
    /// Parse a plugin path in the form "owner/repo/plugin@marketplace".
    ///
    /// Returns [`PluginParts`] with the parsed components.
    fn parse_plugin_path(&self, name: &str) -> Result<PluginParts, InstallError> {
        // Find the last '@' to split plugin@marketplace.
        let at_pos = name.rfind('@').ok_or_else(|| {
            InstallError::Command(
                "parse_plugin_path".to_string(),
                format!("expected 'owner/repo/plugin@marketplace', got: '{name}'"),
            )
        })?;

        let path_part = &name[..at_pos];
        let marketplace = name[at_pos + 1..].to_string();

        // Split path_part by '/' — must yield exactly 3 components.
        let parts: Vec<&str> = path_part.splitn(4, '/').collect();
        if parts.len() != 3 {
            return Err(InstallError::Command(
                "parse_plugin_path".to_string(),
                format!(
                    "expected 'owner/repo/plugin@marketplace', got: '{name}'"
                ),
            ));
        }

        let last_slash = path_part.rfind('/').unwrap();
        let owner_repo = path_part[..last_slash].to_string();
        let plugin = parts[2].to_string();

        Ok(PluginParts {
            owner_repo,
            plugin,
            marketplace,
        })
    }
}

impl Installer for PluginInstaller {
    fn install(
        &self,
        action: &PlannedAction,
        ctx: &InstallContext,
    ) -> Result<InstallResult, InstallError> {
        let parts = self.parse_plugin_path(&action.name)?;

        // Step 1: claude plugin marketplace add <owner_repo>
        let marketplace_args = ["plugin", "marketplace", "add", &parts.owner_repo];

        if ctx.verbose {
            eprintln!("[verbose] claude {}", marketplace_args.join(" "));
        }

        let status = Command::new("claude")
            .args(&marketplace_args)
            .current_dir(ctx.project_root)
            .status()
            .map_err(|e| InstallError::Command("claude".to_string(), e.to_string()))?;

        if !status.success() {
            return Err(InstallError::Command(
                "claude plugin marketplace add".to_string(),
                format!("exited with status {}", status),
            ));
        }

        // Step 2: claude plugin install <plugin>@<marketplace> --scope project
        let plugin_id = format!("{}@{}", parts.plugin, parts.marketplace);
        let install_args = ["plugin", "install", &plugin_id, "--scope", "project"];

        if ctx.verbose {
            eprintln!("[verbose] claude {}", install_args.join(" "));
        }

        let status = Command::new("claude")
            .args(&install_args)
            .current_dir(ctx.project_root)
            .status()
            .map_err(|e| InstallError::Command("claude".to_string(), e.to_string()))?;

        if !status.success() {
            return Err(InstallError::Command(
                "claude plugin install".to_string(),
                format!("exited with status {}", status),
            ));
        }

        Ok(InstallResult {
            installed: true,
            integrity: None,
        })
    }
}
