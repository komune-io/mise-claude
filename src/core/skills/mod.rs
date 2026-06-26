//! Git-native Skill installation: resolve a ref to a commit, fetch it
//! shallowly, discover skills in the tree, and materialize them into the
//! chord-owned `.chord/` store with a symlink into `.claude/skills/`.

pub mod git;

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

/// Build the canonical HTTPS clone URL for a GitHub `owner/repo`.
pub fn github_url(owner_repo: &str) -> String {
    format!("https://github.com/{owner_repo}.git")
}

/// True if `s` looks like a full 40-character lowercase/uppercase hex SHA.
pub fn looks_like_sha(s: &str) -> bool {
    s.len() == 40 && s.chars().all(|c| c.is_ascii_hexdigit())
}
