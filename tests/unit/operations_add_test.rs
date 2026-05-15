use chord::operations::add::{AddSpec, Section};
use chord::operations::{add, OpContext, OperationError};
use chord::store::{ConfigStore, InMemoryConfigStore, InMemoryLockfileStore, LockfileStore};
use std::fs;
use tempfile::TempDir;

fn make_ctx<'a>(
    project: &'a TempDir,
    home: &'a TempDir,
    packages: &'a TempDir,
    config_store: &'a dyn ConfigStore,
    lockfile_store: &'a dyn LockfileStore,
) -> OpContext<'a> {
    OpContext {
        config_store,
        lockfile_store,
        project_root: project.path(),
        home_dir: home.path(),
        packages_dir: packages.path(),
        verbose: false,
    }
}

#[test]
fn parses_mcp_with_explicit_version() {
    let spec = AddSpec::parse("mcp:context7@latest").unwrap();
    assert_eq!(spec.section, Section::Mcp);
    assert_eq!(spec.name, "context7");
    assert_eq!(spec.version, "latest");
}

#[test]
fn parses_cli_with_default_version() {
    let spec = AddSpec::parse("cli:foo").unwrap();
    assert_eq!(spec.section, Section::Cli);
    assert_eq!(spec.name, "foo");
    assert_eq!(spec.version, "latest");
}

#[test]
fn plugin_with_marketplace_uses_last_at_for_version() {
    let spec = AddSpec::parse("plugins:owner/repo/plugin@marketplace@latest").unwrap();
    assert_eq!(spec.section, Section::Plugins);
    assert_eq!(spec.name, "owner/repo/plugin@marketplace");
    assert_eq!(spec.version, "latest");
}

#[test]
fn plugin_without_version_keeps_marketplace_in_name() {
    let spec = AddSpec::parse("plugins:owner/repo/plugin@marketplace").unwrap();
    assert_eq!(spec.section, Section::Plugins);
    assert_eq!(spec.name, "owner/repo/plugin@marketplace");
    assert_eq!(spec.version, "latest");
}

#[test]
fn skill_with_slashes_in_name() {
    let spec = AddSpec::parse("skills:vercel-labs/next-skills/next-best-practices").unwrap();
    assert_eq!(spec.section, Section::Skills);
    assert_eq!(spec.name, "vercel-labs/next-skills/next-best-practices");
    assert_eq!(spec.version, "latest");
}

#[test]
fn rejects_unknown_section() {
    assert!(AddSpec::parse("bogus:foo").is_err());
}

#[test]
fn rejects_empty_name() {
    assert!(AddSpec::parse("mcp:").is_err());
}

#[test]
fn rejects_missing_section() {
    assert!(AddSpec::parse("foo").is_err());
    assert!(AddSpec::parse(":foo@1").is_err());
}

#[test]
fn rejects_empty_input() {
    assert!(AddSpec::parse("").is_err());
}

#[test]
fn rejects_trailing_at() {
    assert!(AddSpec::parse("mcp:foo@").is_err());
}

#[test]
fn add_writes_entry_to_empty_chord_toml() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    fs::write(project.path().join("chord.toml"), "").unwrap();

    let config_store = chord::store::FileConfigStore::new(project.path());
    let lockfile_store = chord::store::FileLockfileStore::new(project.path());
    let ctx = make_ctx(&project, &home, &packages, &config_store, &lockfile_store);
    let spec = AddSpec::parse("mcp:context7@latest").unwrap();

    // Test chord.toml mutation only. Calling add() would also invoke
    // install_one which needs network. Use add::write_toml_entry directly.
    add::write_toml_entry(&spec, &ctx).unwrap();

    let toml_content = fs::read_to_string(project.path().join("chord.toml")).unwrap();
    assert!(toml_content.contains("context7"), "got: {toml_content}");
    assert!(toml_content.contains("[mcp]"));
}

#[test]
fn add_rejects_duplicate_in_same_section() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    fs::write(
        project.path().join("chord.toml"),
        "[mcp]\ncontext7 = \"latest\"\n",
    )
    .unwrap();

    let config_store = chord::store::FileConfigStore::new(project.path());
    let lockfile_store = chord::store::FileLockfileStore::new(project.path());
    let ctx = make_ctx(&project, &home, &packages, &config_store, &lockfile_store);
    let spec = AddSpec::parse("mcp:context7@1.0.0").unwrap();
    let err = add::write_toml_entry(&spec, &ctx).unwrap_err();
    assert!(matches!(err, OperationError::Duplicate(_)));
}

#[test]
fn add_rejects_duplicate_across_sections() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    fs::write(
        project.path().join("chord.toml"),
        "[cli]\nfoo = \"latest\"\n",
    )
    .unwrap();

    let config_store = chord::store::FileConfigStore::new(project.path());
    let lockfile_store = chord::store::FileLockfileStore::new(project.path());
    let ctx = make_ctx(&project, &home, &packages, &config_store, &lockfile_store);
    let spec = AddSpec::parse("mcp:foo@latest").unwrap();
    let err = add::write_toml_entry(&spec, &ctx).unwrap_err();
    assert!(matches!(err, OperationError::Duplicate(_)));
}
