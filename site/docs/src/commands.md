# Commands

Every `chord` verb. The global flag `-v, --verbose` works on all of them.

## `install`

```bash
chord install [--idempotent] [--quiet]
```

Installs everything declared in `chord.toml`. Idempotent — already-installed
tools are skipped, so it's safe to run repeatedly.

- `--idempotent` — documents intent; installs are idempotent regardless.
- `--quiet` — suppress output when nothing changed (used by the mise shell hook).

## `inspect`

```bash
chord inspect [--section <category>] [--json] [--tui]
```

Audits **all** Claude Code configuration — project *and* global: MCP servers,
plugins, skills, commands, agents.

- `--section mcp|plugins|skills|commands|agents` — filter to one category.
- `--json` — machine-readable output.
- `--tui` — interactive terminal UI (see [TUI](./tui.md)).

## `list`

```bash
chord list
```

Lists the tools chord installed, from `chord.lock`.

## `add`

```bash
chord add <name@version>
```

Adds a declaration to `chord.toml`. Example: `chord add context7@latest`.

## `remove`

```bash
chord remove <name>
```

Removes a declaration from `chord.toml`. Example: `chord remove context7`.

## `update`

```bash
chord update [tool]
```

Updates installed tools to their latest matching version. Omit `tool` to update
everything; pass a name to update just one.

## `diff`

```bash
chord diff <tool>
```

Shows the changelog / diff for a tool between what's declared and what's
installed.

## `clean`

```bash
chord clean [-a | --all]
```

Removes everything chord installed (per `chord.lock`), keeping `chord.toml`.

- `--all` — **destructive** full project reset: also removes foreign artifacts
  and *all* Claude tool config in the project (`.agents/`, `skills-lock.json`,
  the whole `.claude/skills/`, `.mcp.json`, `.claude/settings.json`) — not just
  chord-owned state. This deletes config chord did not create.

## `migrate`

```bash
chord migrate
```

Migrates Claude tool declarations from `.mise.toml` into `chord.toml`. Reads the
current directory's `.mise.toml`, finds all `claude:mcp/*`, `claude:skills.sh/*`,
`claude:plugin/*`, and `claude:spec/*` entries, and writes an equivalent
`chord.toml`.
