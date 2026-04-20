# claude-env inspect Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a `claude-env inspect` command that audits all Claude Code configuration (project + global), shows source file paths, tags managed vs manual items, and flags drift against `claude-env.toml`.

**Architecture:** Scanner modules discover items from config files/directories, a reconciler cross-references against `claude-env.toml` to tag management status and drift, and a renderer formats output as colored terminal text or JSON. Each scanner is independent and testable in isolation.

**Tech Stack:** Rust, serde_json (parsing .mcp.json/.claude/settings.json), glob (directory scanning), clap (CLI), existing claude-env modules (config, registry)

---

## File Structure

```
claude-env/src/
├── inspect/
│   ├── mod.rs          # Public types (DiscoveredItem, AuditEntry, Scope, Management) + run_inspect orchestrator
│   ├── scanner.rs      # 5 scanner functions (mcp, plugins, skills, commands, agents)
│   ├── reconciler.rs   # Cross-reference discovered items with claude-env.toml
│   └── renderer.rs     # Terminal + JSON output formatting
├── cli.rs              # (modify) Add Inspect subcommand
├── main.rs             # (modify) Wire Inspect to run_inspect
└── lib.rs              # (modify) Add pub mod inspect

claude-env/tests/
├── unit/
│   ├── scanner_test.rs
│   └── reconciler_test.rs
├── unit.rs             # (modify) Add scanner_test, reconciler_test modules
├── integration/
│   └── inspect_test.rs
└── integration.rs      # (modify) Add inspect_test module
```

---

## Task 1: Core Types + CLI Wiring

**Files:**
- Create: `claude-env/src/inspect/mod.rs`
- Modify: `claude-env/src/cli.rs`
- Modify: `claude-env/src/main.rs`
- Modify: `claude-env/src/lib.rs`

- [ ] **Step 1: Create inspect/mod.rs with core types**

```rust
pub mod reconciler;
pub mod renderer;
pub mod scanner;

use std::path::Path;

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
```

- [ ] **Step 2: Create empty submodule files**

Create `claude-env/src/inspect/scanner.rs`:
```rust
// Scanner functions — implemented in Task 2
```

Create `claude-env/src/inspect/reconciler.rs`:
```rust
// Reconciler — implemented in Task 4
```

Create `claude-env/src/inspect/renderer.rs`:
```rust
// Renderer — implemented in Task 5
```

- [ ] **Step 3: Add Inspect subcommand to cli.rs**

Add to the `Command` enum in `claude-env/src/cli.rs`:

```rust
    /// Audit all Claude Code configuration (project + global).
    Inspect {
        /// Filter to a specific category (mcp, plugins, skills, commands, agents).
        #[arg(long)]
        section: Option<String>,

        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
```

- [ ] **Step 4: Wire into main.rs**

Add to the match in `main()`:

```rust
        Command::Inspect { section, json } => {
            println!("not yet implemented: inspect section={:?} json={}", section, json);
        }
```

- [ ] **Step 5: Add pub mod inspect to lib.rs**

Add `pub mod inspect;` to `claude-env/src/lib.rs`.

- [ ] **Step 6: Verify it compiles**

```bash
cd claude-env && cargo build
```

Expected: compiles with no errors.

- [ ] **Step 7: Verify --help shows inspect**

```bash
cd claude-env && cargo run -- --help
```

Expected: `inspect` appears in the commands list.

- [ ] **Step 8: Commit**

```bash
git add claude-env/src/inspect/ claude-env/src/cli.rs claude-env/src/main.rs claude-env/src/lib.rs
git commit -m "feat(claude-env): add inspect command skeleton with core types"
```

---

## Task 2: MCP + Plugin Scanners

**Files:**
- Modify: `claude-env/src/inspect/scanner.rs`
- Create: `claude-env/tests/unit/scanner_test.rs`
- Modify: `claude-env/tests/unit.rs`

- [ ] **Step 1: Write failing tests for MCP scanner**

Create `claude-env/tests/unit/scanner_test.rs`:

```rust
use claude_env::inspect::scanner;
use claude_env::inspect::Scope;
use tempfile::TempDir;
use std::fs;

#[test]
fn scan_mcp_from_project_mcp_json() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    fs::write(
        project.path().join(".mcp.json"),
        r#"{"mcpServers":{"context7-mcp":{"type":"stdio","command":"/path/to/bin","args":[]}}}"#,
    ).unwrap();

    let items = scanner::scan_mcp(project.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "context7-mcp");
    assert_eq!(items[0].scope, Scope::Project);
    assert_eq!(items[0].source_path, ".mcp.json");
}

#[test]
fn scan_mcp_from_global_settings() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    let claude_dir = home.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"mcpServers":{"global-mcp":{"type":"stdio","command":"cmd"}}}"#,
    ).unwrap();

    let items = scanner::scan_mcp(project.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "global-mcp");
    assert_eq!(items[0].scope, Scope::Global);
    assert!(items[0].source_path.contains(".claude/settings.json"));
}

#[test]
fn scan_mcp_both_scopes() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    fs::write(
        project.path().join(".mcp.json"),
        r#"{"mcpServers":{"shared":{"type":"stdio","command":"proj-cmd"}}}"#,
    ).unwrap();

    let claude_dir = home.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"mcpServers":{"shared":{"type":"stdio","command":"global-cmd"},"global-only":{"type":"stdio","command":"cmd"}}}"#,
    ).unwrap();

    let items = scanner::scan_mcp(project.path(), home.path());
    assert_eq!(items.len(), 3);
    let project_shared = items.iter().find(|i| i.name == "shared" && i.scope == Scope::Project);
    let global_shared = items.iter().find(|i| i.name == "shared" && i.scope == Scope::Global);
    let global_only = items.iter().find(|i| i.name == "global-only");
    assert!(project_shared.is_some());
    assert!(global_shared.is_some());
    assert!(global_only.is_some());
}

#[test]
fn scan_mcp_missing_files() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let items = scanner::scan_mcp(project.path(), home.path());
    assert!(items.is_empty());
}

#[test]
fn scan_plugins_from_project_settings() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    let claude_dir = project.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"enabledPlugins":{"code-review@claude-code-plugins":true,"feature-dev@claude-code-plugins":true}}"#,
    ).unwrap();

    let items = scanner::scan_plugins(project.path(), home.path());
    assert_eq!(items.len(), 2);
    assert!(items.iter().all(|i| i.scope == Scope::Project));
    assert!(items.iter().any(|i| i.name == "code-review@claude-code-plugins"));
}

#[test]
fn scan_plugins_from_global_settings() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    let claude_dir = home.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"enabledPlugins":{"superpowers@superpowers":true}}"#,
    ).unwrap();

    let items = scanner::scan_plugins(project.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "superpowers@superpowers");
    assert_eq!(items[0].scope, Scope::Global);
}
```

- [ ] **Step 2: Wire test module**

Add to `claude-env/tests/unit.rs`:
```rust
#[path = "unit/scanner_test.rs"]
mod scanner_test;
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd claude-env && cargo test --test unit -- scanner_test
```

Expected: compilation error — scanner functions don't exist.

- [ ] **Step 4: Implement MCP and plugin scanners**

Replace `claude-env/src/inspect/scanner.rs`:

```rust
use super::{DiscoveredItem, Scope};
use serde_json::Value;
use std::path::Path;

fn read_json(path: &Path) -> Option<Value> {
    let content = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&content).ok()
}

fn extract_mcp_servers(json: &Value, scope: Scope, source_path: &str) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();
    if let Some(servers) = json.get("mcpServers").and_then(|v| v.as_object()) {
        for name in servers.keys() {
            items.push(DiscoveredItem {
                name: name.clone(),
                version: None,
                scope: scope.clone(),
                source_path: source_path.to_string(),
            });
        }
    }
    items
}

pub fn scan_mcp(project_root: &Path, home_dir: &Path) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();

    // Project: .mcp.json
    let project_mcp = project_root.join(".mcp.json");
    if let Some(json) = read_json(&project_mcp) {
        items.extend(extract_mcp_servers(&json, Scope::Project, ".mcp.json"));
    }

    // Global: ~/.claude/settings.json
    let global_settings = home_dir.join(".claude").join("settings.json");
    if let Some(json) = read_json(&global_settings) {
        let source = format!("~/.claude/settings.json");
        items.extend(extract_mcp_servers(&json, Scope::Global, &source));
    }

    items
}

fn extract_plugins(json: &Value, scope: Scope, source_path: &str) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();
    if let Some(plugins) = json.get("enabledPlugins").and_then(|v| v.as_object()) {
        for name in plugins.keys() {
            items.push(DiscoveredItem {
                name: name.clone(),
                version: None,
                scope: scope.clone(),
                source_path: source_path.to_string(),
            });
        }
    }
    items
}

pub fn scan_plugins(project_root: &Path, home_dir: &Path) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();

    // Project: .claude/settings.json
    let project_settings = project_root.join(".claude").join("settings.json");
    if let Some(json) = read_json(&project_settings) {
        items.extend(extract_plugins(&json, Scope::Project, ".claude/settings.json"));
    }

    // Global: ~/.claude/settings.json
    let global_settings = home_dir.join(".claude").join("settings.json");
    if let Some(json) = read_json(&global_settings) {
        items.extend(extract_plugins(&json, Scope::Global, "~/.claude/settings.json"));
    }

    items
}

pub fn scan_skills(project_root: &Path, home_dir: &Path) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();
    scan_md_dirs(
        &project_root.join(".claude").join("skills"),
        Scope::Project,
        ".claude/skills",
        "SKILL.md",
        &mut items,
    );
    scan_md_dirs(
        &home_dir.join(".claude").join("skills"),
        Scope::Global,
        "~/.claude/skills",
        "SKILL.md",
        &mut items,
    );
    items
}

pub fn scan_commands(project_root: &Path, home_dir: &Path) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();
    scan_md_files_recursive(
        &project_root.join(".claude").join("commands"),
        Scope::Project,
        ".claude/commands",
        &mut items,
    );
    scan_md_files_recursive(
        &home_dir.join(".claude").join("commands"),
        Scope::Global,
        "~/.claude/commands",
        &mut items,
    );
    items
}

pub fn scan_agents(project_root: &Path, home_dir: &Path) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();
    scan_md_files_flat(
        &project_root.join(".claude").join("agents"),
        Scope::Project,
        ".claude/agents",
        &mut items,
    );
    scan_md_files_flat(
        &home_dir.join(".claude").join("agents"),
        Scope::Global,
        "~/.claude/agents",
        &mut items,
    );
    items
}

/// Scan for subdirs containing a specific marker file (e.g., skills/*/SKILL.md)
fn scan_md_dirs(
    dir: &Path,
    scope: Scope,
    display_prefix: &str,
    marker: &str,
    items: &mut Vec<DiscoveredItem>,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        if entry.path().is_dir() {
            let marker_path = entry.path().join(marker);
            if marker_path.exists() {
                let name = entry.file_name().to_string_lossy().to_string();
                let source = format!("{}/{}/{}", display_prefix, name, marker);
                items.push(DiscoveredItem {
                    name,
                    version: None,
                    scope: scope.clone(),
                    source_path: source,
                });
            }
        }
    }
}

/// Scan for .md files recursively (e.g., commands/**/*.md)
fn scan_md_files_recursive(
    dir: &Path,
    scope: Scope,
    display_prefix: &str,
    items: &mut Vec<DiscoveredItem>,
) {
    scan_md_files_inner(dir, dir, scope, display_prefix, items, true);
}

/// Scan for .md files in top-level only (e.g., agents/*.md)
fn scan_md_files_flat(
    dir: &Path,
    scope: Scope,
    display_prefix: &str,
    items: &mut Vec<DiscoveredItem>,
) {
    scan_md_files_inner(dir, dir, scope, display_prefix, items, false);
}

fn scan_md_files_inner(
    base: &Path,
    dir: &Path,
    scope: Scope,
    display_prefix: &str,
    items: &mut Vec<DiscoveredItem>,
    recursive: bool,
) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |e| e == "md") {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let relative = path.strip_prefix(base).unwrap_or(&path);
            let source = format!("{}/{}", display_prefix, relative.display());
            items.push(DiscoveredItem {
                name,
                version: None,
                scope: scope.clone(),
                source_path: source,
            });
        }
        if recursive && path.is_dir() {
            scan_md_files_inner(base, &path, scope.clone(), display_prefix, items, true);
        }
    }
}
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd claude-env && cargo test --test unit -- scanner_test
```

Expected: all 7 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add claude-env/src/inspect/scanner.rs claude-env/tests/unit/scanner_test.rs claude-env/tests/unit.rs
git commit -m "feat(claude-env): implement MCP and plugin scanners for inspect"
```

---

## Task 3: Skills, Commands, and Agents Scanners (Tests)

**Files:**
- Modify: `claude-env/tests/unit/scanner_test.rs` (add more tests)

- [ ] **Step 1: Add scanner tests for skills, commands, agents**

Append to `claude-env/tests/unit/scanner_test.rs`:

```rust
#[test]
fn scan_skills_from_project() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    let skill_dir = project.path().join(".claude/skills/next-best-practices");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "# Next Best Practices").unwrap();

    let items = scanner::scan_skills(project.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "next-best-practices");
    assert_eq!(items[0].scope, Scope::Project);
    assert!(items[0].source_path.contains("SKILL.md"));
}

#[test]
fn scan_skills_ignores_dirs_without_marker() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    let skill_dir = project.path().join(".claude/skills/incomplete");
    fs::create_dir_all(&skill_dir).unwrap();
    // No SKILL.md inside

    let items = scanner::scan_skills(project.path(), home.path());
    assert!(items.is_empty());
}

#[test]
fn scan_commands_recursive() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    let cmd_dir = project.path().join(".claude/commands/gsd");
    fs::create_dir_all(&cmd_dir).unwrap();
    fs::write(cmd_dir.join("plan.md"), "# Plan command").unwrap();
    fs::write(
        project.path().join(".claude/commands/review.md"),
        "# Review command",
    ).unwrap();

    let items = scanner::scan_commands(project.path(), home.path());
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.name == "plan"));
    assert!(items.iter().any(|i| i.name == "review"));
}

#[test]
fn scan_agents_flat_only() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    let agents_dir = project.path().join(".claude/agents");
    fs::create_dir_all(&agents_dir).unwrap();
    fs::write(agents_dir.join("bmad-agent.md"), "# BMAD Agent").unwrap();

    // Nested dir should be ignored
    let nested = agents_dir.join("subdir");
    fs::create_dir_all(&nested).unwrap();
    fs::write(nested.join("hidden.md"), "# Hidden").unwrap();

    let items = scanner::scan_agents(project.path(), home.path());
    assert_eq!(items.len(), 1);
    assert_eq!(items[0].name, "bmad-agent");
}

#[test]
fn scan_skills_both_scopes() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    let proj_skill = project.path().join(".claude/skills/local-skill");
    fs::create_dir_all(&proj_skill).unwrap();
    fs::write(proj_skill.join("SKILL.md"), "# Local").unwrap();

    let global_skill = home.path().join(".claude/skills/global-skill");
    fs::create_dir_all(&global_skill).unwrap();
    fs::write(global_skill.join("SKILL.md"), "# Global").unwrap();

    let items = scanner::scan_skills(project.path(), home.path());
    assert_eq!(items.len(), 2);
    assert!(items.iter().any(|i| i.name == "local-skill" && i.scope == Scope::Project));
    assert!(items.iter().any(|i| i.name == "global-skill" && i.scope == Scope::Global));
}
```

- [ ] **Step 2: Run tests**

```bash
cd claude-env && cargo test --test unit -- scanner_test
```

Expected: all 12 tests PASS (scanners already implemented in Task 2).

- [ ] **Step 3: Commit**

```bash
git add claude-env/tests/unit/scanner_test.rs
git commit -m "test(claude-env): add scanner tests for skills, commands, and agents"
```

---

## Task 4: Reconciler

**Files:**
- Modify: `claude-env/src/inspect/reconciler.rs`
- Create: `claude-env/tests/unit/reconciler_test.rs`
- Modify: `claude-env/tests/unit.rs`

- [ ] **Step 1: Write failing tests**

Create `claude-env/tests/unit/reconciler_test.rs`:

```rust
use claude_env::config::Config;
use claude_env::inspect::reconciler::reconcile;
use claude_env::inspect::{Category, DiscoveredItem, Management, Scope};

fn item(name: &str, scope: Scope, path: &str) -> DiscoveredItem {
    DiscoveredItem {
        name: name.to_string(),
        version: None,
        scope,
        source_path: path.to_string(),
    }
}

#[test]
fn managed_item_matched_in_config() {
    let config = Config::parse("[mcp]\ncontext7 = \"2.1.4\"").unwrap();
    let discovered = vec![item("context7-mcp", Scope::Project, ".mcp.json")];

    let entries = reconcile(Category::Mcp, &discovered, &config);
    // context7-mcp is discovered, and "context7" is in config — the reconciler
    // should match via alias resolution (context7 → @upstash/context7-mcp → binary name context7-mcp)
    let managed = entries.iter().find(|e| e.name == "context7-mcp");
    assert!(managed.is_some());
    assert_eq!(managed.unwrap().management, Management::Managed);
    assert!(!managed.unwrap().drift);
}

#[test]
fn manual_item_not_in_config() {
    let config = Config::parse("").unwrap();
    let discovered = vec![item("random-mcp", Scope::Project, ".mcp.json")];

    let entries = reconcile(Category::Mcp, &discovered, &config);
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].management, Management::Manual);
    assert!(!entries[0].drift);
}

#[test]
fn drift_declared_but_not_discovered() {
    let config = Config::parse("[mcp]\nshadcn = \"0.2.1\"").unwrap();
    let discovered: Vec<DiscoveredItem> = vec![];

    let entries = reconcile(Category::Mcp, &discovered, &config);
    let drifted = entries.iter().find(|e| e.drift);
    assert!(drifted.is_some());
    assert_eq!(drifted.unwrap().name, "shadcn");
    assert_eq!(drifted.unwrap().management, Management::Managed);
}

#[test]
fn override_detected_same_name_both_scopes() {
    let config = Config::parse("").unwrap();
    let discovered = vec![
        item("shared-mcp", Scope::Project, ".mcp.json"),
        item("shared-mcp", Scope::Global, "~/.claude/settings.json"),
    ];

    let entries = reconcile(Category::Mcp, &discovered, &config);
    let global = entries.iter().find(|e| e.name == "shared-mcp" && e.scope == Some(Scope::Global));
    assert!(global.is_some());
    assert!(global.unwrap().overridden_by.is_some());
}

#[test]
fn plugin_reconciliation() {
    let config = Config::parse(
        "[plugins]\n\"anthropics/claude-code/code-review@claude-code-plugins\" = \"latest\""
    ).unwrap();
    let discovered = vec![item("code-review@claude-code-plugins", Scope::Project, ".claude/settings.json")];

    let entries = reconcile(Category::Plugins, &discovered, &config);
    let managed = entries.iter().find(|e| e.name == "code-review@claude-code-plugins");
    assert!(managed.is_some());
    assert_eq!(managed.unwrap().management, Management::Managed);
}

#[test]
fn skills_reconciliation() {
    let config = Config::parse(
        "[skills]\n\"vercel-labs/next-skills/next-best-practices\" = \"latest\""
    ).unwrap();
    let discovered = vec![item("next-best-practices", Scope::Project, ".claude/skills/next-best-practices/SKILL.md")];

    let entries = reconcile(Category::Skills, &discovered, &config);
    let managed = entries.iter().find(|e| e.name == "next-best-practices");
    assert!(managed.is_some());
    assert_eq!(managed.unwrap().management, Management::Managed);
}
```

- [ ] **Step 2: Wire test module**

Add to `claude-env/tests/unit.rs`:
```rust
#[path = "unit/reconciler_test.rs"]
mod reconciler_test;
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd claude-env && cargo test --test unit -- reconciler_test
```

Expected: compilation error.

- [ ] **Step 4: Implement reconciler**

Replace `claude-env/src/inspect/reconciler.rs`:

```rust
use super::{AuditEntry, Category, DiscoveredItem, Management, Scope};
use crate::config::Config;
use crate::registry::Registry;
use std::collections::BTreeMap;

/// Cross-reference discovered items with claude-env.toml config.
///
/// For each category:
/// 1. Mark discovered items as Managed (matches config) or Manual (not in config)
/// 2. Add drift entries for config items not found in discovered
/// 3. Detect overrides (same name at project + global scope)
pub fn reconcile(
    category: Category,
    discovered: &[DiscoveredItem],
    config: &Config,
) -> Vec<AuditEntry> {
    let registry = Registry::default();

    // Build set of "expected names" from config for this category
    let config_names = config_names_for_category(&category, config, &registry);

    // Track which config names we've matched
    let mut matched_config: Vec<bool> = vec![false; config_names.len()];

    // Build entries from discovered items
    let mut entries: Vec<AuditEntry> = Vec::new();

    for item in discovered {
        let is_managed = config_names.iter().enumerate().any(|(i, (_, match_names))| {
            let matches = match_names.iter().any(|mn| *mn == item.name);
            if matches {
                matched_config[i] = true;
            }
            matches
        });

        entries.push(AuditEntry {
            name: item.name.clone(),
            version: item.version.clone(),
            scope: Some(item.scope.clone()),
            management: if is_managed {
                Management::Managed
            } else {
                Management::Manual
            },
            path: Some(item.source_path.clone()),
            drift: false,
            overridden_by: None,
        });
    }

    // Add drift entries for config items not found
    for (i, (config_name, _)) in config_names.iter().enumerate() {
        if !matched_config[i] {
            entries.push(AuditEntry {
                name: config_name.clone(),
                version: config.version_for(&category, config_name),
                scope: None,
                management: Management::Managed,
                path: None,
                drift: true,
                overridden_by: None,
            });
        }
    }

    // Detect overrides: same name at project and global
    let mut name_scopes: BTreeMap<String, Vec<Scope>> = BTreeMap::new();
    for entry in &entries {
        if let Some(ref scope) = entry.scope {
            name_scopes
                .entry(entry.name.clone())
                .or_default()
                .push(scope.clone());
        }
    }

    for entry in &mut entries {
        if entry.scope == Some(Scope::Global) {
            if let Some(scopes) = name_scopes.get(&entry.name) {
                if scopes.contains(&Scope::Project) {
                    entry.overridden_by = Some("project".to_string());
                }
            }
        }
    }

    entries
}

/// Build list of (config_key, possible_match_names) for a category.
///
/// For MCP: config key "context7" might match discovered name "context7-mcp"
/// (via alias → package name → binary name).
/// For plugins: config key "owner/repo/plugin@marketplace" matches "plugin@marketplace".
/// For skills: config key "owner/repo/skill" matches "skill" (last segment).
fn config_names_for_category(
    category: &Category,
    config: &Config,
    registry: &Registry,
) -> Vec<(String, Vec<String>)> {
    let tools: &BTreeMap<String, String> = match category {
        Category::Mcp => &config.mcp,
        Category::Plugins => &config.plugins,
        Category::Skills => &config.skills,
        Category::Commands => return vec![],
        Category::Agents => return vec![],
    };

    tools
        .keys()
        .map(|key| {
            let match_names = match category {
                Category::Mcp => {
                    let package = registry.resolve_alias(key);
                    // Possible binary names: full package, last segment, with -mcp suffix
                    let last_segment = package.split('/').last().unwrap_or(package);
                    let mut names = vec![
                        key.to_string(),
                        package.to_string(),
                        last_segment.to_string(),
                    ];
                    names.dedup();
                    names
                }
                Category::Plugins => {
                    // Config: "owner/repo/plugin@marketplace" → match: "plugin@marketplace"
                    let at_pos = key.rfind('@');
                    if let Some(at) = at_pos {
                        let path_part = &key[..at];
                        let marketplace = &key[at + 1..];
                        let plugin = path_part.split('/').last().unwrap_or(path_part);
                        vec![
                            key.to_string(),
                            format!("{}@{}", plugin, marketplace),
                        ]
                    } else {
                        vec![key.to_string()]
                    }
                }
                Category::Skills => {
                    // Config: "owner/repo/skill" → match: "skill" (last segment)
                    let last = key.split('/').last().unwrap_or(key);
                    vec![key.to_string(), last.to_string()]
                }
                _ => vec![key.to_string()],
            };
            (key.clone(), match_names)
        })
        .collect()
}
```

- [ ] **Step 5: Add version_for helper to Config**

Add to `claude-env/src/config.rs`:

```rust
use crate::inspect::Category;

impl Config {
    // ... existing methods ...

    pub fn version_for(&self, category: &Category, name: &str) -> Option<String> {
        let tools = match category {
            Category::Mcp => &self.mcp,
            Category::Plugins => &self.plugins,
            Category::Skills => &self.skills,
            _ => return None,
        };
        tools.get(name).cloned()
    }
}
```

Note: This introduces a circular dependency (config → inspect types). To avoid it, pass the version as a parameter to the reconciler or use a string for category. The simplest approach: add `version_for` taking a `&str` section name instead:

```rust
impl Config {
    pub fn version_for_section(&self, section: &str, name: &str) -> Option<String> {
        let tools = match section {
            "mcp" => &self.mcp,
            "cli" => &self.cli,
            "skills" => &self.skills,
            "plugins" => &self.plugins,
            _ => return None,
        };
        tools.get(name).cloned()
    }
}
```

Then in reconciler, use `config.version_for_section(category.cli_name(), config_name)`.

- [ ] **Step 6: Run tests**

```bash
cd claude-env && cargo test --test unit -- reconciler_test
```

Expected: all 6 tests PASS.

- [ ] **Step 7: Commit**

```bash
git add claude-env/src/inspect/reconciler.rs claude-env/src/config.rs claude-env/tests/unit/reconciler_test.rs claude-env/tests/unit.rs
git commit -m "feat(claude-env): implement reconciler for inspect drift and override detection"
```

---

## Task 5: Renderer (Terminal + JSON)

**Files:**
- Modify: `claude-env/src/inspect/renderer.rs`

- [ ] **Step 1: Implement terminal renderer**

Replace `claude-env/src/inspect/renderer.rs`:

```rust
use super::{AuditEntry, AuditReport, Category, Management, Scope};
use serde_json::{json, Value};

pub fn render_terminal(report: &AuditReport) {
    for (category, entries) in &report.entries {
        if entries.is_empty() {
            continue;
        }

        println!("\n{}", category.label());

        for entry in entries {
            let symbol = if entry.drift {
                "\x1b[33m⚠\x1b[0m"
            } else if entry.management == Management::Managed {
                "\x1b[32m✓\x1b[0m"
            } else {
                "●"
            };

            let version = entry.version.as_deref().unwrap_or("—");

            let scope_str = if entry.drift {
                "MISSING".to_string()
            } else {
                match &entry.scope {
                    Some(Scope::Project) => "project".to_string(),
                    Some(Scope::Global) => "global".to_string(),
                    None => "—".to_string(),
                }
            };

            let mgmt_str = match entry.management {
                Management::Managed => {
                    if entry.drift {
                        "declared in claude-env.toml but not installed".to_string()
                    } else {
                        "managed (claude-env.toml)".to_string()
                    }
                }
                Management::Manual => "manual".to_string(),
            };

            println!(
                "  {} {:<35} {:<8} {:<10} {}",
                symbol, entry.name, version, scope_str, mgmt_str
            );

            if let Some(ref path) = entry.path {
                println!("    → {}", path);
            }

            if let Some(ref overridden) = entry.overridden_by {
                println!("    └─ overridden by {} config", overridden);
            }
        }
    }
}

pub fn render_json(report: &AuditReport) {
    let mut result = json!({});

    for (category, entries) in &report.entries {
        let items: Vec<Value> = entries
            .iter()
            .map(|e| {
                json!({
                    "name": e.name,
                    "version": e.version,
                    "scope": match &e.scope {
                        Some(Scope::Project) => "project",
                        Some(Scope::Global) => "global",
                        None => if e.drift { "missing" } else { "unknown" },
                    },
                    "source": match e.management {
                        Management::Managed => "managed",
                        Management::Manual => "manual",
                    },
                    "path": e.path,
                    "drift": e.drift,
                    "overridden_by": e.overridden_by,
                })
            })
            .collect();

        result[category.cli_name()] = Value::Array(items);
    }

    println!("{}", serde_json::to_string_pretty(&result).unwrap());
}
```

- [ ] **Step 2: Verify it compiles**

```bash
cd claude-env && cargo build
```

- [ ] **Step 3: Commit**

```bash
git add claude-env/src/inspect/renderer.rs
git commit -m "feat(claude-env): implement terminal and JSON renderers for inspect"
```

---

## Task 6: Wire Everything Into run_inspect

**Files:**
- Modify: `claude-env/src/inspect/mod.rs`
- Modify: `claude-env/src/main.rs`

- [ ] **Step 1: Add run_inspect orchestrator to mod.rs**

Add to `claude-env/src/inspect/mod.rs`:

```rust
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

    let report = AuditReport {
        entries: report_entries,
    };

    if json_output {
        renderer::render_json(&report);
    } else {
        renderer::render_terminal(&report);
    }
}
```

- [ ] **Step 2: Wire into main.rs**

Replace the `Command::Inspect` stub in `main.rs`:

```rust
        Command::Inspect { section, json } => {
            let project_root = PathBuf::from(".");
            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));

            let config_path = PathBuf::from("claude-env.toml");
            let config = Config::from_file(&config_path).unwrap_or_default();

            claude_env::inspect::run_inspect(
                &project_root,
                &home_dir,
                &config,
                section.as_deref(),
                json,
            );
        }
```

- [ ] **Step 3: Verify it compiles and runs**

```bash
cd claude-env && cargo run -- inspect --help
```

Expected: shows inspect help with `--section` and `--json` flags.

- [ ] **Step 4: Commit**

```bash
git add claude-env/src/inspect/mod.rs claude-env/src/main.rs
git commit -m "feat(claude-env): wire inspect command with scanner → reconciler → renderer pipeline"
```

---

## Task 7: Integration Tests

**Files:**
- Create: `claude-env/tests/integration/inspect_test.rs`
- Modify: `claude-env/tests/integration.rs`

- [ ] **Step 1: Write integration tests**

Create `claude-env/tests/integration/inspect_test.rs`:

```rust
use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;

fn setup_project() -> (TempDir, TempDir) {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    // claude-env.toml with one managed mcp tool
    project
        .child("claude-env.toml")
        .write_str(
            r#"
[mcp]
context7 = "2.1.4"

[plugins]
"anthropics/claude-code/code-review@claude-code-plugins" = "latest"
"#,
        )
        .unwrap();

    // .mcp.json with managed + manual server
    project
        .child(".mcp.json")
        .write_str(
            r#"{"mcpServers":{"context7-mcp":{"type":"stdio","command":"cmd"},"manual-mcp":{"type":"stdio","command":"cmd2"}}}"#,
        )
        .unwrap();

    // .claude/settings.json with plugin
    fs::create_dir_all(project.path().join(".claude")).unwrap();
    fs::write(
        project.path().join(".claude/settings.json"),
        r#"{"enabledPlugins":{"code-review@claude-code-plugins":true}}"#,
    )
    .unwrap();

    // A skill
    let skill_dir = project.path().join(".claude/skills/my-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "# My Skill").unwrap();

    // A command
    let cmd_dir = project.path().join(".claude/commands");
    fs::create_dir_all(&cmd_dir).unwrap();
    fs::write(cmd_dir.join("review.md"), "# Review").unwrap();

    (project, home)
}

#[test]
fn inspect_shows_all_categories() {
    let (project, home) = setup_project();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("inspect")
        .current_dir(project.path())
        .env("HOME", home.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    assert!(stdout.contains("MCP Servers"));
    assert!(stdout.contains("context7-mcp"));
    assert!(stdout.contains("manual-mcp"));
    assert!(stdout.contains("manual"));
    assert!(stdout.contains("Plugins"));
    assert!(stdout.contains("code-review@claude-code-plugins"));
    assert!(stdout.contains("Skills"));
    assert!(stdout.contains("my-skill"));
    assert!(stdout.contains("Commands"));
    assert!(stdout.contains("review"));
}

#[test]
fn inspect_section_filter() {
    let (project, home) = setup_project();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.args(["inspect", "--section", "mcp"])
        .current_dir(project.path())
        .env("HOME", home.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    assert!(stdout.contains("MCP Servers"));
    assert!(!stdout.contains("Plugins"));
    assert!(!stdout.contains("Skills"));
}

#[test]
fn inspect_json_output() {
    let (project, home) = setup_project();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.args(["inspect", "--json"])
        .current_dir(project.path())
        .env("HOME", home.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    let json: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    assert!(json["mcp"].is_array());
    assert!(json["plugins"].is_array());
    assert!(json["skills"].is_array());
    assert!(json["commands"].is_array());
    assert!(json["agents"].is_array());

    let mcp = json["mcp"].as_array().unwrap();
    assert!(mcp.iter().any(|e| e["name"] == "context7-mcp" && e["source"] == "managed"));
    assert!(mcp.iter().any(|e| e["name"] == "manual-mcp" && e["source"] == "manual"));
}

#[test]
fn inspect_drift_shown_for_missing_tool() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();

    // Declared but not installed
    project
        .child("claude-env.toml")
        .write_str("[mcp]\nshadcn = \"0.2.1\"")
        .unwrap();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("inspect")
        .current_dir(project.path())
        .env("HOME", home.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    assert!(stdout.contains("shadcn"));
    assert!(stdout.contains("MISSING"));
}
```

- [ ] **Step 2: Wire test module**

Add to `claude-env/tests/integration.rs`:
```rust
#[path = "integration/inspect_test.rs"]
mod inspect_test;
```

- [ ] **Step 3: Run tests**

```bash
cd claude-env && cargo test --test integration -- inspect_test
```

Expected: all 4 tests PASS.

- [ ] **Step 4: Commit**

```bash
git add claude-env/tests/integration/inspect_test.rs claude-env/tests/integration.rs
git commit -m "test(claude-env): add integration tests for inspect command"
```

---

## Summary

| Task | What it delivers |
|------|-----------------|
| 1 | Core types + CLI wiring + empty submodules |
| 2 | MCP + Plugin scanners with tests |
| 3 | Skills, Commands, Agents scanner tests |
| 4 | Reconciler (drift detection, override detection, config matching) |
| 5 | Terminal + JSON renderers |
| 6 | Wire run_inspect orchestrator into main.rs |
| 7 | Integration tests (full inspect, section filter, JSON, drift) |

After Task 6, you have a working `claude-env inspect`. Task 7 is the verification layer.
