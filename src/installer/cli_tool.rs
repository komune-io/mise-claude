use crate::error::InstallError;
use crate::registry::Registry;
use crate::resolver::PlannedAction;

use super::{run_npm_install, InstallContext, InstallResult, Installer};

#[derive(Default)]
pub struct CliToolInstaller {
    registry: Registry,
}

impl Installer for CliToolInstaller {
    fn install(
        &self,
        action: &PlannedAction,
        ctx: &InstallContext,
    ) -> Result<InstallResult, InstallError> {
        let install_dir = run_npm_install(action, ctx, &self.registry)?;

        if let Some(ov) = self.registry.get_override(&action.package) {
            let project_root_str = ctx.project_root.to_string_lossy();
            if let Some(cmd) = ov.resolve_post_install(&project_root_str) {
                let bin_dir = install_dir.join("node_modules").join(".bin");
                let path_env = std::env::var("PATH").unwrap_or_default();
                let new_path = format!("{}:{}", bin_dir.display(), path_env);

                ctx.runner
                    .run(
                        "sh",
                        &["-c", &cmd],
                        ctx.project_root,
                        &[("PATH", &new_path)],
                    )
                    .map_err(|e| {
                        InstallError::Command("post_install".to_string(), e.to_string())
                    })?;
            }
        }

        Ok(InstallResult { integrity: None })
    }
}
