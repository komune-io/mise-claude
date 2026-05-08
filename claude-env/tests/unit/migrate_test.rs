use claude_env::migrate::migrate;
use std::fs;
use tempfile::TempDir;

#[test]
fn migrate_mcp_tool() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:mcp/context7\" = \"2.1.4\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(config.mcp.get("context7").map(String::as_str), Some("2.1.4"));
    assert!(config.skills.is_empty());
    assert!(config.plugins.is_empty());
    assert!(config.cli.is_empty());
}

#[test]
fn migrate_skills_sh_tool() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:skills.sh/vercel-labs/next-skills/next-best-practices\" = \"latest\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(
        config.skills.get("vercel-labs/next-skills/next-best-practices").map(String::as_str),
        Some("latest"),
    );
}

#[test]
fn migrate_plugin_tool() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:plugin/upstash/context7/context7-plugin@context7-marketplace\" = \"latest\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(
        config.plugins.get("upstash/context7/context7-plugin@context7-marketplace").map(String::as_str),
        Some("latest"),
    );
}

#[test]
fn migrate_spec_tool_goes_to_cli_section() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:spec/gsd\" = \"1.22.4\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(config.cli.get("gsd").map(String::as_str), Some("1.22.4"));
}

#[test]
fn migrate_cli_tool_goes_to_cli_section() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:cli/my-tool\" = \"3.0.0\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(config.cli.get("my-tool").map(String::as_str), Some("3.0.0"));
}

#[test]
fn migrate_ignores_non_claude_tools() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\nnode = \"22\"\n\"claude:mcp/context7\" = \"2.1.4\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(config.mcp.len(), 1);
    assert_eq!(config.mcp.get("context7").map(String::as_str), Some("2.1.4"));
}

#[test]
fn migrate_no_claude_tools_returns_error() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".mise.toml"), "[tools]\nnode = \"22\"\n").unwrap();
    assert!(migrate(dir.path()).is_err());
}

#[test]
fn migrate_missing_mise_toml_returns_error() {
    let dir = TempDir::new().unwrap();
    assert!(migrate(dir.path()).is_err());
}

#[test]
fn migrate_multiple_sections() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:mcp/context7\" = \"2.1.4\"\n\"claude:skills.sh/vercel-labs/next-skills/next\" = \"latest\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(config.mcp.len(), 1);
    assert_eq!(config.skills.len(), 1);
}
