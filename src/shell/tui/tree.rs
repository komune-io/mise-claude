use std::collections::BTreeMap;

use crate::core::inspect::{AuditEntry, AuditReport, Category, Scope};

#[derive(Debug, Clone, PartialEq)]
pub enum NodeKind {
    SectionHeader,
    Plugin,
    Skill,
    Command,
    Agent,
    McpServer,
    Hook,
}

#[derive(Debug, Clone)]
pub struct TreeNode {
    pub name: String,
    pub kind: NodeKind,
    pub enabled: bool,
    pub scope: Option<crate::core::inspect::Scope>,
    pub path: Option<String>,
    pub plugin_id: Option<String>,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    pub hidden: bool,
    pub drift: bool,
    pub managed: bool,
    /// For skills installed via `npx skills add`: the upstream `owner/repo`.
    /// Drives source-repo grouping in `build_tree` so standalone skills from
    /// the same repo collapse under one sub-header.
    pub source_repo: Option<String>,
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
            drift: false,
            managed: false,
            source_repo: None,
        }
    }

    /// Create a leaf node from an AuditEntry.
    pub fn leaf(name: &str, kind: NodeKind, entry: &AuditEntry) -> Self {
        use crate::core::inspect::Management;
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
            drift: entry.drift,
            managed: entry.management == Management::Managed,
            source_repo: entry.source_repo.clone(),
        }
    }

    /// Create a plugin node.
    pub fn plugin(name: &str, enabled: bool, scope: Option<Scope>, path: Option<String>) -> Self {
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
            drift: false,
            managed: false,
            source_repo: None,
        }
    }
}

/// Reorganize a plugin node's flat children into type sub-groups:
/// Skills (N), Commands (N), Agents (N) — each as a sub-header with children.
fn organize_plugin_children(plugin: &mut TreeNode) {
    if plugin.children.is_empty() {
        return;
    }

    let mut skills = Vec::new();
    let mut commands = Vec::new();
    let mut agents = Vec::new();
    let mut other = Vec::new();

    for child in std::mem::take(&mut plugin.children) {
        match child.kind {
            NodeKind::Skill => skills.push(child),
            NodeKind::Command => commands.push(child),
            NodeKind::Agent => agents.push(child),
            _ => other.push(child),
        }
    }

    let enabled = plugin.enabled;

    if !skills.is_empty() {
        let mut header = TreeNode::section(&format!("Skills ({})", skills.len()));
        header.enabled = enabled;
        header.expanded = true;
        header.children = skills;
        plugin.children.push(header);
    }
    if !commands.is_empty() {
        let mut header = TreeNode::section(&format!("Commands ({})", commands.len()));
        header.enabled = enabled;
        header.expanded = true;
        header.children = commands;
        plugin.children.push(header);
    }
    if !agents.is_empty() {
        let mut header = TreeNode::section(&format!("Agents ({})", agents.len()));
        header.enabled = enabled;
        header.expanded = true;
        header.children = agents;
        plugin.children.push(header);
    }
    plugin.children.extend(other);
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
    // Items tagged with `from_plugin` become children of that plugin.
    // The rest go into per-category standalone sections.
    let mut standalone: BTreeMap<&'static str, Vec<TreeNode>> = BTreeMap::new();
    let mut mcp_nodes: Vec<TreeNode> = Vec::new();

    let mut hook_nodes: Vec<TreeNode> = Vec::new();

    for (category, entries) in &report.entries {
        let kind = match category {
            Category::Skills => NodeKind::Skill,
            Category::Commands => NodeKind::Command,
            Category::Agents => NodeKind::Agent,
            Category::Mcp => NodeKind::McpServer,
            Category::Hooks => NodeKind::Hook,
            Category::Plugins => continue, // already handled
        };

        for entry in entries {
            if *category == Category::Mcp {
                mcp_nodes.push(TreeNode::leaf(&entry.name, NodeKind::McpServer, entry));
                continue;
            }
            if *category == Category::Hooks {
                hook_nodes.push(TreeNode::leaf(&entry.name, NodeKind::Hook, entry));
                continue;
            }

            let leaf = TreeNode::leaf(&entry.name, kind.clone(), entry);

            if let Some(plugin_id) = entry.from_plugin.clone() {
                // Try exact match first, then fuzzy match on name (before @)
                let matched_key = if plugins.contains_key(&plugin_id) {
                    Some(plugin_id.clone())
                } else {
                    let cache_name = plugin_id.split('@').next().unwrap_or(&plugin_id);
                    plugins
                        .keys()
                        .find(|k| k.split('@').next().unwrap_or(k) == cache_name)
                        .cloned()
                };
                if let Some(key) = matched_key {
                    if let Some(plugin_node) = plugins.get_mut(&key) {
                        plugin_node.children.push(leaf);
                        continue;
                    }
                }
                // No matching plugin node yet — create one from cache info
                let node = plugins.entry(plugin_id.clone()).or_insert_with(|| {
                    TreeNode::plugin(
                        &plugin_id,
                        entry.enabled,
                        entry.scope.clone(),
                        Some(format!("plugin {}", plugin_id)),
                    )
                });
                node.children.push(leaf);
                continue;
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

    // Organize each plugin's children into type sub-groups
    let mut plugin_list: Vec<TreeNode> = plugins.into_values().collect();
    for plugin in &mut plugin_list {
        organize_plugin_children(plugin);
    }
    let plugin_count = plugin_list.len();
    let mut plugins_section = TreeNode::section(&format!("Plugins ({})", plugin_count));
    plugins_section.children = plugin_list;
    tree.push(plugins_section);

    // MCP section
    let mcp_count = mcp_nodes.len();
    let mut mcp_section = TreeNode::section(&format!("MCP Servers ({})", mcp_count));
    mcp_section.children = mcp_nodes;
    tree.push(mcp_section);

    // Hooks section — group by plugin, then by event
    // from_plugin format: "hook:<plugin>/<event_label>"
    {
        // plugin_name → event_label → Vec<TreeNode>
        let mut by_plugin: BTreeMap<String, BTreeMap<String, Vec<TreeNode>>> = BTreeMap::new();
        let mut total = 0usize;

        for (category, entries) in &report.entries {
            if *category != Category::Hooks {
                continue;
            }
            for entry in entries {
                let (plugin_name, event_label) = entry
                    .from_plugin
                    .as_deref()
                    .and_then(|fp| fp.strip_prefix("hook:"))
                    .and_then(|rest| rest.split_once('/'))
                    .map(|(p, e)| (p.to_string(), e.to_string()))
                    .unwrap_or_else(|| ("unknown".to_string(), "unknown".to_string()));

                let leaf = TreeNode::leaf(&entry.name, NodeKind::Hook, entry);
                by_plugin
                    .entry(plugin_name)
                    .or_default()
                    .entry(event_label)
                    .or_default()
                    .push(leaf);
                total += 1;
            }
        }

        if !by_plugin.is_empty() {
            let mut hooks_section = TreeNode::section(&format!("Hooks ({})", total));

            for (plugin_name, events) in by_plugin {
                let plugin_hook_count: usize = events.values().map(|v| v.len()).sum();
                let mut plugin_node =
                    TreeNode::section(&format!("{} ({})", plugin_name, plugin_hook_count));
                plugin_node.expanded = false;

                for (event_name, commands) in events {
                    if commands.len() == 1 {
                        // Single command per event — show inline without sub-group
                        let mut leaf = commands.into_iter().next().unwrap();
                        leaf.name = format!("{} → {}", event_name, leaf.name);
                        plugin_node.children.push(leaf);
                    } else {
                        let mut event_node =
                            TreeNode::section(&format!("{} ({})", event_name, commands.len()));
                        event_node.expanded = false;
                        event_node.children = commands;
                        plugin_node.children.push(event_node);
                    }
                }

                hooks_section.children.push(plugin_node);
            }
            tree.push(hooks_section);
        }
    }

    // Standalone sections (only when non-empty), in a stable order.
    // For "Skills", group leaves by their `source_repo` (set by the
    // scanner from skills-lock.json) into per-repo sub-headers. Items
    // without a source_repo stay as direct children of the section.
    for label in &["Skills", "Commands", "Agents"] {
        if let Some(nodes) = standalone.get(label) {
            if !nodes.is_empty() {
                let mut section = TreeNode::section(label);
                if *label == "Skills" {
                    section.children = group_by_source_repo(nodes.clone());
                } else {
                    section.children = nodes.clone();
                }
                tree.push(section);
            }
        }
    }

    tree
}

/// Partition a flat list of skill leaves into per-source-repo subgroups.
/// Items without `source_repo` stay as top-level children; items sharing
/// the same `source_repo` collapse into a single sub-header. Order is
/// preserved across both pools.
fn group_by_source_repo(nodes: Vec<TreeNode>) -> Vec<TreeNode> {
    use std::collections::BTreeMap;

    let mut grouped: BTreeMap<String, Vec<TreeNode>> = BTreeMap::new();
    let mut ungrouped: Vec<TreeNode> = Vec::new();

    for node in nodes {
        match &node.source_repo {
            Some(repo) => grouped.entry(repo.clone()).or_default().push(node),
            None => ungrouped.push(node),
        }
    }

    let mut out = ungrouped;
    for (repo, children) in grouped {
        let mut header = TreeNode::section(&format!("skills from {} ({})", repo, children.len()));
        header.expanded = true;
        header.children = children;
        out.push(header);
    }
    out
}
