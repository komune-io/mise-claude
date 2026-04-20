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
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Category {
    Mcp,
    Plugins,
    Skills,
    Commands,
    Agents,
}

impl Category {
    pub fn label(&self) -> &'static str {
        match self {
            Category::Mcp => "MCP Servers",
            Category::Plugins => "Plugins",
            Category::Skills => "Skills",
            Category::Commands => "Commands",
            Category::Agents => "Agents",
        }
    }

    pub fn cli_name(&self) -> &'static str {
        match self {
            Category::Mcp => "mcp",
            Category::Plugins => "plugins",
            Category::Skills => "skills",
            Category::Commands => "commands",
            Category::Agents => "agents",
        }
    }

    pub fn all() -> Vec<Category> {
        vec![
            Category::Mcp,
            Category::Plugins,
            Category::Skills,
            Category::Commands,
            Category::Agents,
        ]
    }
}

pub struct AuditReport {
    pub entries: Vec<(Category, Vec<AuditEntry>)>,
}

use crate::config::Config;
use std::path::Path;

pub fn run_inspect(
    project_root: &Path,
    home_dir: &Path,
    config: &Config,
    section_filter: Option<&str>,
    json_output: bool,
) {
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
            Category::Skills => scanner::scan_skills(project_root, home_dir),
            Category::Commands => scanner::scan_commands(project_root, home_dir),
            Category::Agents => scanner::scan_agents(project_root, home_dir),
        };
        let entries = reconciler::reconcile(category.clone(), &discovered, config);
        report_entries.push((category, entries));
    }

    let report = AuditReport { entries: report_entries };
    if json_output {
        renderer::render_json(&report);
    } else {
        renderer::render_terminal(&report);
    }
}
