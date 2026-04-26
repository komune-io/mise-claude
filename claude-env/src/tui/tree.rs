use std::collections::BTreeMap;

use crate::inspect::{AuditEntry, AuditReport, Category, Scope};

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    SectionHeader,
    Plugin,
    Skill,
    Command,
    Agent,
    McpServer,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub kind: NodeKind,
    pub enabled: bool,
    pub scope: Option<crate::inspect::Scope>,
    pub path: Option<String>,
    pub plugin_id: Option<String>,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    pub hidden: bool,
}

impl TreeNode {
    /// Create a section header node (expanded, visible).
    pub fn section(name: &str) -> Self {
        TreeNode {
            name: name.to_string(),
            kind: NodeKind::SectionHeader,
            enabled: true,
            scope: None,
            path: None,
            plugin_id: None,
            children: Vec::new(),
            expanded: true,
            hidden: false,
        }
    }

    /// Create a leaf node from an AuditEntry.
    pub fn leaf(name: &str, kind: NodeKind, entry: &AuditEntry) -> Self {
        TreeNode {
            name: name.to_string(),
            kind,
            enabled: entry.enabled,
            scope: entry.scope.clone(),
            path: entry.path.clone(),
            plugin_id: None,
            children: Vec::new(),
            expanded: false,
            hidden: false,
        }
    }

    /// Create a plugin node.
    pub fn plugin(
        name: &str,
        enabled: bool,
        scope: Option<Scope>,
        path: Option<String>,
    ) -> Self {
        TreeNode {
            name: name.to_string(),
            kind: NodeKind::Plugin,
            enabled,
            scope,
            path,
            plugin_id: Some(name.to_string()),
            children: Vec::new(),
            expanded: true,
            hidden: false,
        }
    }
}

/// If `path` starts with "plugin ", return the remainder (the plugin id).
fn extract_plugin_from_path(path: Option<&str>) -> Option<String> {
    path?.strip_prefix("plugin ")
        .map(|rest| rest.to_string())
}

/// Build the TUI tree from an AuditReport.
///
/// Layout:
/// - Plugins section (with child-count in name), each plugin may have child leaves.
/// - MCP Servers section (flat list).
/// - Standalone sections for Skills / Commands / Agents not owned by any plugin.
pub fn build_tree(report: &AuditReport) -> Vec<TreeNode> {
    // ── Pass 1: build plugin map ──────────────────────────────────────────
    let mut plugins: BTreeMap<String, TreeNode> = BTreeMap::new();

    for (category, entries) in &report.entries {
        if *category == Category::Plugins {
            for entry in entries {
                let node = TreeNode::plugin(
                    &entry.name,
                    entry.enabled,
                    entry.scope.clone(),
                    entry.path.clone(),
                );
                plugins.insert(entry.name.clone(), node);
            }
        }
    }

    // ── Pass 2: route Skills / Commands / Agents ──────────────────────────
    // Items whose path starts with "plugin <id>" become children of that plugin.
    // The rest go into per-category standalone sections.
    let mut standalone: BTreeMap<&'static str, Vec<TreeNode>> = BTreeMap::new();
    let mut mcp_nodes: Vec<TreeNode> = Vec::new();

    for (category, entries) in &report.entries {
        let kind = match category {
            Category::Skills => NodeKind::Skill,
            Category::Commands => NodeKind::Command,
            Category::Agents => NodeKind::Agent,
            Category::Mcp => NodeKind::McpServer,
            Category::Plugins => continue, // already handled
        };

        for entry in entries {
            if *category == Category::Mcp {
                mcp_nodes.push(TreeNode::leaf(&entry.name, NodeKind::McpServer, entry));
                continue;
            }

            let leaf = TreeNode::leaf(&entry.name, kind.clone(), entry);

            if let Some(plugin_id) = extract_plugin_from_path(entry.path.as_deref()) {
                if let Some(plugin_node) = plugins.get_mut(&plugin_id) {
                    plugin_node.children.push(leaf);
                    continue;
                }
            }

            // Standalone
            let label: &'static str = match category {
                Category::Skills => "Skills",
                Category::Commands => "Commands",
                Category::Agents => "Agents",
                _ => unreachable!(),
            };
            standalone.entry(label).or_default().push(leaf);
        }
    }

    // ── Build final tree ──────────────────────────────────────────────────
    let mut tree: Vec<TreeNode> = Vec::new();

    // Plugins section
    let plugin_list: Vec<TreeNode> = plugins.into_values().collect();
    let plugin_count = plugin_list.len();
    let mut plugins_section = TreeNode::section(&format!("Plugins ({})", plugin_count));
    plugins_section.children = plugin_list;
    tree.push(plugins_section);

    // MCP section
    let mcp_count = mcp_nodes.len();
    let mut mcp_section = TreeNode::section(&format!("MCP Servers ({})", mcp_count));
    mcp_section.children = mcp_nodes;
    tree.push(mcp_section);

    // Standalone sections (only when non-empty), in a stable order
    for label in &["Skills", "Commands", "Agents"] {
        if let Some(nodes) = standalone.get(label) {
            if !nodes.is_empty() {
                let mut section = TreeNode::section(label);
                section.children = nodes.clone();
                tree.push(section);
            }
        }
    }

    tree
}
