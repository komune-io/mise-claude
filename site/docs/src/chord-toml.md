# chord.toml reference

A complete `chord.toml`:

```toml
[mcp]
context7 = "2.1.4"
chrome-devtools = "0.20.0"

[skills]
"anthropics/skills/frontend-design" = "latest"
"vercel-labs/next-skills/next-best-practices" = "latest"

[plugins]
"upstash/context7/context7-plugin@context7-marketplace" = "latest"

[cli]
gsd = "1.22.4"
ripgrep = "14.1"
```

`chord.toml` is the single declarative file chord reads. It has four top-level
tables — `[mcp]`, `[skills]`, `[plugins]`, `[cli]` — each a flat map of
`name = "version"` entries. Version strings are either an exact version
(`"2.1.4"`) or `"latest"`. **Unknown top-level keys are rejected**: a typo'd
table name is an error, not a silent no-op.

The rest of this page documents each table.

## `[mcp]` — MCP servers

MCP servers extend what your agent can do — browse the web, read docs, generate
UI. Use a short alias, or any npm package name directly.

```toml
[mcp]
context7 = "2.1.4"
chrome-devtools = "0.20.0"
shadcn = "4.0.6"
```

| You write | What gets installed |
|-----------|---------------------|
| `context7` | `@upstash/context7-mcp` |
| `chrome-devtools` | `chrome-devtools-mcp` |
| `shadcn` | `shadcn` |

### Extra MCP configuration

To pass extra settings to a server, add a `.mcp-config.toml` in your project:

```toml
["@upstash/context7-mcp"]
args = ["--api-key", "${CONTEXT7_API_KEY}"]
env = { LOG_LEVEL = "debug" }
```

- `args` — extra arguments passed to the server.
- `env` — environment variables for the server.
- `${VAR}` references are replaced from your environment.

## `[skills]` — agent skills

Skills from [skills.sh](https://skills.sh) teach Claude Code best practices for a
framework or topic. Declared by their full slug.

```toml
[skills]
"anthropics/skills/frontend-design" = "latest"
"vercel-labs/next-skills/next-best-practices" = "latest"
```

## `[plugins]` — Claude Code plugins

Native Claude Code plugins from GitHub-based marketplaces. The slug ends with
`@<marketplace>`.

```toml
[plugins]
"anthropics/claude-plugins-official/commit-commands@claude-plugins-official" = "latest"
"upstash/context7/context7-plugin@context7-marketplace" = "latest"
```

## `[cli]` — CLI tools

Command-line binaries and workflow tools (e.g. GSD, BMAD, OpenSpec) chord
resolves and installs.

```toml
[cli]
gsd = "1.22.4"
```
