# claude-env inspect: Full Environment Audit Command

**Date:** 2026-04-20
**Status:** Approved

## Problem

Users have no single view of what Claude Code tooling is configured, where it came from, or whether it matches what `claude-env.toml` declares. Configuration is scattered across multiple files at both project and global levels. Drift between declared and actual state is invisible.

## Solution

A new `claude-env inspect` command that scans all Claude Code config locations (project + global), categorizes everything found, shows source file paths, and flags drift against `claude-env.toml`.

## Sources Scanned

| Category | Project-level | Global-level |
|----------|--------------|--------------|
| MCP servers | `.mcp.json` | `~/.claude/settings.json` (`mcpServers` key) |
| Plugins | `.claude/settings.json` | `~/.claude/settings.json` |
| Skills | `.claude/skills/*/SKILL.md` | `~/.claude/skills/*/SKILL.md` |
| Commands | `.claude/commands/**/*.md` | `~/.claude/commands/**/*.md` |
| Agents | `.claude/agents/*.md` | `~/.claude/agents/*.md` |

## Output Format

```
$ claude-env inspect

MCP Servers
  ✓ context7              2.1.4    project   managed (claude-env.toml)
    → .mcp.json
  ● sequential-thinking   —        global    manual
    → ~/.claude/settings.json
  ⚠ shadcn               0.2.1    MISSING   declared in claude-env.toml but not installed

Plugins
  ✓ code-review@claude-code-plugins     project   managed (claude-env.toml)
    → .claude/settings.json
  ● caveman@caveman                     global    manual
    → ~/.claude/settings.json

Skills
  ✓ next-best-practices                 project   managed (claude-env.toml)
    → .claude/skills/next-best-practices/SKILL.md
  ● web-design-guidelines               project   manual
    → .claude/skills/web-design-guidelines/SKILL.md

Commands
  ● commit                              project   manual
    → .claude/commands/commit.md
  ● review                              global    manual
    → ~/.claude/commands/review.md

Agents
  ● bmad-agent                          project   manual
    → .claude/agents/bmad-agent.md
```

Each item shows the file path it was discovered from. For MCP/plugins: the JSON config file. For skills/commands/agents: the actual markdown file.

## Drift Detection

Three states:

| State | Symbol | Meaning |
|-------|--------|---------|
| Declared + installed | `✓` (green) | Managed by claude-env, everything in sync |
| Installed, not declared | `●` (default) | Manually configured, informational |
| Declared, not installed | `⚠` (yellow) | Drift — declared in `claude-env.toml` but missing from actual config |

## Override Detection

When the same name exists at both project and global level, annotate the shadowed entry:

```
  ● sequential-thinking   —    global    manual
    → ~/.claude/settings.json
    └─ overridden by project .mcp.json
```

## CLI Flags

```
claude-env inspect                      # Full audit, all categories
claude-env inspect --section mcp        # Filter to one category
claude-env inspect --section plugins
claude-env inspect --json               # Machine-readable JSON output
```

`--section` accepts: `mcp`, `plugins`, `skills`, `commands`, `agents`.

## JSON Output Format

```json
{
  "mcp": [
    {
      "name": "context7",
      "version": "2.1.4",
      "scope": "project",
      "source": "managed",
      "path": ".mcp.json",
      "drift": false
    },
    {
      "name": "shadcn",
      "version": "0.2.1",
      "scope": "missing",
      "source": "managed",
      "path": null,
      "drift": true
    }
  ],
  "plugins": [...],
  "skills": [...],
  "commands": [...],
  "agents": [...]
}
```

## Implementation Architecture

### Scanner modules

One scanner per category, each returns `Vec<DiscoveredItem>`:

```rust
pub struct DiscoveredItem {
    pub name: String,
    pub version: Option<String>,
    pub scope: Scope,        // Project | Global
    pub source_path: String, // file where it was found
}

pub enum Scope {
    Project,
    Global,
}
```

Scanners:
- `scan_mcp(project_root, home_dir)` — parse `.mcp.json` + `~/.claude/settings.json` mcpServers
- `scan_plugins(project_root, home_dir)` — parse `.claude/settings.json` at both levels
- `scan_skills(project_root, home_dir)` — glob `*/SKILL.md` in skills dirs
- `scan_commands(project_root, home_dir)` — glob `**/*.md` in commands dirs
- `scan_agents(project_root, home_dir)` — glob `*.md` in agents dirs

### Reconciler

Takes discovered items + `claude-env.toml` config, produces `Vec<AuditEntry>`:

```rust
pub struct AuditEntry {
    pub name: String,
    pub version: Option<String>,
    pub scope: Scope,
    pub management: Management,  // Managed | Manual
    pub path: Option<String>,
    pub drift: bool,
    pub overridden_by: Option<String>,
}

pub enum Management {
    Managed,  // in claude-env.toml
    Manual,   // not in claude-env.toml
}
```

### Renderer

Formats `Vec<AuditEntry>` per category as colored terminal output or JSON.

## Testing Strategy

**Unit tests:**
- Scanner tests: create temp dirs with config files, verify correct items discovered
- Reconciler tests: given discovered items + config, verify correct drift/management/override tagging
- JSON serialization roundtrip

**Integration tests:**
- Full `claude-env inspect` with mock project + global dirs
- `--section` filtering
- `--json` output parsing
- Override detection when same tool at project + global

Use `CLAUDE_ENV_HOME` and `HOME` env vars in tests to point at temp dirs.

## Non-Goals

- Does not fix drift (that's `claude-env install`)
- Does not modify any config files
- Does not check versions against npm registry (that's `claude-env update`)
