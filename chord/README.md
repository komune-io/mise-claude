# chord

Declarative agent-tool environment manager. Declare your MCP servers, skills, plugins, and CLI tools in one file — `chord install` handles the rest.

## Install

```bash
cargo install rytmyk-chord
```

The cargo package is `rytmyk-chord`; the installed binary is `chord`.

## Quick Start

Create `chord.toml` in your project root:

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
chord install
```

## Commands

| Command | Description |
|---------|-------------|
| `chord install` | Install from lockfile (or resolve + create lockfile) |
| `chord update` | Check for updates, show changelogs |
| `chord update <tool>` | Update a single tool |
| `chord diff <tool>` | Show changelog between versions |
| `chord list` | Show installed tools and status |
| `chord add <tool>` | Add a tool to config |
| `chord remove <tool>` | Remove tool and clean up |

## How It Works

1. Reads `chord.toml` for declared tools
2. Compares against `chord.lock` to determine what needs installing
3. Installs each tool sequentially (no concurrency issues)
4. Writes config files (`.mcp.json`, `.claude/settings.json`)
5. Updates `chord.lock` with resolved versions

Packages are cached globally at `~/.chord/packages/`.
