use std::process::Command;

use crate::error::InstallError;
use crate::resolver::PlannedAction;

use super::{InstallContext, InstallResult, Installer};

pub struct SkillInstaller;

impl SkillInstaller {
    /// Parse a skill path in the form "owner/repo/skill-name".
    ///
    /// Returns `(owner_repo, skill)` where `owner_repo = "owner/repo"`.
    fn parse_skill_path<'a>(&self, name: &'a str) -> Result<(&'a str, &'a str), InstallError> {
        let parts: Vec<&str> = name.splitn(4, '/').collect();
        if parts.len() != 3 {
            return Err(InstallError::Command(
                "parse_skill_path".to_string(),
                format!(
                    "expected 'owner/repo/skill-name', got: '{name}'"
                ),
            ));
        }
        // owner_repo spans from start to just before the last '/'
        let last_slash = name.rfind('/').unwrap();
        let owner_repo = &name[..last_slash];
        let skill = parts[2];
        Ok((owner_repo, skill))
    }
}

impl Installer for SkillInstaller {
    fn install(
        &self,
        action: &PlannedAction,
        ctx: &InstallContext,
    ) -> Result<InstallResult, InstallError> {
        let (owner_repo, skill) = self.parse_skill_path(&action.name)?;

        let args = [
            "skills",
            "add",
            owner_repo,
            "--skill",
            skill,
            "-a",
            "claude-code",
            "-y",
        ];

        if ctx.verbose {
            eprintln!("[verbose] npx {}", args.join(" "));
        }

        let status = Command::new("npx")
            .args(&args)
            .current_dir(ctx.project_root)
            .status()
            .map_err(|e| InstallError::Command("npx".to_string(), e.to_string()))?;

        if !status.success() {
            return Err(InstallError::Command(
                "npx skills add".to_string(),
                format!("exited with status {}", status),
            ));
        }

        Ok(InstallResult {
            installed: true,
            integrity: None,
        })
    }
}
