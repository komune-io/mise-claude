# chord

Declarative agent-tool environment manager. Declare your MCP servers, skills, plugins, and CLI tools in one file — `chord install` handles the rest.

`chord` is part of the [rytmyk](https://github.com/rytmyk) toolchain. The repo also ships a [mise](https://mise.jdx.dev) backend plugin that bootstraps the binary automatically.

## Install

Via mise (recommended):

```bash
mise plugin install chord https://github.com/komune-io/mise-claude
```

Then, in your project:

```toml
# .mise.toml
[tools]
chord = "latest"
```

```bash
mise install
```

Or directly via cargo:

```bash
cargo install rytmyk-chord
```

(The cargo package is `rytmyk-chord` — org-prefixed because the bare name `chord` is held by an unrelated dormant crate. The installed binary is just `chord`.)

## Quick start

Declare what you want in `chord.toml`:

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

Install:

```bash
chord install
```

Audit your environment:

```bash
chord inspect
```

When you `cd` into a directory containing `chord.toml`, the mise plugin runs `chord install --idempotent --quiet` automatically. Missing tools are installed silently; nothing happens if everything is up to date.

## What can you install?

### MCP servers

MCP servers extend what your agent can do — browse the web, access documentation, generate UI components, and more. Short aliases are available for popular servers:

| You write | What gets installed |
|-----------|--------------------|
| `mcp/context7` | `@upstash/context7-mcp` |
| `mcp/chrome-devtools` | `chrome-devtools-mcp` |
| `mcp/shadcn` | `shadcn` |

You can also use any npm package name directly.

### Workflow tools

Workflow tools add structured methodologies, slash commands, and agents to Claude Code. They set themselves up in your project when installed.

| You write | What it does |
|-----------|--------------|
| `spec/gsd` | GSD — structured project execution workflow |
| `spec/bmad` | BMAD Method — product development agents and commands |
| `spec/openspec` | OpenSpec — API specification tool |

### Skills

Skills from [skills.sh](https://skills.sh) teach Claude Code best practices for specific frameworks and topics — no server required.

```toml
# chord.toml
[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"
"anthropics/skills/frontend-design" = "latest"
```

### Plugins

Native Claude Code plugins from GitHub-based marketplaces.

```toml
# chord.toml
[plugins]
"anthropics/claude-plugins-official/commit-commands@claude-plugins-official" = "latest"
"upstash/context7/context7-plugin@context7-marketplace" = "latest"
```

## Extra configuration

To pass additional settings to MCP servers, create a `.mcp-config.toml` file in your project:

```toml
["@upstash/context7-mcp"]
args = ["--api-key", "${CONTEXT7_API_KEY}"]
env = { LOG_LEVEL = "debug" }
```

- `args` — extra arguments passed to the server
- `env` — environment variables for the server
- `${VAR}` references are replaced with values from your environment

## How it works

The mise plugin (`hooks/`, `metadata.lua`) does only two things:

1. **List/install** — Queries crates.io for `rytmyk-chord` versions and runs `cargo install rytmyk-chord --locked` when mise resolves the tool.
2. **Shell entry** — Adds the binary to `PATH` and runs `chord install --idempotent --quiet` automatically when `chord.toml` exists in the project root.

All actual tool management — MCP server installs, plugin marketplace fetches, skills setup, lockfile maintenance — lives in the `chord` Rust binary (`src/`).

## Local development

```bash
# Link the plugin locally
mise plugin link chord ./
mise install

# Declare tools in chord.toml, then install them
chord install

# Inspect current state
chord inspect
```

## Roadmap

`chord` will grow into an agent-agnostic package manager. Today it targets Claude Code; planned support includes Codex, OpenCode, and aider.

## Contributing

Contributions are not open at this time. This project is in early development and not yet accepting external pull requests.

## License

MIT
