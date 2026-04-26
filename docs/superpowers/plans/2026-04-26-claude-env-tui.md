# claude-env inspect TUI Implementation Plan (Phase 1)

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an interactive TUI mode (`claude-env inspect --tui`) with a tree view of all Claude Code configuration, detail panel, search, and enable/disable toggle.

**Architecture:** ratatui + crossterm for terminal rendering. The existing `AuditReport` from the inspect module is converted into a hierarchical `TreeNode` structure. An event loop reads keyboard input, mutates `App` state, and re-renders each frame. Side effects (toggle plugin) write directly to `settings.json`.

**Tech Stack:** Rust, ratatui 0.29, crossterm 0.28, existing inspect scanners/reconciler

---

## File Structure

```
claude-env/src/
├── tui/
│   ├── mod.rs          # run_tui() entry, terminal setup/teardown, event loop
│   ├── app.rs          # App state struct, Mode enum, navigation methods
│   ├── tree.rs         # TreeNode, build_tree() from AuditReport, flatten/filter
│   ├── ui.rs           # ratatui rendering: layout split, tree widget, detail widget
│   ├── handler.rs      # map KeyEvent → App state mutations
│   └── actions.rs      # toggle_plugin() writes settings.json
├── cli.rs              # (modify) Add --tui flag to Inspect
├── main.rs             # (modify) Call run_tui when --tui
└── lib.rs              # (modify) Add pub mod tui
```

---

## Task 1: Add Dependencies + TUI Module Skeleton

**Files:**
- Modify: `claude-env/Cargo.toml`
- Create: `claude-env/src/tui/mod.rs`
- Create: `claude-env/src/tui/app.rs`
- Create: `claude-env/src/tui/tree.rs`
- Create: `claude-env/src/tui/ui.rs`
- Create: `claude-env/src/tui/handler.rs`
- Create: `claude-env/src/tui/actions.rs`
- Modify: `claude-env/src/lib.rs`
- Modify: `claude-env/src/cli.rs`
- Modify: `claude-env/src/main.rs`

- [ ] **Step 1: Add ratatui and crossterm to Cargo.toml**

Add under `[dependencies]`:
```toml
ratatui = "0.29"
crossterm = "0.28"
```

- [ ] **Step 2: Create tui/mod.rs with run_tui stub**

```rust
pub mod actions;
pub mod app;
pub mod handler;
pub mod tree;
pub mod ui;

use std::io;
use std::path::Path;
use crate::config::Config;

pub fn run_tui(
    project_root: &Path,
    home_dir: &Path,
    config: &Config,
) -> io::Result<()> {
    println!("TUI not yet implemented");
    Ok(())
}
```

- [ ] **Step 3: Create empty placeholder files**

`tui/app.rs`:
```rust
// App state — implemented in Task 2
```

`tui/tree.rs`:
```rust
// Tree building — implemented in Task 3
```

`tui/ui.rs`:
```rust
// Rendering — implemented in Task 5
```

`tui/handler.rs`:
```rust
// Key handling — implemented in Task 6
```

`tui/actions.rs`:
```rust
// Side effects — implemented in Task 7
```

- [ ] **Step 4: Add `pub mod tui;` to lib.rs**

- [ ] **Step 5: Add `--tui` flag to cli.rs Inspect variant**

```rust
    Inspect {
        #[arg(long)]
        section: Option<String>,

        #[arg(long)]
        json: bool,

        /// Launch interactive TUI mode.
        #[arg(long)]
        tui: bool,
    },
```

- [ ] **Step 6: Wire --tui into main.rs**

Replace the `Command::Inspect` arm. Read main.rs first — keep existing logic, add tui branch:

```rust
        Command::Inspect { section, json, tui } => {
            let project_root = std::path::PathBuf::from(".");
            let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            let config_path = std::path::PathBuf::from("claude-env.toml");
            let config = Config::from_file(&config_path).unwrap_or_default();

            if tui {
                if let Err(e) = claude_env::tui::run_tui(&project_root, &home_dir, &config) {
                    eprintln!("TUI error: {e}");
                    process::exit(1);
                }
            } else {
                claude_env::inspect::run_inspect(
                    &project_root,
                    &home_dir,
                    &config,
                    section.as_deref(),
                    json,
                );
            }
        }
```

- [ ] **Step 7: Verify build and --help**

```bash
cd claude-env && cargo build && cargo run -- inspect --help
```

Expected: `--tui` flag visible in help output.

- [ ] **Step 8: Commit**

```bash
git add claude-env/Cargo.toml claude-env/src/tui/ claude-env/src/lib.rs claude-env/src/cli.rs claude-env/src/main.rs
git commit -m "feat(claude-env): add TUI module skeleton with --tui flag"
```

---

## Task 2: App State

**Files:**
- Replace: `claude-env/src/tui/app.rs`

- [ ] **Step 1: Implement App struct and Mode enum**

```rust
use crate::tui::tree::TreeNode;

#[derive(Debug, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    ViewMarkdown,
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
}

/// A flattened view of the tree for rendering. Each entry knows its depth.
#[derive(Debug, Clone)]
pub struct FlatEntry {
    pub depth: usize,
    pub node_index: Vec<usize>,  // path into tree: [root_idx, child_idx, ...]
    pub is_expandable: bool,
    pub expanded: bool,
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
        };
        app.rebuild_flat();
        app
    }

    pub fn rebuild_flat(&mut self) {
        self.flat.clear();
        for (i, node) in self.tree.iter().enumerate() {
            self.flatten_node(node, vec![i], 0);
        }
        if self.selected >= self.flat.len() && !self.flat.is_empty() {
            self.selected = self.flat.len() - 1;
        }
    }

    fn flatten_node(&mut self, node: &TreeNode, path: Vec<usize>, depth: usize) {
        let is_expandable = !node.children.is_empty();
        self.flat.push(FlatEntry {
            depth,
            node_index: path.clone(),
            is_expandable,
            expanded: node.expanded,
        });

        if node.expanded {
            for (i, child) in node.children.iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(i);
                self.flatten_node(child, child_path, depth + 1);
            }
        }
    }

    pub fn selected_node(&self) -> Option<&TreeNode> {
        self.flat.get(self.selected).and_then(|entry| {
            self.resolve_node(&entry.node_index)
        })
    }

    pub fn selected_node_mut(&mut self) -> Option<&mut TreeNode> {
        let path = self.flat.get(self.selected)?.node_index.clone();
        self.resolve_node_mut(&path)
    }

    fn resolve_node(&self, path: &[usize]) -> Option<&TreeNode> {
        let mut nodes = &self.tree;
        let mut node = None;
        for (i, &idx) in path.iter().enumerate() {
            let n = nodes.get(idx)?;
            if i == path.len() - 1 {
                node = Some(n);
            } else {
                nodes = &n.children;
            }
        }
        node
    }

    fn resolve_node_mut(&mut self, path: &[usize]) -> Option<&mut TreeNode> {
        let mut nodes = &mut self.tree;
        let len = path.len();
        for (i, &idx) in path.iter().enumerate() {
            if i == len - 1 {
                return nodes.get_mut(idx);
            }
            nodes = &mut nodes.get_mut(idx)?.children;
        }
        None
    }

    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
            self.detail_scroll = 0;
        }
    }

    pub fn move_down(&mut self) {
        if self.selected + 1 < self.flat.len() {
            self.selected += 1;
            self.detail_scroll = 0;
        }
    }

    pub fn toggle_expand(&mut self) {
        if let Some(node) = self.selected_node_mut() {
            if !node.children.is_empty() {
                node.expanded = !node.expanded;
                self.rebuild_flat();
            }
        }
    }

    pub fn expand(&mut self) {
        if let Some(node) = self.selected_node_mut() {
            if !node.children.is_empty() && !node.expanded {
                node.expanded = true;
                self.rebuild_flat();
            }
        }
    }

    pub fn collapse(&mut self) {
        if let Some(node) = self.selected_node_mut() {
            if node.expanded {
                node.expanded = false;
                self.rebuild_flat();
            }
        }
    }

    pub fn set_status(&mut self, msg: String) {
        self.status_message = Some((msg, std::time::Instant::now()));
    }

    pub fn apply_search_filter(&mut self) {
        fn mark_matching(node: &mut TreeNode, query: &str) -> bool {
            let self_matches = node.name.to_lowercase().contains(query);
            let mut any_child_matches = false;
            for child in &mut node.children {
                if mark_matching(child, query) {
                    any_child_matches = true;
                }
            }
            node.hidden = !self_matches && !any_child_matches;
            if any_child_matches {
                node.expanded = true;
            }
            self_matches || any_child_matches
        }

        let query = self.search_query.to_lowercase();
        if query.is_empty() {
            fn unhide_all(node: &mut TreeNode) {
                node.hidden = false;
                for child in &mut node.children {
                    unhide_all(child);
                }
            }
            for node in &mut self.tree {
                unhide_all(node);
            }
        } else {
            for node in &mut self.tree {
                mark_matching(node, &query);
            }
        }
        self.rebuild_flat();
        self.selected = 0;
    }
}
```

Note: `rebuild_flat` must skip nodes where `hidden == true`. Update `flatten_node`:

```rust
    fn flatten_node(&mut self, node: &TreeNode, path: Vec<usize>, depth: usize) {
        if node.hidden {
            return;
        }
        let is_expandable = !node.children.is_empty();
        self.flat.push(FlatEntry {
            depth,
            node_index: path.clone(),
            is_expandable,
            expanded: node.expanded,
        });

        if node.expanded {
            for (i, child) in node.children.iter().enumerate() {
                let mut child_path = path.clone();
                child_path.push(i);
                self.flatten_node(child, child_path, depth + 1);
            }
        }
    }
```

- [ ] **Step 2: Verify build**

```bash
cargo build
```

- [ ] **Step 3: Commit**

```bash
git add claude-env/src/tui/app.rs
git commit -m "feat(claude-env): implement App state with tree navigation and search"
```

---

## Task 3: Tree Builder

**Files:**
- Replace: `claude-env/src/tui/tree.rs`
- Create: `claude-env/tests/unit/tree_test.rs`
- Modify: `claude-env/tests/unit.rs`

- [ ] **Step 1: Write failing tests**

Create `claude-env/tests/unit/tree_test.rs`:

```rust
use claude_env::inspect::{AuditEntry, AuditReport, Category, Management, Scope};
use claude_env::tui::tree::{build_tree, NodeKind};

fn make_entry(name: &str, scope: Scope, enabled: bool, path: &str) -> AuditEntry {
    AuditEntry {
        name: name.to_string(),
        version: None,
        scope: Some(scope),
        management: Management::Manual,
        path: Some(path.to_string()),
        drift: false,
        overridden_by: None,
        enabled,
    }
}

#[test]
fn build_tree_groups_by_plugin() {
    let report = AuditReport {
        entries: vec![
            (Category::Plugins, vec![
                make_entry("superpowers@claude-plugins-official", Scope::Global, true, "~/.claude/settings.json"),
            ]),
            (Category::Skills, vec![
                {
                    let mut e = make_entry("brainstorming", Scope::Global, true, "plugin superpowers@claude-plugins-official");
                    e
                },
                make_entry("standalone-skill", Scope::Global, true, "~/.claude/skills/standalone-skill/SKILL.md"),
            ]),
            (Category::Commands, vec![]),
            (Category::Agents, vec![]),
            (Category::Mcp, vec![]),
        ],
    };

    let tree = build_tree(&report);

    // Should have "Plugins" section with 1 plugin, and plugin should have "brainstorming" child
    let plugins_node = tree.iter().find(|n| n.name == "Plugins").unwrap();
    assert_eq!(plugins_node.children.len(), 1);
    let sp_node = &plugins_node.children[0];
    assert_eq!(sp_node.name, "superpowers@claude-plugins-official");
    assert!(sp_node.enabled);
    // brainstorming should be a child of the plugin
    assert!(sp_node.children.iter().any(|c| c.name == "brainstorming"));
}

#[test]
fn build_tree_standalone_items() {
    let report = AuditReport {
        entries: vec![
            (Category::Plugins, vec![]),
            (Category::Skills, vec![
                make_entry("my-skill", Scope::Project, true, ".claude/skills/my-skill/SKILL.md"),
            ]),
            (Category::Commands, vec![
                make_entry("review", Scope::Project, true, ".claude/commands/review.md"),
            ]),
            (Category::Agents, vec![]),
            (Category::Mcp, vec![]),
        ],
    };

    let tree = build_tree(&report);
    let skills = tree.iter().find(|n| n.name == "Skills").unwrap();
    assert_eq!(skills.children.len(), 1);
    assert_eq!(skills.children[0].name, "my-skill");
}

#[test]
fn build_tree_mcp_section() {
    let report = AuditReport {
        entries: vec![
            (Category::Plugins, vec![]),
            (Category::Skills, vec![]),
            (Category::Commands, vec![]),
            (Category::Agents, vec![]),
            (Category::Mcp, vec![
                make_entry("context7-mcp", Scope::Project, true, ".mcp.json"),
            ]),
        ],
    };

    let tree = build_tree(&report);
    let mcp = tree.iter().find(|n| n.name == "MCP Servers").unwrap();
    assert_eq!(mcp.children.len(), 1);
    assert_eq!(mcp.children[0].name, "context7-mcp");
}
```

- [ ] **Step 2: Wire test module in unit.rs**

Add to `claude-env/tests/unit.rs`:
```rust
#[path = "unit/tree_test.rs"]
mod tree_test;
```

- [ ] **Step 3: Implement tree.rs**

```rust
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
    pub scope: Option<Scope>,
    pub path: Option<String>,
    pub plugin_id: Option<String>,
    pub children: Vec<TreeNode>,
    pub expanded: bool,
    pub hidden: bool,
}

impl TreeNode {
    fn section(name: &str) -> Self {
        Self {
            name: name.to_string(),
            kind: NodeKind::SectionHeader,
            enabled: false,
            scope: None,
            path: None,
            plugin_id: None,
            children: Vec::new(),
            expanded: true,
            hidden: false,
        }
    }

    fn leaf(name: &str, kind: NodeKind, entry: &AuditEntry) -> Self {
        Self {
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

    fn plugin(name: &str, enabled: bool, scope: Option<Scope>, path: Option<String>) -> Self {
        Self {
            name: name.to_string(),
            kind: NodeKind::Plugin,
            enabled,
            scope,
            path,
            plugin_id: Some(name.to_string()),
            children: Vec::new(),
            expanded: false,
            hidden: false,
        }
    }
}

/// Build a tree from the flat AuditReport.
///
/// Structure:
/// - Plugins section: each plugin node contains its skills/commands/agents as children
/// - MCP Servers section: flat list
/// - Standalone Skills/Commands/Agents: items not from plugins
pub fn build_tree(report: &AuditReport) -> Vec<TreeNode> {
    let mut plugins_section = TreeNode::section("Plugins");
    let mut mcp_section = TreeNode::section("MCP Servers");
    let mut standalone_skills = TreeNode::section("Skills");
    let mut standalone_commands = TreeNode::section("Commands");
    let mut standalone_agents = TreeNode::section("Agents");

    // First pass: collect plugins
    let mut plugin_nodes: BTreeMap<String, TreeNode> = BTreeMap::new();

    for (category, entries) in &report.entries {
        if *category == Category::Plugins {
            for entry in entries {
                let node = TreeNode::plugin(
                    &entry.name,
                    entry.enabled,
                    entry.scope.clone(),
                    entry.path.clone(),
                );
                plugin_nodes.insert(entry.name.clone(), node);
            }
        }
    }

    // Second pass: assign skills/commands/agents to plugins or standalone
    for (category, entries) in &report.entries {
        match category {
            Category::Plugins => {} // already handled
            Category::Mcp => {
                for entry in entries {
                    mcp_section.children.push(TreeNode::leaf(
                        &entry.name,
                        NodeKind::McpServer,
                        entry,
                    ));
                }
            }
            Category::Skills | Category::Commands | Category::Agents => {
                let kind = match category {
                    Category::Skills => NodeKind::Skill,
                    Category::Commands => NodeKind::Command,
                    Category::Agents => NodeKind::Agent,
                    _ => unreachable!(),
                };

                for entry in entries {
                    let plugin_source = extract_plugin_from_path(entry.path.as_deref());

                    if let Some(ref plugin_id) = plugin_source {
                        if let Some(plugin_node) = plugin_nodes.get_mut(plugin_id) {
                            plugin_node.children.push(TreeNode::leaf(&entry.name, kind.clone(), entry));
                            continue;
                        }
                    }

                    // Standalone
                    let target = match category {
                        Category::Skills => &mut standalone_skills,
                        Category::Commands => &mut standalone_commands,
                        Category::Agents => &mut standalone_agents,
                        _ => unreachable!(),
                    };
                    target.children.push(TreeNode::leaf(&entry.name, kind.clone(), entry));
                }
            }
        }
    }

    // Build final tree
    for (_, node) in plugin_nodes {
        plugins_section.children.push(node);
    }

    let mut tree = Vec::new();
    if !plugins_section.children.is_empty() {
        // Count enabled
        let enabled = plugins_section.children.iter().filter(|n| n.enabled).count();
        let total = plugins_section.children.len();
        plugins_section.name = format!("Plugins ({} enabled, {} cached)", enabled, total - enabled);
        tree.push(plugins_section);
    }
    if !mcp_section.children.is_empty() {
        mcp_section.name = format!("MCP Servers ({})", mcp_section.children.len());
        tree.push(mcp_section);
    }
    if !standalone_skills.children.is_empty() {
        tree.push(standalone_skills);
    }
    if !standalone_commands.children.is_empty() {
        tree.push(standalone_commands);
    }
    if !standalone_agents.children.is_empty() {
        tree.push(standalone_agents);
    }

    tree
}

/// Extract plugin identifier from a path like "plugin superpowers@claude-plugins-official"
fn extract_plugin_from_path(path: Option<&str>) -> Option<String> {
    let p = path?;
    if p.starts_with("plugin ") {
        Some(p.strip_prefix("plugin ")?.to_string())
    } else {
        None
    }
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test unit -- tree_test
```

Expected: all 3 PASS.

- [ ] **Step 5: Commit**

```bash
git add claude-env/src/tui/tree.rs claude-env/tests/unit/tree_test.rs claude-env/tests/unit.rs
git commit -m "feat(claude-env): implement tree builder from AuditReport"
```

---

## Task 4: Terminal Setup + Event Loop

**Files:**
- Replace: `claude-env/src/tui/mod.rs`

- [ ] **Step 1: Implement run_tui with terminal setup, scan, build tree, event loop**

```rust
pub mod actions;
pub mod app;
pub mod handler;
pub mod tree;
pub mod ui;

use std::io;
use std::path::Path;
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::prelude::*;

use crate::config::Config;
use crate::inspect::{scanner, reconciler, AuditReport, Category};
use app::App;
use tree::build_tree;

pub fn run_tui(
    project_root: &Path,
    home_dir: &Path,
    config: &Config,
) -> io::Result<()> {
    // Build report using existing inspect pipeline
    let enabled_plugins = scanner::collect_enabled_plugins(project_root, home_dir);

    let mut report_entries = Vec::new();
    for category in Category::all() {
        let discovered = match category {
            Category::Mcp => scanner::scan_mcp(project_root, home_dir),
            Category::Plugins => scanner::scan_plugins(project_root, home_dir),
            Category::Skills => scanner::scan_skills(project_root, home_dir),
            Category::Commands => scanner::scan_commands(project_root, home_dir),
            Category::Agents => scanner::scan_agents(project_root, home_dir),
        };
        let entries = reconciler::reconcile(category.clone(), &discovered, config, &enabled_plugins);
        report_entries.push((category, entries));
    }

    let report = AuditReport { entries: report_entries };
    let tree = build_tree(&report);
    let mut app = App::new(tree);

    // Setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Event loop
    let result = run_loop(&mut terminal, &mut app, home_dir);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn run_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
    home_dir: &Path,
) -> io::Result<()> {
    loop {
        terminal.draw(|frame| ui::render(frame, app))?;

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                handler::handle_key(app, key, home_dir)?;
            }
        }

        // Clear expired status messages (after 3 seconds)
        if let Some((_, time)) = &app.status_message {
            if time.elapsed() > Duration::from_secs(3) {
                app.status_message = None;
            }
        }

        if app.should_quit {
            break;
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cargo build
```

- [ ] **Step 3: Commit**

```bash
git add claude-env/src/tui/mod.rs
git commit -m "feat(claude-env): implement TUI terminal setup and event loop"
```

---

## Task 5: UI Rendering

**Files:**
- Replace: `claude-env/src/tui/ui.rs`

- [ ] **Step 1: Implement render function**

```rust
use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::inspect::Scope;
use crate::tui::app::{App, Mode};
use crate::tui::tree::NodeKind;

pub fn render(frame: &mut Frame, app: &App) {
    match app.mode {
        Mode::ViewMarkdown => render_markdown_overlay(frame, app),
        _ => render_main(frame, app),
    }
}

fn render_main(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Vertical: main area + status bar
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(area);

    let main_area = vertical[0];
    let status_area = vertical[1];

    // Horizontal: tree (45%) + detail (55%)
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(main_area);

    render_tree(frame, app, horizontal[0]);
    render_detail(frame, app, horizontal[1]);
    render_status(frame, app, status_area);
}

fn render_tree(frame: &mut Frame, app: &App, area: Rect) {
    let items: Vec<ListItem> = app
        .flat
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let node = app.resolve_node_pub(&entry.node_index);
            let indent = "  ".repeat(entry.depth);

            let (symbol, style) = match node {
                Some(n) => {
                    let arrow = if entry.is_expandable {
                        if entry.expanded { "▼ " } else { "▶ " }
                    } else {
                        "  "
                    };

                    let (dot, style) = match n.kind {
                        NodeKind::SectionHeader => ("", Style::default().fg(Color::Cyan).bold()),
                        NodeKind::Plugin if n.enabled => ("● ", Style::default().fg(Color::Green)),
                        NodeKind::Plugin => ("○ ", Style::default().fg(Color::DarkGray)),
                        _ if n.enabled => ("", Style::default().fg(Color::Green)),
                        _ => ("", Style::default().fg(Color::DarkGray)),
                    };

                    let s = format!("{indent}{arrow}{dot}{}", n.name);
                    (s, style)
                }
                None => (format!("{indent}???"), Style::default()),
            };

            let mut item = ListItem::new(symbol);
            if i == app.selected {
                item = item.style(style.reversed());
            } else {
                item = item.style(style);
            }
            item
        })
        .collect();

    let title = if app.mode == Mode::Search {
        format!(" Tree [/{}] ", app.search_query)
    } else {
        " Tree ".to_string()
    };

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    frame.render_widget(list, area);
}

fn render_detail(frame: &mut Frame, app: &App, area: Rect) {
    let node = app.selected_node();

    let text = match node {
        Some(n) => {
            let mut lines = vec![
                Line::from(Span::styled(&n.name, Style::default().fg(Color::Cyan).bold())),
                Line::from(""),
            ];

            // Metadata
            let kind_str = match n.kind {
                NodeKind::SectionHeader => "Section",
                NodeKind::Plugin => "Plugin",
                NodeKind::Skill => "Skill",
                NodeKind::Command => "Command",
                NodeKind::Agent => "Agent",
                NodeKind::McpServer => "MCP Server",
            };
            lines.push(Line::from(vec![
                Span::styled("  Type    ", Style::default().fg(Color::DarkGray)),
                Span::raw(kind_str),
            ]));

            if let Some(ref scope) = n.scope {
                let scope_str = match scope {
                    Scope::Project => "project",
                    Scope::Global => "global",
                };
                lines.push(Line::from(vec![
                    Span::styled("  Scope   ", Style::default().fg(Color::DarkGray)),
                    Span::raw(scope_str),
                ]));
            }

            let status = if n.enabled { "● enabled" } else { "○ cached" };
            let status_color = if n.enabled { Color::Green } else { Color::DarkGray };
            lines.push(Line::from(vec![
                Span::styled("  Status  ", Style::default().fg(Color::DarkGray)),
                Span::styled(status, Style::default().fg(status_color)),
            ]));

            if let Some(ref path) = n.path {
                lines.push(Line::from(vec![
                    Span::styled("  Path    ", Style::default().fg(Color::DarkGray)),
                    Span::styled(path, Style::default().fg(Color::DarkGray)),
                ]));
            }

            if let Some(ref plugin_id) = n.plugin_id {
                lines.push(Line::from(vec![
                    Span::styled("  Plugin  ", Style::default().fg(Color::DarkGray)),
                    Span::raw(plugin_id),
                ]));
            }

            if n.kind == NodeKind::Plugin {
                let child_count = n.children.len();
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled(
                        format!("  {} items", child_count),
                        Style::default().fg(Color::DarkGray),
                    ),
                ]));
            }

            // Preview markdown content if loaded
            if let Some(ref content) = app.markdown_content {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled(
                    "  ── Preview ──",
                    Style::default().fg(Color::DarkGray),
                )));
                for line in content.lines().take(20) {
                    lines.push(Line::from(Span::styled(
                        format!("  {}", line),
                        Style::default().fg(Color::DarkGray),
                    )));
                }
            }

            // Keybindings
            lines.push(Line::from(""));
            lines.push(Line::from(vec![
                Span::styled(" [e]", Style::default().fg(Color::Cyan)),
                Span::raw(" toggle "),
                Span::styled("[v]", Style::default().fg(Color::Cyan)),
                Span::raw(" view "),
                Span::styled("[/]", Style::default().fg(Color::Cyan)),
                Span::raw(" search "),
                Span::styled("[q]", Style::default().fg(Color::Cyan)),
                Span::raw(" quit"),
            ]));

            Text::from(lines)
        }
        None => Text::from("No item selected"),
    };

    let detail = Paragraph::new(text)
        .scroll((app.detail_scroll, 0))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Detail ")
                .border_style(Style::default().fg(Color::DarkGray)),
        );

    frame.render_widget(detail, area);
}

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let text = if app.mode == Mode::Search {
        format!(" /{} ", app.search_query)
    } else if let Some((ref msg, _)) = app.status_message {
        format!(" {} ", msg)
    } else {
        " ↑↓ navigate  ←→ expand/collapse  e toggle  v view  / search  q quit ".to_string()
    };

    let style = if app.mode == Mode::Search {
        Style::default().fg(Color::Black).bg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };

    let bar = Paragraph::new(text).style(style);
    frame.render_widget(bar, area);
}

fn render_markdown_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();
    let content = app.markdown_content.as_deref().unwrap_or("No content");

    let lines: Vec<Line> = content.lines().map(|l| Line::from(l.to_string())).collect();
    let text = Text::from(lines);

    let paragraph = Paragraph::new(text)
        .scroll((app.markdown_scroll, 0))
        .wrap(Wrap { trim: false })
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Markdown (q/Esc to close) ")
                .border_style(Style::default().fg(Color::Cyan)),
        );

    frame.render_widget(paragraph, area);
}
```

- [ ] **Step 2: Add public `resolve_node_pub` to App (app.rs)**

Add to `impl App`:
```rust
    pub fn resolve_node_pub(&self, path: &[usize]) -> Option<&TreeNode> {
        self.resolve_node(path)
    }
```

- [ ] **Step 3: Verify build**

```bash
cargo build
```

- [ ] **Step 4: Commit**

```bash
git add claude-env/src/tui/ui.rs claude-env/src/tui/app.rs
git commit -m "feat(claude-env): implement TUI rendering with tree and detail panels"
```

---

## Task 6: Keyboard Handler

**Files:**
- Replace: `claude-env/src/tui/handler.rs`

- [ ] **Step 1: Implement key handler**

```rust
use std::io;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::actions;
use crate::tui::app::{App, Mode};

pub fn handle_key(app: &mut App, key: KeyEvent, home_dir: &Path) -> io::Result<()> {
    match app.mode {
        Mode::Normal => handle_normal(app, key, home_dir),
        Mode::Search => handle_search(app, key),
        Mode::ViewMarkdown => handle_markdown(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent, home_dir: &Path) -> io::Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.move_up();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.move_down();
        }
        KeyCode::Left | KeyCode::Char('h') => {
            app.collapse();
        }
        KeyCode::Right | KeyCode::Char('l') => {
            app.expand();
        }
        KeyCode::Enter => {
            app.toggle_expand();
        }
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.search_query.clear();
        }
        KeyCode::Char('e') => {
            if let Some(node) = app.selected_node() {
                if let Some(ref plugin_id) = node.plugin_id {
                    let plugin_id = plugin_id.clone();
                    let was_enabled = node.enabled;
                    match actions::toggle_plugin(home_dir, &plugin_id, was_enabled) {
                        Ok(()) => {
                            // Update tree state
                            if let Some(node) = app.selected_node_mut() {
                                let new_enabled = !was_enabled;
                                node.enabled = new_enabled;
                                for child in &mut node.children {
                                    child.enabled = new_enabled;
                                }
                            }
                            let verb = if was_enabled { "Disabled" } else { "Enabled" };
                            app.set_status(format!("{} {}", verb, plugin_id));
                        }
                        Err(e) => {
                            app.set_status(format!("Error: {}", e));
                        }
                    }
                } else {
                    app.set_status("Toggle only works on plugin nodes".to_string());
                }
            }
        }
        KeyCode::Char('v') => {
            if let Some(node) = app.selected_node() {
                if let Some(ref path) = node.path {
                    // Try to read the file if it's a real path (not "plugin ...")
                    let content = if path.starts_with("plugin ") || path.starts_with('/') || path.starts_with('~') {
                        // For plugin-sourced items, resolve actual file path
                        // from the source_path stored during scanning
                        None
                    } else {
                        std::fs::read_to_string(path).ok()
                    };

                    // Try home-expanded path
                    let content = content.or_else(|| {
                        let expanded = if path.starts_with("~/") {
                            dirs::home_dir()?.join(&path[2..])
                        } else {
                            std::path::PathBuf::from(path)
                        };
                        std::fs::read_to_string(expanded).ok()
                    });

                    if let Some(text) = content {
                        app.markdown_content = Some(text);
                        app.markdown_scroll = 0;
                        app.mode = Mode::ViewMarkdown;
                    } else {
                        app.set_status(format!("Cannot read: {}", path));
                    }
                }
            }
        }
        _ => {}
    }
    Ok(())
}

fn handle_search(app: &mut App, key: KeyEvent) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.search_query.clear();
            app.apply_search_filter(); // clear filter
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
            // Keep filter active
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.apply_search_filter();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.apply_search_filter();
        }
        _ => {}
    }
    Ok(())
}

fn handle_markdown(app: &mut App, key: KeyEvent) -> io::Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.markdown_content = None;
            app.markdown_scroll = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            if app.markdown_scroll > 0 {
                app.markdown_scroll -= 1;
            }
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.markdown_scroll += 1;
        }
        KeyCode::PageUp => {
            app.markdown_scroll = app.markdown_scroll.saturating_sub(20);
        }
        KeyCode::PageDown => {
            app.markdown_scroll += 20;
        }
        _ => {}
    }
    Ok(())
}
```

- [ ] **Step 2: Verify build**

```bash
cargo build
```

- [ ] **Step 3: Commit**

```bash
git add claude-env/src/tui/handler.rs
git commit -m "feat(claude-env): implement TUI keyboard handler with all Phase 1 shortcuts"
```

---

## Task 7: Toggle Plugin Action

**Files:**
- Replace: `claude-env/src/tui/actions.rs`
- Create: `claude-env/tests/unit/actions_test.rs`
- Modify: `claude-env/tests/unit.rs`

- [ ] **Step 1: Write failing test**

Create `claude-env/tests/unit/actions_test.rs`:

```rust
use tempfile::TempDir;
use std::fs;
use claude_env::tui::actions;

#[test]
fn toggle_plugin_disables() {
    let home = TempDir::new().unwrap();
    let claude_dir = home.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"enabledPlugins":{"superpowers@claude-plugins-official":true,"caveman@caveman":true}}"#,
    ).unwrap();

    actions::toggle_plugin(home.path(), "superpowers@claude-plugins-official", true).unwrap();

    let content = fs::read_to_string(claude_dir.join("settings.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["enabledPlugins"].get("superpowers@claude-plugins-official").is_none());
    assert!(json["enabledPlugins"]["caveman@caveman"].as_bool().unwrap());
}

#[test]
fn toggle_plugin_enables() {
    let home = TempDir::new().unwrap();
    let claude_dir = home.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"enabledPlugins":{"caveman@caveman":true}}"#,
    ).unwrap();

    actions::toggle_plugin(home.path(), "superpowers@claude-plugins-official", false).unwrap();

    let content = fs::read_to_string(claude_dir.join("settings.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["enabledPlugins"]["superpowers@claude-plugins-official"].as_bool().unwrap());
    assert!(json["enabledPlugins"]["caveman@caveman"].as_bool().unwrap());
}

#[test]
fn toggle_plugin_creates_settings_if_missing() {
    let home = TempDir::new().unwrap();
    let claude_dir = home.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    // No settings.json

    actions::toggle_plugin(home.path(), "new-plugin@marketplace", false).unwrap();

    let content = fs::read_to_string(claude_dir.join("settings.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["enabledPlugins"]["new-plugin@marketplace"].as_bool().unwrap());
}
```

- [ ] **Step 2: Wire test in unit.rs**

Add: `#[path = "unit/actions_test.rs"] mod actions_test;`

- [ ] **Step 3: Implement actions.rs**

```rust
use std::io;
use std::path::Path;
use serde_json::{json, Value};

/// Toggle a plugin's enabled state in ~/.claude/settings.json.
///
/// If `currently_enabled` is true, removes the key. Otherwise adds it.
pub fn toggle_plugin(home_dir: &Path, plugin_id: &str, currently_enabled: bool) -> io::Result<()> {
    let settings_path = home_dir.join(".claude").join("settings.json");

    let mut settings: Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
    } else {
        json!({})
    };

    let plugins = settings
        .as_object_mut()
        .unwrap()
        .entry("enabledPlugins")
        .or_insert_with(|| json!({}));

    if currently_enabled {
        if let Some(obj) = plugins.as_object_mut() {
            obj.remove(plugin_id);
        }
    } else {
        if let Some(obj) = plugins.as_object_mut() {
            obj.insert(plugin_id.to_string(), json!(true));
        }
    }

    // Ensure .claude dir exists
    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(&settings_path, content)
}
```

- [ ] **Step 4: Run tests**

```bash
cargo test --test unit -- actions_test
```

Expected: all 3 PASS.

- [ ] **Step 5: Commit**

```bash
git add claude-env/src/tui/actions.rs claude-env/tests/unit/actions_test.rs claude-env/tests/unit.rs
git commit -m "feat(claude-env): implement toggle plugin action with settings.json write"
```

---

## Task 8: Manual Testing + Polish

**Files:**
- Various small fixes across tui/ files

- [ ] **Step 1: Build release and test manually**

```bash
cargo build --release && ./target/release/claude-env inspect --tui
```

Test:
- Arrow keys navigate the tree
- Enter/←/→ expand/collapse
- `e` on a plugin toggles it (check `~/.claude/settings.json` after)
- `/` enters search, type to filter, Esc clears
- `v` on a skill opens markdown view, `q` closes
- `q` quits

- [ ] **Step 2: Fix any issues found during manual testing**

- [ ] **Step 3: Run all tests**

```bash
cargo test
```

- [ ] **Step 4: Commit**

```bash
git add -A claude-env/src/tui/
git commit -m "fix(claude-env): polish TUI after manual testing"
```

---

## Summary

| Task | Delivers |
|------|----------|
| 1 | Dependencies + module skeleton + `--tui` flag |
| 2 | App state (navigation, expand/collapse, search filter) |
| 3 | Tree builder from AuditReport + tests |
| 4 | Terminal setup + event loop |
| 5 | ratatui rendering (tree panel, detail panel, status bar, markdown overlay) |
| 6 | Keyboard handler (all shortcuts) |
| 7 | Toggle plugin action + tests |
| 8 | Manual testing + polish |

After Task 4, you have a running (but empty) TUI. After Task 5, you can see the tree. After Task 6, you can interact. After Task 7, you can toggle plugins. Task 8 is verification.
