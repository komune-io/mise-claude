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
