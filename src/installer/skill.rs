use crate::error::InstallError;
use crate::resolver::PlannedAction;

use super::{InstallContext, InstallResult, Installer};

pub struct SkillInstaller;

impl SkillInstaller {
    /// Parse a skill path. Two accepted shapes:
    ///
    /// - `"owner/repo/skill-name"` → install one named skill.
    /// - `"owner/repo"` → install every skill exposed by the repo
    ///   (the underlying `skills` CLI receives `--skill '*'`).
    ///
    /// Returns `(owner_repo, skill)`. The wildcard form returns
    /// `(name, "*")` so the caller can pass it through verbatim.
    fn parse_skill_path<'a>(&self, name: &'a str) -> Result<(&'a str, &'a str), InstallError> {
        let parts: Vec<&str> = name.splitn(4, '/').collect();
        match parts.len() {
            2 => Ok((name, "*")),
            3 => {
                // owner_repo spans from start to just before the last '/'
                let last_slash = name.rfind('/').unwrap();
                let owner_repo = &name[..last_slash];
                let skill = parts[2];
                Ok((owner_repo, skill))
            }
            _ => Err(InstallError::Command(
                "parse_skill_path".to_string(),
                format!("expected 'owner/repo' or 'owner/repo/skill-name', got: '{name}'"),
            )),
        }
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

        ctx.runner
            .run("npx", &args, ctx.project_root, &[])
            .map_err(|e| InstallError::Command("npx skills add".to_string(), e.to_string()))?;

        Ok(InstallResult { integrity: None })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_three_segment_path() {
        let installer = SkillInstaller;
        let (owner_repo, skill) = installer
            .parse_skill_path("vercel-labs/next-skills/next-best-practices")
            .unwrap();
        assert_eq!(owner_repo, "vercel-labs/next-skills");
        assert_eq!(skill, "next-best-practices");
    }

    #[test]
    fn parses_two_segment_path_as_wildcard() {
        let installer = SkillInstaller;
        let (owner_repo, skill) = installer.parse_skill_path("mattpocock/skills").unwrap();
        assert_eq!(owner_repo, "mattpocock/skills");
        assert_eq!(skill, "*");
    }

    #[test]
    fn rejects_one_segment() {
        let installer = SkillInstaller;
        assert!(installer.parse_skill_path("lonely").is_err());
    }

    #[test]
    fn rejects_four_segments() {
        let installer = SkillInstaller;
        assert!(installer.parse_skill_path("a/b/c/d").is_err());
    }
}
