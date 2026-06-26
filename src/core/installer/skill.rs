use crate::core::error::InstallError;
use crate::core::resolver::PlannedAction;
use crate::core::skills::{discover, git, github_url, materialize, SkillError};

use super::{InstallContext, InstallResult, Installer, MaterializedSkill};

pub struct SkillInstaller;

impl SkillInstaller {
    /// Parse a skill path. Two accepted shapes:
    ///
    /// - `"owner/repo/skill-name"` → install one named skill.
    /// - `"owner/repo"` → install every skill exposed by the repo
    ///   (wildcard: selector is `"*"`).
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
        let (owner_repo, selector) = self
            .parse_skill_path(&action.name)
            .map_err(|e| InstallError::Command("parse skill".into(), e.to_string()))?;

        let url = github_url(owner_repo);
        // action.version is the chord.toml ref (latest/tag/branch/sha).
        let sha = git::resolve_ref(ctx.runner, &url, &action.version, ctx.project_root)
            .map_err(skill_err)?;

        let cache_dir = ctx.project_root.join(".chord").join(".cache").join(&sha);
        let checkout = git::fetch_commit(ctx.runner, &url, &sha, &cache_dir).map_err(skill_err)?;

        let repo_name = owner_repo.rsplit('/').next().unwrap_or(owner_repo);
        let skills = if selector == "*" {
            discover::discover_all(&checkout, repo_name)
        } else {
            match discover::find_one(&checkout, repo_name, selector) {
                Some(s) => vec![s],
                None => {
                    return Err(skill_err(SkillError::SkillNotFound(
                        selector.to_string(),
                        checkout.display().to_string(),
                    )))
                }
            }
        };
        if skills.is_empty() {
            return Err(skill_err(SkillError::SkillNotFound(
                selector.to_string(),
                checkout.display().to_string(),
            )));
        }

        let mut materialized = Vec::new();
        for skill in &skills {
            let integrity =
                materialize::materialize(ctx.project_root, owner_repo, skill).map_err(skill_err)?;
            materialized.push(MaterializedSkill {
                flat_name: skill.name.clone(),
                integrity,
            });
        }

        Ok(InstallResult {
            integrity: None,
            commit: sha.clone(),
            materialized,
        })
    }
}

fn skill_err(e: SkillError) -> InstallError {
    InstallError::Command("skill install".into(), e.to_string())
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
