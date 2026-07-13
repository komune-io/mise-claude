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
`name = "version"` entries. A version string is an exact version (`"2.1.4"`),
`"latest"`, or `"*"` (both track the newest); `[skills]` entries may also pin to
a branch or tag. **Unknown top-level keys are rejected**: a typo'd table name is
an error, not a silent no-op.

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

Installing a server writes it into the project's `.mcp.json`.

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

Command-line binaries. Workflow tools (GSD, BMAD, OpenSpec) also live here and
run a setup step in your project when installed. Use a short alias or any npm
package name.

```toml
[cli]
gsd = "1.22.4"
ripgrep = "14.1"
```

| You write | What gets installed |
|-----------|---------------------|
| `gsd` | `get-shit-done-cc` |
| `bmad` | `bmad-method` |
| `openspec` | `@fission-ai/openspec` |
