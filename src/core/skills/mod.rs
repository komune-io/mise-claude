//! Git-native Skill installation: resolve a ref to a commit, fetch it
//! shallowly, discover skills in the tree, and materialize them into the
//! chord-owned `.chord/` store with a symlink into `.claude/skills/`.

pub mod discover;
pub mod git;
pub mod materialize;

use thiserror::Error;

/// Errors raised while installing a Skill from git.
#[derive(Debug, Error)]
pub enum SkillError {
    #[error("git command failed: {0}")]
    Git(String),

    #[error("could not resolve ref '{0}' in {1}")]
    UnresolvedRef(String, String),

    #[error("skill '{0}' not found in repo at {1}")]
    SkillNotFound(String, String),

    #[error("invalid skill key '{0}': expected 'owner/repo' or 'owner/repo/name'")]
    InvalidKey(String),

    #[error(
        "skill name collision: '{0}' is already managed from '{1}', cannot install from '{2}'"
    )]
    NameCollision(String, String, String),

    #[error("filesystem error at {0}: {1}")]
    Io(String, String),
}

/// Build the clone URL for a GitHub `owner/repo`.
///
/// If the `CHORD_SKILLS_BASE_URL` environment variable is set, it is used as
/// the base (e.g. `file:///tmp/fixture`), enabling fully offline tests.
/// In production the default is `https://github.com`.
pub fn github_url(owner_repo: &str) -> String {
    let base =
        std::env::var("CHORD_SKILLS_BASE_URL").unwrap_or_else(|_| "https://github.com".to_string());
    format!("{base}/{owner_repo}.git")
}

/// True if `s` looks like a full 40-character lowercase/uppercase hex SHA.
pub fn looks_like_sha(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn github_url_default() {
        // Ensure env var is not set for this test.
        let _guard = EnvVarGuard::unset("CHORD_SKILLS_BASE_URL");
        assert_eq!(
            github_url("owner/repo"),
            "https://github.com/owner/repo.git"
        );
    }

    #[test]
    fn github_url_override() {
        let _guard = EnvVarGuard::set("CHORD_SKILLS_BASE_URL", "file:///tmp/fixture");
        assert_eq!(github_url("o/r"), "file:///tmp/fixture/o/r.git");
    }

    /// RAII helper: restores (or removes) an env var on drop.
    struct EnvVarGuard {
        key: &'static str,
        prev: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, val: &str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::set_var(key, val);
            Self { key, prev }
        }

        fn unset(key: &'static str) -> Self {
            let prev = std::env::var(key).ok();
            std::env::remove_var(key);
            Self { key, prev }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match &self.prev {
                Some(v) => std::env::set_var(self.key, v),
                None => std::env::remove_var(self.key),
            }
        }
    }
}
