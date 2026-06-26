pub mod reconciler;
pub mod renderer;
pub mod scanner;

#[derive(Debug, Clone, PartialEq)]
pub enum Scope {
    Project,
    Global,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Management {
    Managed,
    Manual,
}

#[derive(Debug, Clone)]
pub struct DiscoveredItem {
    pub name: String,
    pub version: Option<String>,
    pub scope: Scope,
    pub source_path: String,
    /// For plugin-cache items: "plugin@marketplace" identifier
    pub from_plugin: Option<String>,
    /// For project-scoped skills: the upstream `owner/repo` recorded in
    /// chord.lock. Used by the reconciler to match 2-segment wildcard
    /// entries in chord.toml (`"mattpocock/skills" = "latest"`) against
    /// the individual SKILL.md directories on disk.
    pub source_repo: Option<String>,
}

#[derive(Debug, Clone)]
pub struct AuditEntry {
    pub name: String,
    pub version: Option<String>,
    pub scope: Option<Scope>,
    pub management: Management,
    pub path: Option<String>,
    pub drift: bool,
    pub overridden_by: Option<String>,
    /// Whether this item's parent plugin is enabled
    pub enabled: bool,
    /// For plugin-cache items: "plugin@marketplace". For hooks: "hook:EventName".
    pub from_plugin: Option<String>,
    /// For project-scoped skills: the upstream `owner/repo` from chord.lock.
    /// Drives source-repo grouping in the renderer and the TUI tree.
    pub source_repo: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    Mcp,
    Plugins,
    Skills,
    Commands,
    Agents,
    Hooks,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Mcp => "MCP Servers",
            Category::Plugins => "Plugins",
            Category::Skills => "Skills",
            Category::Commands => "Commands",
            Category::Agents => "Agents",
            Category::Hooks => "Hooks",
        }
    }

    pub fn cli_name(&self) -> &'static str {
        match self {
            Category::Mcp => "mcp",
            Category::Plugins => "plugins",
            Category::Skills => "skills",
            Category::Commands => "commands",
            Category::Agents => "agents",
            Category::Hooks => "hooks",
        }
    }

    pub fn all() -> Vec<Category> {
        vec![
            Category::Mcp,
            Category::Plugins,
            Category::Skills,
            Category::Commands,
            Category::Agents,
            Category::Hooks,
        ]
    }
}

pub struct AuditReport {
    pub entries: Vec<(Category, Vec<AuditEntry>)>,
}

use crate::core::config::Config;
use std::path::Path;

pub fn run_inspect(
    project_root: &Path,
    home_dir: &Path,
    config: &Config,
    section_filter: Option<&str>,
    json_output: bool,
) {
    // Collect enabled plugins to tag active items
    let enabled_plugins = scanner::collect_enabled_plugins(project_root, home_dir);

    let categories: Vec<Category> = if let Some(filter) = section_filter {
        Category::all()
            .into_iter()
            .filter(|c| c.cli_name() == filter)
            .collect()
    } else {
        Category::all()
    };

    let mut report_entries = Vec::new();
    for category in categories {
        let discovered = match category {
            Category::Mcp => scanner::scan_mcp(project_root, home_dir),
            Category::Plugins => scanner::scan_plugins(project_root, home_dir),
            Category::Skills => {
                let mut items = scanner::scan_skills(project_root, home_dir);
                let lockfile =
                    crate::core::lockfile::Lockfile::from_file(&project_root.join("chord.lock"))
                        .unwrap_or_default();
                let sources = scanner::skill_sources_from_lock(&lockfile);
                for item in &mut items {
                    if item.scope == Scope::Project {
                        if let Some(src) = sources.get(&item.name) {
                            item.source_repo = Some(src.clone());
                        }
                    }
                }
                items
            }
            Category::Commands => scanner::scan_commands(project_root, home_dir),
            Category::Agents => scanner::scan_agents(project_root, home_dir),
            Category::Hooks => scanner::scan_hooks(project_root, home_dir),
        };
        let entries =
            reconciler::reconcile(category.clone(), &discovered, config, &enabled_plugins);
        report_entries.push((category, entries));
    }

    let report = AuditReport {
        entries: report_entries,
    };
    if json_output {
        renderer::render_json(&report);
    } else {
        renderer::render_terminal(&report);
    }
}
