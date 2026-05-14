use chord::operations::add::{AddSpec, Section};

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
