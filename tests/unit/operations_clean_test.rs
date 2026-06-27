use chord::core::installer::{DefaultInstallers, InstallerSet};
use chord::core::lockfile::{LockedSkill, LockedTool, Lockfile};
use chord::core::operations::clean::{clean, plugin_uninstall_argv, CleanOutcome};
use chord::core::operations::OpContext;
use chord::core::store::{InMemoryConfigStore, InMemoryLockfileStore};
use std::fs;
use std::path::Path;
use tempfile::TempDir;

fn seeded_lock() -> Lockfile {
    let mut lf = Lockfile::new();
    // wildcard anchor row -> flat skills foo, bar
    lf.set(
        "skills",
        "o/r",
        LockedTool {
            package: None,
            version: "sha".into(),
            integrity: None,
            resolved_at: None,
            skills: Some(vec![
                LockedSkill {
                    name: "foo".into(),
                    integrity: "sha256:1".into(),
                },
                LockedSkill {
                    name: "bar".into(),
                    integrity: "sha256:2".into(),
                },
            ]),
        },
    );
    // named skill row (skills: None) — exercises leaf-segment key path
    lf.set(
        "skills",
        "o/r/solo",
        LockedTool {
            package: None,
            version: "sha".into(),
            integrity: Some("sha256:9".into()),
            resolved_at: None,
            skills: None,
        },
    );
    lf.set(
        "cli",
        "mytool",
        LockedTool {
            package: Some("mytool".into()),
            version: "1.0.0".into(),
            integrity: None,
            resolved_at: None,
            skills: None,
        },
    );
    lf
}

fn make_ctx<'a>(
    project: &'a TempDir,
    home: &'a TempDir,
    packages: &'a TempDir,
    config_store: &'a InMemoryConfigStore,
    lockfile_store: &'a InMemoryLockfileStore,
    installers: &'a InstallerSet<'a>,
) -> OpContext<'a> {
    OpContext {
        config_store,
        lockfile_store,
        installers,
        project_root: project.path(),
        home_dir: home.path(),
        packages_dir: packages.path(),
        verbose: false,
    }
}

fn write(p: &Path, c: &str) {
    fs::create_dir_all(p.parent().unwrap()).unwrap();
    fs::write(p, c).unwrap();
}

#[test]
fn plugin_uninstall_argv_extracts_plugin_at_marketplace() {
    let argv = plugin_uninstall_argv("anthropics/repo/code-review@official");
    assert_eq!(
        argv,
        vec![
            "plugin",
            "uninstall",
            "code-review@official",
            "--scope",
            "project"
        ]
    );
}

#[test]
fn clean_default_removes_chord_owned_only() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    let root = project.path();

    // chord-owned skill store + symlink for foo
    write(&root.join(".chord/o/r/foo/SKILL.md"), "x");
    fs::create_dir_all(root.join(".claude/skills")).unwrap();
    std::os::unix::fs::symlink("../../.chord/o/r/foo", root.join(".claude/skills/foo")).unwrap();
    // named skill (skills: None) — leaf-segment branch
    write(&root.join(".chord/o/r/solo/SKILL.md"), "x");
    std::os::unix::fs::symlink("../../.chord/o/r/solo", root.join(".claude/skills/solo")).unwrap();
    // foreign symlink (npx) for grill — must be LEFT
    fs::create_dir_all(root.join(".agents/skills/grill")).unwrap();
    std::os::unix::fs::symlink(
        "../../.agents/skills/grill",
        root.join(".claude/skills/grill"),
    )
    .unwrap();
    // cli package dir
    fs::create_dir_all(packages.path().join("mytool/node_modules")).unwrap();

    let cfg = InMemoryConfigStore::empty();
    let lock = InMemoryLockfileStore::new(seeded_lock());
    let installers = DefaultInstallers::new();
    let set = installers.as_set();
    let ctx = make_ctx(&project, &home, &packages, &cfg, &lock, &set);

    let out = clean(&ctx, false).unwrap();

    assert!(!root.join(".chord").exists(), ".chord removed");
    assert!(
        root.join(".claude/skills/foo").symlink_metadata().is_err(),
        "chord-owned symlink removed"
    );
    assert!(
        root.join(".claude/skills/solo").symlink_metadata().is_err(),
        "named-skill symlink removed"
    );
    assert!(
        root.join(".claude/skills/grill").symlink_metadata().is_ok(),
        "foreign symlink kept"
    );
    assert!(
        root.join(".agents").exists(),
        ".agents kept in default mode"
    );
    assert!(
        !packages.path().join("mytool").exists(),
        "cli package dir removed"
    );
    assert_eq!(out.skills, 2);
    assert_eq!(out.cli, 1);
    assert_eq!(out.extra_removed, 0);
}

#[test]
fn clean_all_wipes_foreign_and_config() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    let root = project.path();

    write(&root.join(".chord/o/r/foo/SKILL.md"), "x");
    fs::create_dir_all(root.join(".claude/skills")).unwrap();
    std::os::unix::fs::symlink(
        "../../.agents/skills/grill",
        root.join(".claude/skills/grill"),
    )
    .unwrap();
    fs::create_dir_all(root.join(".agents/skills/grill")).unwrap();
    write(&root.join("skills-lock.json"), "{}");
    write(&root.join(".mcp.json"), "{}");
    write(&root.join(".claude/settings.json"), "{}");

    let cfg = InMemoryConfigStore::empty();
    let lock = InMemoryLockfileStore::new(seeded_lock());
    let installers = DefaultInstallers::new();
    let set = installers.as_set();
    let ctx = make_ctx(&project, &home, &packages, &cfg, &lock, &set);

    let out = clean(&ctx, true).unwrap();

    assert!(
        !root.join(".claude/skills").exists(),
        "whole .claude/skills wiped"
    );
    assert!(!root.join(".agents").exists(), ".agents wiped");
    assert!(!root.join("skills-lock.json").exists());
    assert!(!root.join(".mcp.json").exists());
    assert!(!root.join(".claude/settings.json").exists());
    assert_eq!(out.extra_removed, 5);
}

#[test]
fn clean_is_idempotent_on_empty() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    let cfg = InMemoryConfigStore::empty();
    let lock = InMemoryLockfileStore::empty();
    let installers = DefaultInstallers::new();
    let set = installers.as_set();
    let ctx = make_ctx(&project, &home, &packages, &cfg, &lock, &set);
    let out = clean(&ctx, false).unwrap();
    assert_eq!(out, CleanOutcome::default());
}
