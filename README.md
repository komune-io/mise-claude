# mise-claude

A [mise](https://mise.jdx.dev) backend plugin for managing your entire [Claude Code](https://docs.anthropic.com/en/docs/claude-code) ecosystem declaratively.

Declare MCP servers, workflow plugins, and CLI tools in `.mise.toml` — one `mise install` sets everything up.

## Install

```bash
mise plugin install claude https://github.com/komune-io/mise-claude
```

## Usage

Add tools to your `.mise.toml`:

```toml
[tools]
# MCP servers — automatically configured in .mcp.json
"claude:mcp/context7" = "latest"
"claude:mcp/chrome-devtools" = "latest"
"claude:mcp/shadcn" = "latest"

# Spec/CLI tools — installed via npm with post-install setup
"claude:spec/gsd" = "latest"
"claude:spec/bmad" = "latest"
"claude:spec/openspec" = "latest"

# Skills — installed from skills.sh marketplace
"claude:skills.sh/vercel-labs/next-skills/next-best-practices" = "latest"
"claude:skills.sh/anthropics/skills/frontend-design" = "latest"

# Plugins — installed from Claude Code plugin marketplace
"claude:plugin/anthropics/claude-code/commit-commands" = "latest"
```

Then install:

```bash
mise install
```

## Tool Types

### `mcp/<name>` — MCP Servers

MCP servers provide tools and resources to Claude Code via the Model Context Protocol. The plugin installs the npm package and registers the server in `.mcp.json`.

Friendly aliases resolve to npm packages (e.g. `mcp/context7` → `@upstash/context7-mcp`). Unaliased names pass through as-is — you can use any npm package directly (e.g. `claude:@anthropic-ai/claude-code-mcp`).

| Alias | npm Package |
|-------|-------------|
| `mcp/context7` | `@upstash/context7-mcp` |
| `mcp/chrome-devtools` | `chrome-devtools-mcp` |
| `mcp/shadcn` | `shadcn` |

Multiple `mise install` runs safely merge into the same `.mcp.json`.

### `spec/<name>` — Spec & CLI Tools

Spec tools are npm packages that extend Claude Code with slash commands, agents, or workflows. They run a post-install command to scaffold into your project and skip `.mcp.json`.

| Alias | npm Package | Description |
|-------|-------------|-------------|
| `spec/gsd` | `get-shit-done-cc` | GSD workflow plugin for structured project execution |
| `spec/bmad` | `bmad-method` | BMAD Method agents and commands for product development |
| `spec/openspec` | `@fission-ai/openspec` | OpenSpec CLI tool |

### `skills.sh/<owner>/<repo>/<skill>` — Skills

Skills from [skills.sh](https://skills.sh) are curated prompt-based capabilities installed via the skills CLI. They add specialized knowledge and instructions to Claude Code without requiring an MCP server.

Format: `skills.sh/<owner>/<repo>/<skill>`

```toml
"claude:skills.sh/vercel-labs/next-skills/next-best-practices" = "latest"
"claude:skills.sh/anthropics/skills/frontend-design" = "latest"
```

Under the hood, runs: `npx skills add <owner>/<repo> --skill <skill> -a claude-code -y`

### `plugin/<owner>/<repo>/<plugin>` — Claude Code Plugins

Native Claude Code plugins installed from GitHub-based marketplaces using the `claude` CLI.

Format: `plugin/<owner>/<repo>/<plugin>`

```toml
"claude:plugin/anthropics/claude-code/commit-commands" = "latest"
```

Under the hood, runs:
1. `claude plugin marketplace add <owner>/<repo>` — registers the marketplace
2. Parses `claude plugin marketplace list` to find the marketplace name for the repo
3. `claude plugin install <plugin>@<marketplace> --scope project`

## Extra Configuration

Create a `.mcp-config.toml` in your project root to pass additional args or env vars to MCP servers:

```toml
["@upstash/context7-mcp"]
args = ["--api-key", "${CONTEXT7_API_KEY}"]
env = { LOG_LEVEL = "debug" }

["chrome-devtools-mcp"]
# no extra config needed — omit or leave empty
```

- `args`: array of strings passed to the server command
- `env`: inline table of environment variables
- `${VAR}` references are resolved from your environment at install time

## Generated `.mcp.json`

The plugin generates a `.mcp.json` that Claude Code reads automatically:

```json
{
  "mcpServers": {
    "context7-mcp": {
      "type": "stdio",
      "command": "~/.local/share/mise/installs/claude-upstash-context7-mcp/1.0.0/node_modules/.bin/context7-mcp",
      "args": ["--api-key", "actual-key-value"],
      "env": {}
    }
  }
}
```

## How It Works

The plugin uses mise's Lua backend hook system:

| Hook | Purpose |
|------|---------|
| `backend_list_versions` | Queries the npm registry for available versions |
| `backend_install` | Runs `npm install`, detects the binary, updates `.mcp.json` or runs post-install |
| `backend_exec_env` | Adds `node_modules/.bin` to PATH |

Key design decisions:

- **No npx at runtime** — binaries are installed directly for faster startup
- **Merge strategy** — each install reads and merges `.mcp.json` so multiple servers coexist
- **Tool registry** — distinguishes MCP servers from CLI plugins, with optional post-install hooks
- **Server naming** — derived from the binary name in `node_modules/.bin/`

## Local Development

```bash
# Link the plugin locally
mise plugin link claude ./

# Test installation
mise install claude:@upstash/context7-mcp@latest

# Check the result
mise ls
cat .mcp.json
```

## License

MIT
