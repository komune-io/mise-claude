use crate::tui::tree::TreeNode;

#[derive(Debug, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    ViewMarkdown,
}

#[derive(Debug, Clone)]
pub struct FlatEntry {
    pub depth: usize,
    pub node_index: Vec<usize>, // path into tree: [root_idx, child_idx, ...]
    pub is_expandable: bool,
    pub expanded: bool,
}

pub struct App {
    pub tree: Vec<TreeNode>,
    pub flat: Vec<FlatEntry>,
    pub selected: usize,
    pub mode: Mode,
    pub search_query: String,
    pub detail_scroll: u16,
    pub markdown_content: Option<String>,
    pub markdown_scroll: u16,
    pub status_message: Option<(String, std::time::Instant)>,
    pub should_quit: bool,
    pub show_enabled_only: bool,
}

impl App {
    pub fn new(tree: Vec<TreeNode>) -> Self {
        let mut app = Self {
            tree,
            flat: Vec::new(),
            selected: 0,
            mode: Mode::Normal,
            search_query: String::new(),
            detail_scroll: 0,
            markdown_content: None,
            markdown_scroll: 0,
            status_message: None,
            should_quit: false,
            show_enabled_only: false,
        };
        app.rebuild_flat();
        app
    }

    pub fn rebuild_flat(&mut self) {
        self.flat.clear();
        let enabled_only = self.show_enabled_only;
        for i in 0..self.tree.len() {
            flatten_node(&self.tree[i], vec![i], 0, enabled_only, &mut self.flat);
        }
    }

    pub fn selected_node(&self) -> Option<&TreeNode> {
        let entry = self.flat.get(self.selected)?;
        self.resolve_node(&entry.node_index)
    }

    pub fn selected_node_mut(&mut self) -> Option<&mut TreeNode> {
        let path = self.flat.get(self.selected)?.node_index.clone();
        self.resolve_node_mut(&path)
    }

    pub fn resolve_node(&self, path: &[usize]) -> Option<&TreeNode> {
        let (first, rest) = path.split_first()?;
        let mut node = self.tree.get(*first)?;
        for &idx in rest {
            node = node.children.get(idx)?;
        }
        Some(node)
    }

    pub fn resolve_node_mut(&mut self, path: &[usize]) -> Option<&mut TreeNode> {
        let (first, rest) = path.split_first()?;
        let mut node = self.tree.get_mut(*first)?;
        for &idx in rest {
            node = node.children.get_mut(idx)?;
        }
        Some(node)
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.detail_scroll = 0;
    }

    pub fn move_down(&mut self) {
        if !self.flat.is_empty() && self.selected < self.flat.len() - 1 {
            self.selected += 1;
        }
        self.detail_scroll = 0;
    }

    pub fn toggle_expand(&mut self) {
        if let Some(entry) = self.flat.get(self.selected) {
            if !entry.is_expandable {
                return;
            }
            let path = entry.node_index.clone();
            if let Some(node) = self.resolve_node_mut(&path) {
                node.expanded = !node.expanded;
            }
            self.rebuild_flat();
        }
    }

    pub fn expand(&mut self) {
        if let Some(entry) = self.flat.get(self.selected) {
            if !entry.is_expandable {
                return;
            }
            let path = entry.node_index.clone();
            if let Some(node) = self.resolve_node_mut(&path) {
                node.expanded = true;
            }
            self.rebuild_flat();
        }
    }

    pub fn collapse(&mut self) {
        if let Some(entry) = self.flat.get(self.selected) {
            let path = entry.node_index.clone();
            if let Some(node) = self.resolve_node_mut(&path) {
                node.expanded = false;
            }
            self.rebuild_flat();
        }
    }

    pub fn toggle_enabled_filter(&mut self) {
        self.show_enabled_only = !self.show_enabled_only;
        self.rebuild_flat();
        self.selected = 0;
        let label = if self.show_enabled_only { "enabled only" } else { "all" };
        self.set_status(format!("Filter: {}", label));
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some((msg, std::time::Instant::now()));
    }

    pub fn apply_search_filter(&mut self) {
        if self.search_query.is_empty() {
            // Unhide all nodes
            for node in &mut self.tree {
                unhide_all(node);
            }
        } else {
            let query = self.search_query.to_lowercase();
            for node in &mut self.tree {
                filter_node(node, &query);
            }
        }
        self.rebuild_flat();
        self.selected = 0;
    }
}

fn flatten_node(
    node: &TreeNode,
    path: Vec<usize>,
    depth: usize,
    enabled_only: bool,
    flat: &mut Vec<FlatEntry>,
) {
    use crate::tui::tree::NodeKind;

    if node.hidden {
        return;
    }
    if enabled_only && !node.enabled && node.kind != NodeKind::SectionHeader {
        return;
    }
    let is_expandable = if enabled_only {
        node.children.iter().any(|c| c.enabled || c.kind == NodeKind::SectionHeader)
    } else {
        !node.children.is_empty()
    };
    flat.push(FlatEntry {
        depth,
        node_index: path.clone(),
        is_expandable,
        expanded: node.expanded,
    });
    if node.expanded {
        for (i, child) in node.children.iter().enumerate() {
            let mut child_path = path.clone();
            child_path.push(i);
            flatten_node(child, child_path, depth + 1, enabled_only, flat);
        }
    }
}

/// Recursively unhide all nodes in the subtree.
fn unhide_all(node: &mut TreeNode) {
    node.hidden = false;
    for child in &mut node.children {
        unhide_all(child);
    }
}

/// Returns true if this node or any descendant matches the query.
/// Sets `hidden` accordingly, and expands parents of matches.
fn filter_node(node: &mut TreeNode, query: &str) -> bool {
    let self_matches = node.name.to_lowercase().contains(query);

    let any_child_matches = node
        .children
        .iter_mut()
        .map(|child| filter_node(child, query))
        .fold(false, |acc, m| acc || m);

    if self_matches || any_child_matches {
        node.hidden = false;
        if any_child_matches {
            node.expanded = true;
        }
        true
    } else {
        node.hidden = true;
        false
    }
}
