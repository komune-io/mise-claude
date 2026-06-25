use chord::core::config::Config;
use chord::core::inspect::reconciler::reconcile;
use chord::core::inspect::{Category, DiscoveredItem, Management, Scope};
use std::collections::HashSet;

fn make_item(name: &str, scope: Scope) -> DiscoveredItem {
    DiscoveredItem {
        name: name.to_string(),
        version: Some("1.0.0".to_string()),
        scope,
        source_path: "/some/path".to_string(),
        from_plugin: None,
        source_repo: None,
    }
}

fn no_plugins() -> HashSet<String> {
    HashSet::new()
}

fn config_with_mcp(key: &str) -> Config {
    Config::parse(&format!("[mcp]\n\"{}\" = \"latest\"\n", key)).unwrap()
}

// ----- MCP tests -----

#[test]
fn managed_item_matched_in_config() {
    // Config declares "context7" (friendly alias → @upstash/context7-mcp → bare "context7-mcp").
    // Discovered item has name "context7-mcp" (the bare package name).
    let config = config_with_mcp("context7");
    let discovered = vec![make_item("context7-mcp", Scope::Project)];

    let entries = reconcile(Category::Mcp, &discovered, &config, &no_plugins());

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

    let entries = reconcile(Category::Mcp, &discovered, &config, &no_plugins());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].management, Management::Manual);
    assert!(!entries[0].drift);
}

#[test]
fn drift_declared_but_not_discovered() {
    // Config declares "shadcn" but nothing is discovered → drift entry emitted.
    let config = config_with_mcp("shadcn");
    let discovered = vec![];

    let entries = reconcile(Category::Mcp, &discovered, &config, &no_plugins());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].name, "shadcn");
    assert!(
        entries[0].drift,
        "Expected drift=true for unmatched config entry"
    );
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

    let entries = reconcile(Category::Mcp, &discovered, &config, &no_plugins());

    assert_eq!(entries.len(), 2);

    let project_entry = entries
        .iter()
        .find(|e| e.scope == Some(Scope::Project))
        .unwrap();
    assert!(project_entry.overridden_by.is_none());

    let global_entry = entries
        .iter()
        .find(|e| e.scope == Some(Scope::Global))
        .unwrap();
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
        "[plugins]\n\"anthropics/claude-plugins-official/code-review@claude-plugins-official\" = \"latest\"\n",
    )
    .unwrap();
    // Short form = last '/' segment = "code-review@claude-plugins-official"
    let discovered = vec![make_item(
        "code-review@claude-plugins-official",
        Scope::Project,
    )];

    let entries = reconcile(Category::Plugins, &discovered, &config, &no_plugins());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].management, Management::Managed);
    assert!(!entries[0].drift);
}

// ----- Skills tests -----

#[test]
fn skills_reconciliation() {
    // Config key is full "owner/repo/skill"; discovered has only the leaf "skill".
    let config =
        Config::parse("[skills]\n\"vercel-labs/next-skills/next-best-practices\" = \"latest\"\n")
            .unwrap();
    let discovered = vec![make_item("next-best-practices", Scope::Project)];

    let entries = reconcile(Category::Skills, &discovered, &config, &no_plugins());

    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].management, Management::Managed);
    assert!(!entries[0].drift);
}

/// Helper: build a discovered skill item with the source_repo populated
/// (as the scanner does when it finds `skills-lock.json`).
fn sourced_item(name: &str, source_repo: &str) -> DiscoveredItem {
    let mut item = make_item(name, Scope::Project);
    item.source_repo = Some(source_repo.to_string());
    item
}

#[test]
fn wildcard_skill_entry_matches_by_source_repo() {
    // `"mattpocock/skills" = "latest"` is a 2-segment wildcard. Multiple
    // discovered items with source_repo == "mattpocock/skills" should all
    // be Managed; no Drift entry should be emitted for the wildcard key.
    let config = Config::parse("[skills]\n\"mattpocock/skills\" = \"latest\"\n").unwrap();
    let discovered = vec![
        sourced_item("caveman", "mattpocock/skills"),
        sourced_item("diagnose", "mattpocock/skills"),
    ];

    let entries = reconcile(Category::Skills, &discovered, &config, &no_plugins());

    assert_eq!(entries.len(), 2, "got entries: {:?}", entries);
    for e in &entries {
        assert_eq!(e.management, Management::Managed);
        assert!(!e.drift);
    }
}

#[test]
fn wildcard_skill_entry_drifts_when_no_source_matches() {
    // chord.toml declares `"mattpocock/skills"` but no discovered skill
    // has that source. The wildcard should emit a Drift entry.
    let config = Config::parse("[skills]\n\"mattpocock/skills\" = \"latest\"\n").unwrap();
    let discovered = vec![sourced_item("unrelated", "someone-else/repo")];

    let entries = reconcile(Category::Skills, &discovered, &config, &no_plugins());

    // One Manual entry for the unrelated item, one Drift entry for the wildcard.
    assert_eq!(entries.len(), 2);
    let drift_entries: Vec<_> = entries.iter().filter(|e| e.drift).collect();
    assert_eq!(drift_entries.len(), 1);
    assert_eq!(drift_entries[0].name, "mattpocock/skills");
}

#[test]
fn wildcard_skill_entry_ignores_items_without_source_repo() {
    // A skill with `source_repo: None` (no skills-lock.json mapping) should
    // not satisfy a wildcard match — chord can't prove ownership.
    let config = Config::parse("[skills]\n\"mattpocock/skills\" = \"latest\"\n").unwrap();
    let discovered = vec![make_item("orphan-skill", Scope::Project)];

    let entries = reconcile(Category::Skills, &discovered, &config, &no_plugins());

    assert_eq!(entries.len(), 2);
    let drift_entries: Vec<_> = entries.iter().filter(|e| e.drift).collect();
    assert_eq!(drift_entries.len(), 1);
}
