# claude-env inspect TUI: Interactive Environment Browser

**Date:** 2026-04-26
**Status:** Approved
**Phase:** 1 of 2

## Problem

The current `claude-env inspect` outputs a static text dump. With 280+ items across plugins, skills, commands, and agents, it's hard to browse, understand hierarchy, or take action. Users need an interactive way to explore their Claude Code environment and toggle plugins on/off.

## Solution

An interactive TUI (terminal UI) built with [ratatui](https://github.com/ratatui/ratatui) that replaces the static output when running `claude-env inspect --tui`. Tree view on the left showing plugin hierarchy, detail panel on the right showing metadata and markdown preview.

## Scope: Phase 1

**In scope (this spec):**
- Tree view: Plugins → Skills/Commands/Agents, MCP Servers
- Detail panel: metadata + markdown preview
- Toggle enable/disable plugin
- View full markdown content
- Search/filter by name
- Keyboard navigation

**Phase 2 (future):**
- Install missing / fix drift
- Remove tool/plugin
- Scope switch (project ↔ global)

## Layout

```
┌──────────────────────────────────┬────────────────────────────────────┐
│  ▼ Plugins (11 enabled, 47 cac) │  subagent-driven-development       │
│    ▼ ● superpowers               │                                    │
│        Skills:                   │  Type    Skill                     │
│          brainstorming           │  Plugin  superpowers@claude-plugi  │
│          writing-plans           │  Scope   global                    │
│          executing-plans         │  Status  ● enabled                 │
│        ▸ subagent-driven-dev     │  Path    ~/.claude/plugins/cache/  │
│          test-driven-dev         │                                    │
│        Commands:                 │  Preview:                          │
│          brainstorm              │  # Subagent-Driven Development     │
│          write-plan              │                                    │
│        Agents:                   │  Execute plan by dispatching       │
│          code-reviewer           │  fresh subagent per task...        │
│    ▶ ● caveman                   │                                    │
│    ▶ ● feature-dev               │  ──────────────────────────────    │
│    ▶ ○ customer-support          │  [e] toggle  [v] view  [/] search │
│    ▶ ○ financial-analysis        │  [q] quit                         │
│  ▶ MCP Servers (0)              │                                    │
└──────────────────────────────────┴────────────────────────────────────┘
```

## Tree Structure

The tree is built from the existing `AuditReport` data:

```
▼ Plugins (N enabled, M cached)
  ▼ ● superpowers @claude-plugins-official
      Skills:
        brainstorming
        writing-plans
        ...
      Commands:
        brainstorm
        write-plan
        ...
      Agents:
        code-reviewer
  ▶ ● caveman @caveman
  ▶ ○ customer-support @knowledge-work-plugins
▼ MCP Servers (N)
  ● context7-mcp (project)
  ○ sequential-thinking (global)
▼ Standalone Skills (N)        ← skills not from plugins
▼ Standalone Commands (N)      ← commands not from plugins
```

**Legend:**
- `●` green = enabled plugin (or item from enabled plugin)
- `○` dim = cached but not enabled
- `⚠` yellow = drift (declared in claude-env.toml but missing)

**Grouping logic:** Items with `from_plugin` set are grouped under their parent plugin node. Items without `from_plugin` (standalone, from `.claude/skills/`, `.claude/commands/`, etc.) appear in separate "Standalone" sections.

## Detail Panel

When an item is selected, the right panel shows:

**For a plugin node:**
| Field | Value |
|-------|-------|
| Name | `superpowers@claude-plugins-official` |
| Scope | global |
| Status | ● enabled / ○ cached |
| Contents | 14 skills, 3 commands, 1 agent |

**For a skill/command/agent:**
| Field | Value |
|-------|-------|
| Name | `subagent-driven-development` |
| Type | Skill |
| Plugin | `superpowers@claude-plugins-official` |
| Scope | global |
| Status | ● enabled |
| Path | `~/.claude/plugins/cache/.../SKILL.md` |
| Preview | First ~20 lines of the markdown file |

## Keyboard Shortcuts

| Key | Action |
|-----|--------|
| `↑` / `k` | Move up in tree |
| `↓` / `j` | Move down in tree |
| `←` / `h` | Collapse node |
| `→` / `l` | Expand node |
| `Enter` | Expand/collapse toggle |
| `e` | Toggle enable/disable (plugin nodes only) |
| `v` | View full markdown in scrollable overlay |
| `/` | Enter search mode (filter tree by name) |
| `Esc` | Exit search / close overlay / quit |
| `Tab` | Switch focus: tree ↔ detail panel |
| `q` | Quit |

## Enable/Disable Action

When user presses `e` on a plugin node:
1. Read `~/.claude/settings.json` (or `.claude/settings.json` for project-scoped)
2. Toggle the plugin key in `enabledPlugins`
3. Write back the file
4. Update the in-memory tree (re-color the node and its children)
5. Show confirmation in status bar: "Disabled superpowers@claude-plugins-official"

Only works on plugin nodes, not individual skills/commands/agents.

## Search/Filter

When user presses `/`:
1. Show search input in status bar at bottom
2. As user types, filter the tree to show only matching items (and their parent nodes)
3. `Enter` to confirm filter, `Esc` to clear and show all
4. Search matches against item names (case-insensitive substring)

## View Full Markdown

When user presses `v` on an item with a source file:
1. Read the markdown file from `path`
2. Show in a full-screen scrollable overlay (dimmed background)
3. `↑`/`↓`/`j`/`k`/`PgUp`/`PgDn` to scroll
4. `q`/`Esc` to close overlay

Render as plain text (no markdown rendering needed — just syntax-highlighted raw content).

## CLI Integration

```
claude-env inspect              # existing: static text output
claude-env inspect --tui        # new: interactive TUI
claude-env inspect --json       # existing: JSON output
```

The `--tui` flag launches the interactive mode. The existing static and JSON outputs are unchanged.

## Architecture

```
src/
├── tui/
│   ├── mod.rs          # run_tui() entry point, event loop
│   ├── app.rs          # App state (tree, selection, mode, search)
│   ├── tree.rs         # TreeNode structure, build from AuditReport
│   ├── ui.rs           # ratatui widget rendering (layout, tree, detail)
│   ├── handler.rs      # Keyboard event handling → state mutations
│   └── actions.rs      # Side effects (toggle plugin, read markdown)
```

**Dependencies:**
```toml
ratatui = "0.29"
crossterm = "0.28"
```

**Data flow:**
1. `run_tui()` calls existing `scanner` + `reconciler` to build `AuditReport`
2. `tree.rs` converts `AuditReport` → `Vec<TreeNode>` (hierarchical)
3. Event loop: `crossterm` events → `handler.rs` → mutate `App` state → `ui.rs` renders

**App state:**
```rust
struct App {
    tree: Vec<TreeNode>,
    flat_visible: Vec<FlatIndex>,  // flattened visible nodes for rendering
    selected: usize,               // index into flat_visible
    mode: Mode,                    // Normal | Search | ViewMarkdown
    search_query: String,
    detail_scroll: u16,
    markdown_content: Option<String>,
    markdown_scroll: u16,
    status_message: Option<String>,
}

enum Mode {
    Normal,
    Search,
    ViewMarkdown,
}

struct TreeNode {
    name: String,
    kind: NodeKind,    // Plugin | Skill | Command | Agent | McpServer | SectionHeader
    enabled: bool,
    scope: Option<Scope>,
    path: Option<String>,
    plugin_id: Option<String>,
    children: Vec<TreeNode>,
    expanded: bool,
}
```

## Testing Strategy

**Unit tests:**
- `tree.rs`: build tree from sample AuditReport, verify structure, expand/collapse, search filter
- `handler.rs`: given App state + key event, verify state mutation
- `actions.rs`: toggle plugin in temp settings.json, verify file updated

**Manual testing:**
- Run `claude-env inspect --tui` on real environment
- Verify navigation, expand/collapse, search, enable/disable, view markdown

No integration tests for the TUI rendering itself (ratatui widgets are hard to test automatically — visual verification is more practical).

## Non-Goals

- No markdown rendering (bold, headers, etc.) — raw text is fine for v1
- No mouse support — keyboard only
- No live file watching — tree is built once at startup
- No multi-select — one item at a time
- No color themes — single dark theme
