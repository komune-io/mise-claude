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
