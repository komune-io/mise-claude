use chord::config::Config;
use chord::operations::add::{AddSpec, Section};
use chord::operations::{add, OpContext, OperationError};
use chord::store::{ConfigStore, InMemoryConfigStore, InMemoryLockfileStore, LockfileStore};
use std::path::Path;

/// Build a minimal OpContext backed by in-memory stores. No TempDir,
/// no filesystem — the path fields are inert placeholders since none of
/// the Op functions under test reach for them.
fn ctx_with<'a>(
    config_store: &'a dyn ConfigStore,
    lockfile_store: &'a dyn LockfileStore,
) -> OpContext<'a> {
    OpContext {
        config_store,
        lockfile_store,
        project_root: Path::new("."),
        home_dir: Path::new("."),
        packages_dir: Path::new("."),
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
    let config_store = InMemoryConfigStore::empty();
    let lockfile_store = InMemoryLockfileStore::empty();
    let ctx = ctx_with(&config_store, &lockfile_store);
    let spec = AddSpec::parse("mcp:context7@latest").unwrap();

    add::write_toml_entry(&spec, &ctx).unwrap();

    let stored = config_store.load().unwrap();
    assert_eq!(
        stored.mcp.get("context7").map(String::as_str),
        Some("latest")
    );
}

#[test]
fn add_rejects_duplicate_in_same_section() {
    let mut seeded = Config::default();
    seeded
        .mcp
        .insert("context7".to_string(), "latest".to_string());
    let config_store = InMemoryConfigStore::new(seeded.clone());
    let lockfile_store = InMemoryLockfileStore::empty();
    let ctx = ctx_with(&config_store, &lockfile_store);
    let spec = AddSpec::parse("mcp:context7@1.0.0").unwrap();

    let err = add::write_toml_entry(&spec, &ctx).unwrap_err();
    assert!(matches!(err, OperationError::Duplicate(_)));

    // chord.toml is left untouched.
    assert_eq!(config_store.load().unwrap(), seeded);
}

#[test]
fn add_rejects_duplicate_across_sections() {
    let mut seeded = Config::default();
    seeded.cli.insert("foo".to_string(), "latest".to_string());
    let config_store = InMemoryConfigStore::new(seeded);
    let lockfile_store = InMemoryLockfileStore::empty();
    let ctx = ctx_with(&config_store, &lockfile_store);
    let spec = AddSpec::parse("mcp:foo@latest").unwrap();

    let err = add::write_toml_entry(&spec, &ctx).unwrap_err();
    assert!(matches!(err, OperationError::Duplicate(_)));
}
