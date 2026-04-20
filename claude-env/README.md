# claude-env

Declarative Claude Code environment manager. Declare your MCP servers, skills, plugins, and CLI tools in one file — `claude-env install` handles the rest.

## Install

```bash
cargo install claude-env
```

## Quick Start

Create `claude-env.toml` in your project root:

```toml
[mcp]
context7 = "2.1.4"

[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"

[plugins]
"anthropics/claude-code/code-review@claude-code-plugins" = "latest"

[cli]
get-shit-done-cc = "1.22.4"
```

Then run:

```bash
claude-env install
```

## Commands

| Command | Description |
|---------|-------------|
| `claude-env install` | Install from lockfile (or resolve + create lockfile) |
| `claude-env update` | Check for updates, show changelogs |
| `claude-env update <tool>` | Update a single tool |
| `claude-env diff <tool>` | Show changelog between versions |
| `claude-env list` | Show installed tools and status |
| `claude-env add <tool>` | Add a tool to config |
| `claude-env remove <tool>` | Remove tool and clean up |

## How It Works

1. Reads `claude-env.toml` for declared tools
2. Compares against `claude-env.lock` to determine what needs installing
3. Installs each tool sequentially (no concurrency issues)
4. Writes config files (`.mcp.json`, `.claude/settings.json`)
5. Updates `claude-env.lock` with resolved versions

Packages are cached globally at `~/.claude-env/packages/`.
