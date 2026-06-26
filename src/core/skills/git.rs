//! Git operations for Skill installation, all routed through the
//! [`CommandRunner`] port so they are testable without real subprocesses.

use std::path::{Path, PathBuf};

use crate::core::process::CommandRunner;

use super::{looks_like_sha, SkillError};

/// Resolve a git ref to a concrete commit SHA.
///
/// A full SHA is returned unchanged (no network). `latest` maps to the
/// remote `HEAD`. Any other ref (branch/tag) is resolved via
/// `git ls-remote <url> <ref>`, taking the SHA in the first column of the
/// first output line.
pub fn resolve_ref(
    runner: &dyn CommandRunner,
    url: &str,
    git_ref: &str,
    cwd: &Path,
) -> Result<String, SkillError> {
    if looks_like_sha(git_ref) {
        return Ok(git_ref.to_string());
    }
    let query = if git_ref == "latest" { "HEAD" } else { git_ref };
    let stdout = runner
        .run_capture("git", &["ls-remote", url, query], cwd, &[])
        .map_err(|e| SkillError::Git(e.to_string()))?;
    let sha = stdout
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().next())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| SkillError::UnresolvedRef(git_ref.to_string(), url.to_string()))?;
    Ok(sha.to_string())
}

/// Shallow-fetch `sha` from `url` into `cache_dir` and check it out detached.
///
/// Idempotent: if `cache_dir/.git` already exists the fetch is assumed done
/// and the existing checkout is returned (a SHA is immutable).
pub fn fetch_commit(
    runner: &dyn CommandRunner,
    url: &str,
    sha: &str,
    cache_dir: &Path,
) -> Result<PathBuf, SkillError> {
    let dir = cache_dir.to_path_buf();
    if dir.join(".git").exists() {
        return Ok(dir);
    }
    std::fs::create_dir_all(&dir)
        .map_err(|e| SkillError::Io(dir.display().to_string(), e.to_string()))?;
    let dir_str = dir.to_string_lossy().into_owned();

    let run = |args: &[&str]| -> Result<(), SkillError> {
        runner
            .run("git", args, &dir, &[])
            .map_err(|e| SkillError::Git(e.to_string()))
    };

    run(&["init", &dir_str])?;
    run(&["-C", &dir_str, "remote", "add", "origin", url])?;
    run(&["-C", &dir_str, "fetch", "--depth", "1", "origin", sha])?;
    run(&["-C", &dir_str, "checkout", "--detach", "FETCH_HEAD"])?;
    Ok(dir)
}
