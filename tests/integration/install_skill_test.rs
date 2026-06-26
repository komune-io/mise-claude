//! Integration tests for `chord install` with skill entries in chord.toml.
//!
//! These tests use a local git fixture (via CHORD_SKILLS_BASE_URL) so no
//! network access is required. The fixture is a git repo at
//! `base/o/r.git` which `github_url("o/r")` resolves to.

use assert_cmd::Command;
use std::path::Path;
use std::process::Command as StdCommand;
use tempfile::TempDir;

fn git(args: &[&str], cwd: &Path) {
    let ok = StdCommand::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?} failed");
}

/// Create a git repo with two skills (foo, bar) at the given path.
fn make_skill_repo(repo_path: &Path) {
    std::fs::create_dir_all(repo_path.join("skills/foo")).unwrap();
    std::fs::create_dir_all(repo_path.join("skills/bar")).unwrap();
    std::fs::write(
        repo_path.join("skills/foo/SKILL.md"),
        "---\nname: foo\n---\n",
    )
    .unwrap();
    std::fs::write(
        repo_path.join("skills/bar/SKILL.md"),
        "---\nname: bar\n---\n",
    )
    .unwrap();
    git(&["init"], repo_path);
    git(&["config", "user.email", "t@t"], repo_path);
    git(&["config", "user.name", "t"], repo_path);
    git(&["add", "."], repo_path);
    git(&["commit", "-m", "init"], repo_path);
}

#[test]
fn install_skill() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();

    // Create fixture: base/o/r.git → github_url("o/r") = "{base}/o/r.git"
    let base_dir = TempDir::new().unwrap();
    let repo_path = base_dir.path().join("o").join("r.git");
    make_skill_repo(&repo_path);

    let base_url = format!("file://{}", base_dir.path().display());

    // chord.toml with a named skill.
    std::fs::write(
        project_dir.path().join("chord.toml"),
        "[skills]\n\"o/r/foo\" = \"latest\"\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("chord").unwrap();
    cmd.arg("install")
        .current_dir(project_dir.path())
        .env("CHORD_HOME", packages_dir.path())
        .env("CHORD_SKILLS_BASE_URL", &base_url);

    cmd.assert().success();

    // Verify the skill was materialized.
    let store = project_dir.path().join(".chord/o/r/foo/SKILL.md");
    assert!(
        store.is_file(),
        "SKILL.md should be materialized in .chord store"
    );

    let link = project_dir.path().join(".claude/skills/foo");
    assert!(
        link.exists() || link.is_symlink(),
        "symlink .claude/skills/foo should exist"
    );

    // Verify lockfile records the skill.
    let lock_path = project_dir.path().join("chord.lock");
    assert!(lock_path.exists(), "chord.lock should exist");
    let lock_content = std::fs::read_to_string(&lock_path).unwrap();
    assert!(
        lock_content.contains("foo"),
        "lockfile should contain 'foo', got: {lock_content}"
    );
    assert!(
        lock_content.contains("sha256:"),
        "lockfile should contain integrity hash, got: {lock_content}"
    );
}

#[test]
fn install_skill_wildcard() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();

    let base_dir = TempDir::new().unwrap();
    let repo_path = base_dir.path().join("o").join("r.git");
    make_skill_repo(&repo_path);

    let base_url = format!("file://{}", base_dir.path().display());

    // Two-segment key → wildcard install: all skills from the repo.
    std::fs::write(
        project_dir.path().join("chord.toml"),
        "[skills]\n\"o/r\" = \"latest\"\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("chord").unwrap();
    cmd.arg("install")
        .current_dir(project_dir.path())
        .env("CHORD_HOME", packages_dir.path())
        .env("CHORD_SKILLS_BASE_URL", &base_url);

    cmd.assert().success();

    // Both skills should be materialized.
    assert!(
        project_dir.path().join(".chord/o/r/foo/SKILL.md").is_file(),
        "foo SKILL.md should be in store"
    );
    assert!(
        project_dir.path().join(".chord/o/r/bar/SKILL.md").is_file(),
        "bar SKILL.md should be in store"
    );

    let lock_path = project_dir.path().join("chord.lock");
    let lock_content = std::fs::read_to_string(&lock_path).unwrap();
    // Wildcard anchor row records both sub-skills.
    assert!(
        lock_content.contains("foo"),
        "lockfile should contain 'foo', got: {lock_content}"
    );
    assert!(
        lock_content.contains("bar"),
        "lockfile should contain 'bar', got: {lock_content}"
    );
}
