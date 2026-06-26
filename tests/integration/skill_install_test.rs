//! Offline integration test for the git-native SkillInstaller.
//!
//! Builds a local git fixture with two skills (foo, bar), then drives the
//! real installer via `operations::install_one` and the lower-level
//! git+discover+materialize helpers. No network access required.
//!
//! Directory layout used by the end-to-end test:
//!   tmp/
//!     base/o/r.git/   ← git repo for owner="o", repo="r"
//!                         (github_url("o/r") → "{base}/o/r.git")
//!     proj/           ← project root
//!     home/           ← fake HOME

use std::path::Path;
use std::process::Command;

fn git(args: &[&str], cwd: &Path) {
    let ok = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .status()
        .unwrap()
        .success();
    assert!(ok, "git {args:?} failed in {}", cwd.display());
}

/// Create a git repo with two skills at `wt_path`, return the HEAD SHA.
fn make_repo_at(wt_path: &Path) -> String {
    std::fs::create_dir_all(wt_path.join("skills/foo")).unwrap();
    std::fs::create_dir_all(wt_path.join("skills/bar")).unwrap();
    std::fs::write(wt_path.join("skills/foo/SKILL.md"), "---\nname: foo\n---\n").unwrap();
    std::fs::write(wt_path.join("skills/bar/SKILL.md"), "---\nname: bar\n---\n").unwrap();
    git(&["init"], wt_path);
    git(&["config", "user.email", "t@t"], wt_path);
    git(&["config", "user.name", "t"], wt_path);
    git(&["add", "."], wt_path);
    git(&["commit", "-m", "init"], wt_path);
    let out = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(wt_path)
        .output()
        .unwrap();
    String::from_utf8(out.stdout).unwrap().trim().to_string()
}

/// Test the lower-level git+discover+materialize helpers directly.
#[test]
fn named_skill_materializes_store_and_symlink() {
    let tmp = tempfile::tempdir().unwrap();
    // Repo lives at tmp/upstream (regular non-bare git repo).
    let wt = tmp.path().join("upstream");
    let sha = make_repo_at(&wt);
    // file:// URL pointing at the repo root works with modern git.
    let url = format!("file://{}", wt.display());

    let project = tmp.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();
    let runner = chord::core::process::SystemCommandRunner::new(false);
    let cache = project.join(".chord/.cache").join(&sha);

    let checkout = chord::core::skills::git::fetch_commit(&runner, &url, &sha, &cache).unwrap();
    let skill = chord::core::skills::discover::find_one(&checkout, "r", "foo").unwrap();
    let integrity = chord::core::skills::materialize::materialize(&project, "o/r", &skill).unwrap();

    assert!(integrity.starts_with("sha256:"), "integrity: {integrity}");
    assert!(
        project.join(".chord/o/r/foo/SKILL.md").is_file(),
        "SKILL.md should be in store"
    );
    let link = project.join(".claude/skills/foo");
    assert_eq!(
        std::fs::read_link(&link).unwrap(),
        Path::new("../../.chord/o/r/foo")
    );
}

/// Test the full SkillInstaller path via `operations::install_one`.
///
/// Uses `CHORD_SKILLS_BASE_URL` to redirect `github_url("o/r")` to the
/// local fixture at `base/o/r.git` so no network is needed.
#[test]
fn skill_installer_end_to_end_via_operations() {
    use chord::core::installer::DefaultInstallers;
    use chord::core::operations::install::install_one;
    use chord::core::operations::OpContext;
    use chord::core::store::{FileConfigStore, FileLockfileStore};

    let tmp = tempfile::tempdir().unwrap();

    // Create fixture at base/o/r.git — github_url("o/r") → "{base}/o/r.git"
    let repo_dir = tmp.path().join("base").join("o").join("r.git");
    make_repo_at(&repo_dir);

    let base_url = format!("file://{}/base", tmp.path().display());

    let project = tmp.path().join("proj");
    std::fs::create_dir_all(&project).unwrap();

    let home = tmp.path().join("home");
    std::fs::create_dir_all(&home).unwrap();

    // chord.toml declares a named skill using "latest" so resolve_ref calls
    // `git ls-remote` to get HEAD (works fine with file:// repos).
    std::fs::write(
        project.join("chord.toml"),
        "[skills]\n\"o/r/foo\" = \"latest\"\n",
    )
    .unwrap();

    let packages_dir = tmp.path().join("packages");
    std::fs::create_dir_all(&packages_dir).unwrap();

    let config_store = FileConfigStore::new(&project);
    let lockfile_store = FileLockfileStore::new(&project);
    let installers = DefaultInstallers::new();
    let installer_set = installers.as_set();

    let ctx = OpContext {
        config_store: &config_store,
        lockfile_store: &lockfile_store,
        installers: &installer_set,
        project_root: &project,
        home_dir: &home,
        packages_dir: &packages_dir,
        verbose: false,
    };

    // Set env var so github_url() resolves to our local fixture.
    // Tests run in parallel; we set/unset quickly around the call.
    std::env::set_var("CHORD_SKILLS_BASE_URL", &base_url);
    let outcome = install_one("o/r/foo", &ctx, true);
    std::env::remove_var("CHORD_SKILLS_BASE_URL");

    let outcome = outcome.expect("install_one should succeed");
    assert_eq!(outcome.installed, 1, "expected one installed skill");

    // Verify materialized artifacts.
    assert!(
        project.join(".chord/o/r/foo/SKILL.md").is_file(),
        "SKILL.md should be in store"
    );
    let link = project.join(".claude/skills/foo");
    assert!(link.exists() || link.is_symlink(), "symlink should exist");

    // Verify lockfile was written with the resolved SHA and integrity.
    let lock_content =
        std::fs::read_to_string(project.join("chord.lock")).expect("chord.lock should exist");
    assert!(
        lock_content.contains("sha256:"),
        "lockfile should contain integrity hash, got:\n{lock_content}"
    );
}
