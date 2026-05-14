use chord::inspect::{AuditEntry, AuditReport, Category, Management, Scope};
use chord::tui::tree::{build_tree, NodeKind};

fn make_entry(name: &str, scope: Option<Scope>, enabled: bool, path: Option<&str>) -> AuditEntry {
    AuditEntry {
        name: name.to_string(),
        version: None,
        scope,
        management: Management::Managed,
        path: path.map(|p| p.to_string()),
        drift: false,
        overridden_by: None,
        enabled,
        from_plugin: None,
    }
}

fn make_plugin_child(name: &str, plugin_id: &str, path: Option<&str>) -> AuditEntry {
    AuditEntry {
        name: name.to_string(),
        version: None,
        scope: Some(Scope::Global),
        management: Management::Managed,
        path: path.map(|p| p.to_string()),
        drift: false,
        overridden_by: None,
        enabled: true,
        from_plugin: Some(plugin_id.to_string()),
    }
}

// ── build_tree_groups_by_plugin ───────────────────────────────────────────────

#[test]
fn build_tree_groups_by_plugin() {
    let plugin_entry = make_entry("my-plugin", Some(Scope::Project), true, None);
    let skill_entry = make_plugin_child("my-skill", "my-plugin", Some("/path/to/my-skill.md"));

    let report = AuditReport {
        entries: vec![
            (Category::Plugins, vec![plugin_entry]),
            (Category::Skills, vec![skill_entry]),
        ],
    };

    let tree = build_tree(&report);

    // There should be a Plugins section and a MCP section (even if empty).
    let plugins_section = tree.iter().find(|n| n.name.starts_with("Plugins")).unwrap();
    assert_eq!(plugins_section.kind, NodeKind::SectionHeader);

    // The plugin node itself
    let plugin_node = plugins_section
        .children
        .iter()
        .find(|n| n.name == "my-plugin")
        .expect("plugin node should exist");
    assert_eq!(plugin_node.kind, NodeKind::Plugin);

    // The skill should be under a "Skills (1)" sub-header inside the plugin
    let skills_header = plugin_node
        .children
        .iter()
        .find(|n| n.name.starts_with("Skills"))
        .expect("Skills sub-header should exist in plugin");
    assert_eq!(skills_header.kind, NodeKind::SectionHeader);
    let skill_node = skills_header
        .children
        .iter()
        .find(|n| n.name == "my-skill")
        .expect("skill should be under Skills sub-header");
    assert_eq!(skill_node.kind, NodeKind::Skill);

    // Regression: plugin-cache items must keep their real markdown file path,
    // not be overwritten with a "plugin <id>" sentinel.
    assert_eq!(
        skill_node.path.as_deref(),
        Some("/path/to/my-skill.md"),
        "skill node should keep its real markdown file path"
    );

    // No standalone Skills section
    assert!(
        tree.iter().all(|n| n.name != "Skills"),
        "standalone Skills section should not exist"
    );
}

// ── build_tree_standalone_items ───────────────────────────────────────────────

#[test]
fn build_tree_standalone_items() {
    let skill_entry = make_entry("standalone-skill", Some(Scope::Global), true, None);
    let cmd_entry = make_entry("standalone-cmd", Some(Scope::Project), true, None);

    let report = AuditReport {
        entries: vec![
            (Category::Plugins, vec![]),
            (Category::Skills, vec![skill_entry]),
            (Category::Commands, vec![cmd_entry]),
        ],
    };

    let tree = build_tree(&report);

    let skills_section = tree
        .iter()
        .find(|n| n.name == "Skills")
        .expect("Skills standalone section should exist");
    assert_eq!(skills_section.kind, NodeKind::SectionHeader);
    assert!(
        skills_section
            .children
            .iter()
            .any(|n| n.name == "standalone-skill"),
        "standalone-skill should be in Skills section"
    );

    let cmds_section = tree
        .iter()
        .find(|n| n.name == "Commands")
        .expect("Commands standalone section should exist");
    assert!(
        cmds_section
            .children
            .iter()
            .any(|n| n.name == "standalone-cmd"),
        "standalone-cmd should be in Commands section"
    );
}

// ── build_tree_mcp_section ────────────────────────────────────────────────────

#[test]
fn build_tree_mcp_section() {
    let mcp1 = make_entry("context7", Some(Scope::Project), true, None);
    let mcp2 = make_entry("filesystem", Some(Scope::Global), false, None);

    let report = AuditReport {
        entries: vec![(Category::Mcp, vec![mcp1, mcp2])],
    };

    let tree = build_tree(&report);

    let mcp_section = tree
        .iter()
        .find(|n| n.name.starts_with("MCP Servers"))
        .expect("MCP Servers section should exist");
    assert_eq!(mcp_section.kind, NodeKind::SectionHeader);
    assert_eq!(mcp_section.children.len(), 2);

    let context7 = mcp_section
        .children
        .iter()
        .find(|n| n.name == "context7")
        .unwrap();
    assert_eq!(context7.kind, NodeKind::McpServer);
    assert!(context7.enabled);

    let fs = mcp_section
        .children
        .iter()
        .find(|n| n.name == "filesystem")
        .unwrap();
    assert!(!fs.enabled);
}
