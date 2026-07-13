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
chord add <section>:<name>[@<version>]
```

Writes the declaration to `chord.toml` **and installs it**. The section is
required — one of `mcp`, `cli`, `skills`, `plugins`. Omit `@<version>` and it
defaults to `latest`.

```bash
chord add mcp:context7@latest
chord add cli:ripgrep            # version defaults to "latest"
```

For `plugins` the name already contains an `@marketplace` qualifier, so a
version is only read when a second `@` is present.

## `remove`

```bash
chord remove <name>
```

Removes a tool by bare name — chord finds which section it's in — and **fully
uninstalls** it: the entry is dropped from `chord.toml`, and the lockfile,
`.mcp.json`, and installed files are cleaned up. Example: `chord remove context7`.

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
current directory's `.mise.toml` and maps each `claude:*` tool entry into the
matching table:

| `.mise.toml` key | chord.toml table |
|------------------|------------------|
| `claude:mcp/*` | `[mcp]` |
| `claude:skills.sh/*` | `[skills]` |
| `claude:plugin/*` | `[plugins]` |
| `claude:spec/*`, `claude:cli/*` | `[cli]` |
