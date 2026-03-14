# mise-claude

A plugin for [mise](https://mise.jdx.dev) that lets you set up your entire [Claude Code](https://docs.anthropic.com/en/docs/claude-code) tooling with a single command.

List the tools you want in a configuration file, run `mise install`, and everything is ready to use.

## Install

```bash
mise plugin install claude https://github.com/komune-io/mise-claude
```

## Quick Start

Add tools to your project's `.mise.toml` file:

```toml
[tools]
# MCP servers — give Claude Code extra capabilities (web search, browser access, UI components)
"claude:mcp/context7" = "2.1.4"
"claude:mcp/chrome-devtools" = "0.20.0"
"claude:mcp/shadcn" = "4.0.6"

# Workflow tools — add structured methodologies and commands to Claude Code
"claude:spec/gsd" = "1.22.4"
"claude:spec/bmad" = "6.1.0"
"claude:spec/openspec" = "1.2.0"

# Skills — teach Claude Code best practices for specific frameworks
"claude:skills.sh/vercel-labs/next-skills/next-best-practices" = "latest"
"claude:skills.sh/anthropics/skills/frontend-design" = "latest"

# Plugins — extend Claude Code with new commands
"claude:plugin/anthropics/claude-code/commit-commands@claude-code-plugins" = "latest"
```

Then install everything:

```bash
mise install
```

That's it. Claude Code will automatically pick up all the tools you've configured.

## What Can You Install?

### MCP Servers

MCP servers extend what Claude Code can do — browse the web, access documentation, generate UI components, and more. The plugin handles all the setup automatically.

Short aliases are available for popular servers:

| You write | What gets installed |
|-----------|-------------------|
| `mcp/context7` | `@upstash/context7-mcp` |
| `mcp/chrome-devtools` | `chrome-devtools-mcp` |
| `mcp/shadcn` | `shadcn` |

You can also use any npm package name directly (e.g. `claude:@anthropic-ai/claude-code-mcp`).

### Workflow Tools

Workflow tools add structured methodologies, slash commands, and agents to Claude Code. They set themselves up in your project when installed.

| You write | What it does |
|-----------|-------------|
| `spec/gsd` | GSD — structured project execution workflow |
| `spec/bmad` | BMAD Method — product development agents and commands |
| `spec/openspec` | OpenSpec — API specification tool |

### Skills

Skills from [skills.sh](https://skills.sh) teach Claude Code best practices for specific frameworks and topics — no server required.

Format: `skills.sh/<owner>/<repo>/<skill>`

```toml
"claude:skills.sh/vercel-labs/next-skills/next-best-practices" = "latest"
"claude:skills.sh/anthropics/skills/frontend-design" = "latest"
```

### Plugins

Native Claude Code plugins from GitHub-based marketplaces.

Format: `plugin/<owner>/<repo>/<plugin>@<marketplace>`

```toml
"claude:plugin/anthropics/claude-code/commit-commands@claude-code-plugins" = "latest"
```

## Extra Configuration

To pass additional settings to MCP servers, create a `.mcp-config.toml` file in your project:

```toml
["@upstash/context7-mcp"]
args = ["--api-key", "${CONTEXT7_API_KEY}"]
env = { LOG_LEVEL = "debug" }
```

- `args` — extra arguments passed to the server
- `env` — environment variables for the server
- `${VAR}` references are replaced with values from your environment

## How It Works

The plugin hooks into mise's install lifecycle:

1. **List versions** — checks the npm registry for available versions
2. **Install** — downloads the package and either registers it as an MCP server or runs its setup command
3. **Configure PATH** — makes installed binaries available in your terminal

Design principles:
- **Fast startup** — binaries are installed directly, no wrapper scripts
- **Safe to re-run** — running `mise install` again merges new tools without overwriting existing ones
- **Convention over configuration** — sensible defaults, override only when needed

## Local Development

```bash
mise plugin link claude ./
mise install claude:@upstash/context7-mcp@latest
mise ls
```

## Contributing

Contributions are not open at this time. This project is in early development and not yet accepting external pull requests.

## License

MIT
