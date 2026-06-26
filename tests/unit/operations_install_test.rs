//! In-memory unit tests for `operations::install::install_all` and
//! `install_one`. The corresponding integration tests under
//! `tests/integration/install_*` exercise the same logic via the file
//! adapter; this file covers the same logic via `InMemoryConfigStore` /
//! `InMemoryLockfileStore` for speed.

use chord::core::config::Config;
use chord::core::installer::InstallerSet;
use chord::core::operations::{install, OpContext, OperationError};
use chord::core::store::{InMemoryConfigStore, InMemoryLockfileStore};
use std::path::Path;
use tempfile::TempDir;

fn ctx_with<'a>(
    config_store: &'a InMemoryConfigStore,
    lockfile_store: &'a InMemoryLockfileStore,
    installers: &'a InstallerSet<'a>,
    packages_dir: &'a Path,
) -> OpContext<'a> {
    OpContext {
        config_store,
        lockfile_store,
        installers,
        project_root: Path::new("."),
        home_dir: Path::new("."),
        packages_dir,
        verbose: false,
    }
}

#[test]
fn install_all_with_empty_config_succeeds() {
    let config_store = InMemoryConfigStore::empty();
    let lockfile_store = InMemoryLockfileStore::empty();
    let installers = chord::core::installer::DefaultInstallers::new();
    let installer_set = installers.as_set();
    // packages_dir is consulted by the `is_installed` closure but never
    // reached with an empty plan, so a placeholder is fine.
    let packages = TempDir::new().unwrap();
    let ctx = ctx_with(
        &config_store,
        &lockfile_store,
        &installer_set,
        packages.path(),
    );

    let outcome = install::install_all(&ctx, false).unwrap();
    assert_eq!(outcome.installed, 0);
    assert_eq!(outcome.failed, 0);
    assert_eq!(outcome.skipped, 0);
}

#[test]
fn install_one_missing_tool_returns_not_found() {
    let mut seeded = Config::default();
    seeded
        .mcp
        .insert("context7".to_string(), "latest".to_string());
    let config_store = InMemoryConfigStore::new(seeded);
    let lockfile_store = InMemoryLockfileStore::empty();
    let installers = chord::core::installer::DefaultInstallers::new();
    let installer_set = installers.as_set();
    let packages = TempDir::new().unwrap();
    let ctx = ctx_with(
        &config_store,
        &lockfile_store,
        &installer_set,
        packages.path(),
    );

    let err = install::install_one("nope", &ctx, false).unwrap_err();
    assert!(matches!(err, OperationError::NotFound(_)));
}

#[test]
fn prune_orphan_skills_removes_vanished_skills_only() {
    use chord::core::operations::install::prune_orphan_skills;
    use std::collections::HashSet;

    let proj = TempDir::new().unwrap();

    // Create store dirs + symlinks for both "foo" and "bar".
    for s in ["foo", "bar"] {
        let store = proj.path().join(".chord/o/r").join(s);
        std::fs::create_dir_all(&store).unwrap();
        std::fs::write(store.join("SKILL.md"), "x").unwrap();
        let linkdir = proj.path().join(".claude/skills");
        std::fs::create_dir_all(&linkdir).unwrap();
        std::os::unix::fs::symlink(format!("../../.chord/o/r/{s}"), linkdir.join(s)).unwrap();
    }

    // "foo" stays; "bar" vanished from upstream.
    let mut keep = HashSet::new();
    keep.insert("foo".to_string());
    prune_orphan_skills(
        proj.path(),
        "o/r",
        &["foo".to_string(), "bar".to_string()],
        &keep,
    );

    assert!(
        proj.path().join(".chord/o/r/foo").exists(),
        "kept skill store survives"
    );
    assert!(
        proj.path().join(".claude/skills/foo").exists(),
        "kept skill symlink survives"
    );
    assert!(
        !proj.path().join(".chord/o/r/bar").exists(),
        "vanished skill store removed"
    );
    assert!(
        proj.path()
            .join(".claude/skills/bar")
            .symlink_metadata()
            .is_err(),
        "vanished skill symlink removed"
    );
}
