use claude_env::inspect::scanner;
use claude_env::inspect::Scope;
use std::fs;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn make_dirs() -> (TempDir, TempDir) {
    (TempDir::new().unwrap(), TempDir::new().unwrap())
}

fn write_file(path: &std::path::Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

// ---------------------------------------------------------------------------
// scan_mcp
// ---------------------------------------------------------------------------

#[test]
fn scan_mcp_from_project_mcp_json() {
    let (proj, home) = make_dirs();
    write_file(
        &proj.path().join(".mcp.json"),
        r#"{"mcpServers": {"context7": {"command": "npx"}}}"#,
    );

    let items = scanner::scan_mcp(proj.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "context7");
    assert_eq!(items[0].scope, Scope::Project);
    assert!(items[0].source_path.ends_with(".mcp.json"));
}

#[test]
fn scan_mcp_from_global_settings() {
    let (proj, home) = make_dirs();
    write_file(
        &home.path().join(".claude").join("settings.json"),
        r#"{"mcpServers": {"memory": {"command": "npx"}}}"#,
    );

    let items = scanner::scan_mcp(proj.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "memory");
    assert_eq!(items[0].scope, Scope::Global);
    assert!(items[0].source_path.ends_with("settings.json"));
}

#[test]
fn scan_mcp_both_scopes() {
    let (proj, home) = make_dirs();
    // Project has "context7" and "memory"
    write_file(
        &proj.path().join(".mcp.json"),
        r#"{"mcpServers": {"context7": {}, "memory": {}}}"#,
    );
    // Global has "memory" (shared) — appears as separate Global item
    write_file(
        &home.path().join(".claude").join("settings.json"),
        r#"{"mcpServers": {"memory": {}}}"#,
    );

    let items = scanner::scan_mcp(proj.path(), home.path());
    // 2 project + 1 global = 3 total
    assert_eq!(items.len(), 3);

    let project_items: Vec<_> = items.iter().filter(|i| i.scope == Scope::Project).collect();
    let global_items: Vec<_> = items.iter().filter(|i| i.scope == Scope::Global).collect();
    assert_eq!(project_items.len(), 2);
    assert_eq!(global_items.len(), 1);
    assert_eq!(global_items[0].name, "memory");
}

#[test]
fn scan_mcp_missing_files() {
    let (proj, home) = make_dirs();
    let items = scanner::scan_mcp(proj.path(), home.path());
    assert!(items.is_empty());
}

#[test]
fn scan_mcp_invalid_json_skipped() {
    let (proj, home) = make_dirs();
    write_file(&proj.path().join(".mcp.json"), "not valid json {{");

    let items = scanner::scan_mcp(proj.path(), home.path());
    assert!(items.is_empty());
}

#[test]
fn scan_mcp_missing_mcp_servers_key() {
    let (proj, home) = make_dirs();
    write_file(&proj.path().join(".mcp.json"), r#"{"otherKey": {}}"#);

    let items = scanner::scan_mcp(proj.path(), home.path());
    assert!(items.is_empty());
}

// ---------------------------------------------------------------------------
// scan_plugins
// ---------------------------------------------------------------------------

#[test]
fn scan_plugins_from_project_settings() {
    let (proj, home) = make_dirs();
    write_file(
        &proj.path().join(".claude").join("settings.json"),
        r#"{"enabledPlugins": {"my-plugin": {}}}"#,
    );

    let items = scanner::scan_plugins(proj.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "my-plugin");
    assert_eq!(items[0].scope, Scope::Project);
    assert!(items[0].source_path.ends_with("settings.json"));
}

#[test]
fn scan_plugins_from_global_settings() {
    let (proj, home) = make_dirs();
    write_file(
        &home.path().join(".claude").join("settings.json"),
        r#"{"enabledPlugins": {"global-plugin": {}}}"#,
    );

    let items = scanner::scan_plugins(proj.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "global-plugin");
    assert_eq!(items[0].scope, Scope::Global);
}

#[test]
fn scan_plugins_missing_files() {
    let (proj, home) = make_dirs();
    let items = scanner::scan_plugins(proj.path(), home.path());
    assert!(items.is_empty());
}

// ---------------------------------------------------------------------------
// scan_skills
// ---------------------------------------------------------------------------

#[test]
fn scan_skills_from_project() {
    let (proj, home) = make_dirs();
    write_file(
        &proj
            .path()
            .join(".claude")
            .join("skills")
            .join("my-skill")
            .join("SKILL.md"),
        "# My Skill",
    );

    let items = scanner::scan_skills(proj.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "my-skill");
    assert_eq!(items[0].scope, Scope::Project);
    assert!(items[0].source_path.ends_with("SKILL.md"));
}

#[test]
fn scan_skills_ignores_dirs_without_marker() {
    let (proj, home) = make_dirs();
    // Create a directory under skills/ but no SKILL.md inside it
    let skill_dir = proj.path().join(".claude").join("skills").join("orphan");
    fs::create_dir_all(&skill_dir).unwrap();
    // Write a different file — not SKILL.md
    write_file(&skill_dir.join("README.md"), "nope");

    let items = scanner::scan_skills(proj.path(), home.path());
    assert!(items.is_empty());
}

#[test]
fn scan_skills_both_scopes() {
    let (proj, home) = make_dirs();
    write_file(
        &proj
            .path()
            .join(".claude")
            .join("skills")
            .join("local-skill")
            .join("SKILL.md"),
        "# Local",
    );
    write_file(
        &home
            .path()
            .join(".claude")
            .join("skills")
            .join("global-skill")
            .join("SKILL.md"),
        "# Global",
    );

    let items = scanner::scan_skills(proj.path(), home.path());
    assert_eq!(items.len(), 2);

    let project_items: Vec<_> = items.iter().filter(|i| i.scope == Scope::Project).collect();
    let global_items: Vec<_> = items.iter().filter(|i| i.scope == Scope::Global).collect();
    assert_eq!(project_items.len(), 1);
    assert_eq!(global_items.len(), 1);
    assert_eq!(project_items[0].name, "local-skill");
    assert_eq!(global_items[0].name, "global-skill");
}

// ---------------------------------------------------------------------------
// scan_commands
// ---------------------------------------------------------------------------

#[test]
fn scan_commands_recursive() {
    let (proj, home) = make_dirs();
    let commands_dir = proj.path().join(".claude").join("commands");

    // Top-level command
    write_file(&commands_dir.join("top-level.md"), "# Top");
    // Nested command
    write_file(&commands_dir.join("subdir").join("nested.md"), "# Nested");

    let items = scanner::scan_commands(proj.path(), home.path());
    assert_eq!(items.len(), 2);

    let names: Vec<&str> = items.iter().map(|i| i.name.as_str()).collect();
    assert!(names.contains(&"top-level"));
    // Nested item includes relative path
    assert!(names.iter().any(|n| n.contains("nested")));
    assert!(items.iter().all(|i| i.scope == Scope::Project));
}

#[test]
fn scan_commands_missing_dir() {
    let (proj, home) = make_dirs();
    let items = scanner::scan_commands(proj.path(), home.path());
    assert!(items.is_empty());
}

// ---------------------------------------------------------------------------
// scan_agents
// ---------------------------------------------------------------------------

#[test]
fn scan_agents_flat_only() {
    let (proj, home) = make_dirs();
    let agents_dir = proj.path().join(".claude").join("agents");

    // Top-level agent — should be found
    write_file(&agents_dir.join("my-agent.md"), "# Agent");
    // Nested file — should be ignored (flat scan)
    write_file(
        &agents_dir.join("subdir").join("nested-agent.md"),
        "# Nested",
    );

    let items = scanner::scan_agents(proj.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "my-agent");
    assert_eq!(items[0].scope, Scope::Project);
}

#[test]
fn scan_agents_global() {
    let (proj, home) = make_dirs();
    write_file(
        &home.path().join(".claude").join("agents").join("global-agent.md"),
        "# Global Agent",
    );

    let items = scanner::scan_agents(proj.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "global-agent");
    assert_eq!(items[0].scope, Scope::Global);
}

#[test]
fn scan_agents_missing_dir() {
    let (proj, home) = make_dirs();
    let items = scanner::scan_agents(proj.path(), home.path());
    assert!(items.is_empty());
}
