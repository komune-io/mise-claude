use claude_env::config::Config;
use claude_env::inspect::reconciler::reconcile;
use claude_env::inspect::{Category, DiscoveredItem, Management, Scope};

fn make_item(name: &str, scope: Scope) -> DiscoveredItem {
    DiscoveredItem {
        name: name.to_string(),
        version: Some("1.0.0".to_string()),
        scope,
        source_path: "/some/path".to_string(),
    }
}

fn config_with_mcp(key: &str) -> Config {
    Config::parse(&format!(
        "[mcp]\n\"{}\" = \"latest\"\n",
        key
    ))
    .unwrap()
}

// ----- MCP tests -----

#[test]
fn managed_item_matched_in_config() {
    // Config declares "context7" (friendly alias → @upstash/context7-mcp → bare "context7-mcp").
    // Discovered item has name "context7-mcp" (the bare package name).
    let config = config_with_mcp("context7");
    let discovered = vec![make_item("context7-mcp", Scope::Project)];

    let entries = reconcile(Category::Mcp, &discovered, &config);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "context7-mcp");
    assert_eq!(entries[0].management, Management::Managed);
    assert!(!entries[0].drift);
}

#[test]
fn manual_item_not_in_config() {
    // Empty config → everything is Manual.
    let config = Config::default();
    let discovered = vec![make_item("some-tool", Scope::Global)];

    let entries = reconcile(Category::Mcp, &discovered, &config);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].management, Management::Manual);
    assert!(!entries[0].drift);
}

#[test]
fn drift_declared_but_not_discovered() {
    // Config declares "shadcn" but nothing is discovered → drift entry emitted.
    let config = config_with_mcp("shadcn");
    let discovered = vec![];

    let entries = reconcile(Category::Mcp, &discovered, &config);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "shadcn");
    assert!(entries[0].drift, "Expected drift=true for unmatched config entry");
    assert!(entries[0].scope.is_none());
    assert!(entries[0].path.is_none());
}

#[test]
fn override_detected_same_name_both_scopes() {
    // Same MCP name installed at both Project and Global scope.
    // Global entry should receive overridden_by = Some("project").
    let config = Config::default();
    let discovered = vec![
        make_item("my-mcp", Scope::Project),
        make_item("my-mcp", Scope::Global),
    ];

    let entries = reconcile(Category::Mcp, &discovered, &config);

    assert_eq!(entries.len(), 2);

    let project_entry = entries.iter().find(|e| e.scope == Some(Scope::Project)).unwrap();
    assert!(project_entry.overridden_by.is_none());

    let global_entry = entries.iter().find(|e| e.scope == Some(Scope::Global)).unwrap();
    assert_eq!(
        global_entry.overridden_by.as_deref(),
        Some("project"),
        "Global entry should be marked as overridden by project"
    );
}

// ----- Plugin tests -----

#[test]
fn plugin_reconciliation() {
    // Config key is the full path; discovered uses only the short form.
    let config = Config::parse(
        "[plugins]\n\"anthropics/claude-code/code-review@claude-code-plugins\" = \"latest\"\n",
    )
    .unwrap();
    // Short form = last '/' segment = "code-review@claude-code-plugins"
    let discovered = vec![make_item("code-review@claude-code-plugins", Scope::Project)];

    let entries = reconcile(Category::Plugins, &discovered, &config);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].management, Management::Managed);
    assert!(!entries[0].drift);
}

// ----- Skills tests -----

#[test]
fn skills_reconciliation() {
    // Config key is full "owner/repo/skill"; discovered has only the leaf "skill".
    let config = Config::parse(
        "[skills]\n\"vercel-labs/next-skills/next-best-practices\" = \"latest\"\n",
    )
    .unwrap();
    let discovered = vec![make_item("next-best-practices", Scope::Project)];

    let entries = reconcile(Category::Skills, &discovered, &config);

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].management, Management::Managed);
    assert!(!entries[0].drift);
}
