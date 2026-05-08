# mise-claude

A plugin for [mise](https://mise.jdx.dev) that lets you set up your entire [Claude Code](https://docs.anthropic.com/en/docs/claude-code) tooling with a single command.

List the tools you want in a configuration file, run `mise install`, and everything is ready to use.

## Install

```bash
mise plugin install claude https://github.com/komune-io/mise-claude
```

## Quick start

**1. Bootstrap claude-env via mise:**
```toml
# .mise.toml
[tools]
claude = "latest"
```
```bash
mise install
```

**2. Declare Claude tools in `claude-env.toml`:**
```toml
[mcp]
context7 = "2.1.4"

[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"

[plugins]
"upstash/context7/context7-plugin@context7-marketplace" = "latest"

[cli]
gsd = "1.22.4"
```

**3. Install (auto-runs on shell entry after step 1):**
```bash
claude-env install
```

**4. Audit your environment:**
```bash
claude-env inspect
```

**Migrating from the old mise plugin?**
```bash
claude-env migrate   # reads .mise.toml claude:* entries → writes claude-env.toml
```

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

```toml
# claude-env.toml
[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"
"anthropics/skills/frontend-design" = "latest"
```

### Plugins

Native Claude Code plugins from GitHub-based marketplaces.

```toml
# claude-env.toml
[plugins]
"anthropics/claude-code/commit-commands@claude-code-plugins" = "latest"
"upstash/context7/context7-plugin@context7-marketplace" = "latest"
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

The plugin hooks into mise's install lifecycle to bootstrap the `claude-env` binary:

1. **List versions** — queries crates.io for available `claude-env` releases
2. **Install** — runs `cargo install claude-env` to put the binary in PATH
3. **Shell entry** — `claude-env install --idempotent --quiet` runs automatically when `claude-env.toml` exists in the project root, installing any missing or outdated tools silently

All Claude tool management (MCP servers, skills, plugins, CLI specs) is handled by `claude-env`, not by mise directly.

## Local Development

```bash
# Link the plugin locally
mise plugin link claude ./
mise install

# Declare tools in claude-env.toml, then install them
claude-env install

# Inspect current state
claude-env inspect
```

## Contributing

Contributions are not open at this time. This project is in early development and not yet accepting external pull requests.

## License

MIT
