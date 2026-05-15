use std::path::PathBuf;

use crate::tui::tree::TreeNode;

#[derive(Debug, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    ViewMarkdown,
    ConfirmDisable, // still here; replaced by ScopePicker in Task 16
    AddPrompt,
    ConfirmRemove,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Focus {
    Tree,
    Preview,
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
    /// Plugin awaiting disable confirmation in `Mode::ConfirmDisable`.
    pub pending_disable: Option<String>,
    pub add_input: String,
    pub focus: Focus,
    pub home_dir: Option<PathBuf>,
    pub pending_remove: Option<String>,
}

impl App {
    pub fn new(tree: Vec<TreeNode>, home_dir: Option<PathBuf>) -> Self {
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
            // Default to enabled-only so users see what's actually active in
            // their environment; `i` toggles to show everything including
            // cached-but-disabled plugins.
            show_enabled_only: true,
            pending_disable: None,
            add_input: String::new(),
            focus: Focus::Tree,
            home_dir,
            pending_remove: None,
        };
        app.rebuild_flat();
        app.update_preview();
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
        self.update_preview();
    }

    pub fn move_down(&mut self) {
        if !self.flat.is_empty() && self.selected < self.flat.len() - 1 {
            self.selected += 1;
        }
        self.detail_scroll = 0;
        self.update_preview();
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
            self.update_preview();
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
            self.update_preview();
        }
    }

    pub fn collapse(&mut self) {
        if let Some(entry) = self.flat.get(self.selected) {
            let path = entry.node_index.clone();
            if let Some(node) = self.resolve_node_mut(&path) {
                node.expanded = false;
            }
            self.rebuild_flat();
            self.update_preview();
        }
    }

    pub fn toggle_enabled_filter(&mut self) {
        self.show_enabled_only = !self.show_enabled_only;
        self.rebuild_flat();
        self.selected = 0;
        let label = if self.show_enabled_only {
            "enabled only"
        } else {
            "all"
        };
        self.set_status(format!("Filter: {}", label));
        self.update_preview();
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
        self.update_preview();
    }

    /// Re-scan the project + home environment and rebuild the tree.
    ///
    /// Best-effort selection preservation: if a node with the same name+kind as
    /// the previous selection exists in the new flat list, select it. Otherwise
    /// fall back to index 0. `show_enabled_only` and `focus` are preserved.
    /// `mode` is forced to `Mode::Normal`.
    pub fn reload(
        &mut self,
        project_root: &std::path::Path,
        home_dir: &std::path::Path,
        config: &crate::config::Config,
    ) {
        use crate::inspect::{reconciler, scanner, AuditReport, Category};
        use crate::tui::tree::build_tree;

        let enabled_plugins = scanner::collect_enabled_plugins(project_root, home_dir);
        let mut report_entries = Vec::new();
        for category in Category::all() {
            let discovered = match category {
                Category::Mcp => scanner::scan_mcp(project_root, home_dir),
                Category::Plugins => scanner::scan_plugins(project_root, home_dir),
                Category::Skills => scanner::scan_skills(project_root, home_dir),
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
        let new_tree = build_tree(&report);

        let prev = self
            .selected_node()
            .map(|n| (n.name.clone(), n.kind.clone()));

        self.tree = new_tree;
        self.mode = Mode::Normal;
        self.rebuild_flat();

        if let Some((name, kind)) = prev {
            for (i, entry) in self.flat.iter().enumerate() {
                if let Some(node) = self.resolve_node(&entry.node_index) {
                    if node.name == name && node.kind == kind {
                        self.selected = i;
                        break;
                    }
                }
            }
        }

        if self.selected >= self.flat.len().max(1) {
            self.selected = 0;
        }
        self.detail_scroll = 0;
        self.markdown_scroll = 0;
        self.update_preview();
    }

    /// Reload the markdown preview based on the current selection.
    ///
    /// Hides the preview when the selected node has no readable file. Resets
    /// scroll and force-returns focus to the tree whenever preview becomes
    /// unavailable, so the user is never stranded in preview focus.
    pub fn update_preview(&mut self) {
        self.markdown_content = None;
        self.markdown_scroll = 0;

        let Some(node) = self.selected_node() else {
            self.focus = Focus::Tree;
            return;
        };

        let Some(path) = node.path.clone() else {
            self.focus = Focus::Tree;
            return;
        };

        if path.starts_with("plugin ") {
            self.focus = Focus::Tree;
            return;
        }

        let expanded = expand_tilde(&path, self.home_dir.as_deref());
        match std::fs::read_to_string(&expanded) {
            Ok(content) => {
                self.markdown_content = Some(content);
            }
            Err(e) => {
                self.focus = Focus::Tree;
                self.set_status(format!("Preview unavailable: {}", e));
            }
        }
    }
}

/// Expand a leading "~/" segment using the supplied home directory.
/// Leaves the path unchanged if it does not start with "~/" or if no home is set.
pub fn expand_tilde(path: &str, home: Option<&std::path::Path>) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(h) = home {
            return h.join(rest).to_string_lossy().to_string();
        }
    }
    path.to_string()
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
    if enabled_only && !node.enabled && node.kind != NodeKind::SectionHeader && !node.drift {
        return;
    }
    let is_expandable = if enabled_only {
        node.children
            .iter()
            .any(|c| c.enabled || c.kind == NodeKind::SectionHeader || c.drift)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::tree::{NodeKind, TreeNode};
    use std::io::Write;

    fn leaf_with_path(name: &str, path: Option<String>) -> TreeNode {
        TreeNode {
            name: name.to_string(),
            kind: NodeKind::Skill,
            enabled: true,
            scope: None,
            path,
            plugin_id: None,
            children: Vec::new(),
            expanded: false,
            hidden: false,
            drift: false,
            managed: false,
        }
    }

    fn make_app_with(nodes: Vec<TreeNode>) -> App {
        App::new(nodes, None)
    }

    #[test]
    fn update_preview_clears_for_node_without_path() {
        let mut app = make_app_with(vec![leaf_with_path("orphan", None)]);
        app.markdown_content = Some("stale".to_string());
        app.focus = Focus::Preview;
        app.update_preview();
        assert!(app.markdown_content.is_none());
        assert_eq!(app.focus, Focus::Tree);
    }

    #[test]
    fn update_preview_clears_for_plugin_pseudo_path() {
        let mut app = make_app_with(vec![leaf_with_path("plug", Some("plugin foo".to_string()))]);
        app.markdown_content = Some("stale".to_string());
        app.focus = Focus::Preview;
        app.update_preview();
        assert!(app.markdown_content.is_none());
        assert_eq!(app.focus, Focus::Tree);
    }

    #[test]
    fn update_preview_loads_existing_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "# Hello").unwrap();
        let path = file.path().to_string_lossy().to_string();

        let mut app = make_app_with(vec![leaf_with_path("skill", Some(path))]);
        app.update_preview();
        let content = app.markdown_content.as_deref().unwrap_or("");
        assert!(content.contains("# Hello"));
        assert_eq!(app.markdown_scroll, 0);
    }

    #[test]
    fn update_preview_clears_and_resets_focus_on_read_error() {
        let mut app = make_app_with(vec![leaf_with_path(
            "missing",
            Some("/nonexistent/path/file.md".to_string()),
        )]);
        app.markdown_content = Some("stale".to_string());
        app.focus = Focus::Preview;
        app.markdown_scroll = 5;
        app.update_preview();
        assert!(app.markdown_content.is_none());
        assert_eq!(app.focus, Focus::Tree);
        assert_eq!(app.markdown_scroll, 0);
        assert!(app.status_message.is_some());
    }

    #[test]
    fn expand_tilde_uses_home_dir() {
        let home = std::path::PathBuf::from("/home/user");
        assert_eq!(expand_tilde("~/file.md", Some(&home)), "/home/user/file.md");
        assert_eq!(expand_tilde("/abs/path", Some(&home)), "/abs/path");
        assert_eq!(expand_tilde("~/file.md", None), "~/file.md");
    }

    #[test]
    fn move_down_calls_update_preview() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "second").unwrap();
        let path = file.path().to_string_lossy().to_string();

        let mut app = make_app_with(vec![
            leaf_with_path("first", None),
            leaf_with_path("second", Some(path.clone())),
        ]);
        assert!(app.markdown_content.is_none());
        app.move_down();
        assert!(app
            .markdown_content
            .as_deref()
            .unwrap_or("")
            .contains("second"));
    }

    #[test]
    fn move_up_calls_update_preview() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "first").unwrap();
        let path = file.path().to_string_lossy().to_string();

        let mut app = make_app_with(vec![
            leaf_with_path("first", Some(path)),
            leaf_with_path("second", None),
        ]);
        app.move_down();
        assert!(app.markdown_content.is_none());
        app.move_up();
        assert!(app
            .markdown_content
            .as_deref()
            .unwrap_or("")
            .contains("first"));
    }

    #[test]
    fn new_calls_update_preview_for_initial_selection() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "initial").unwrap();
        let path = file.path().to_string_lossy().to_string();

        let app = make_app_with(vec![leaf_with_path("initial", Some(path))]);
        assert!(app
            .markdown_content
            .as_deref()
            .unwrap_or("")
            .contains("initial"));
    }
}
