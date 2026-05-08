# Merge Design: mise-claude plugin + claude-env CLI

**Date:** 2026-05-08
**Branch:** feat/claude-env-design
**Status:** Approved

## Goal

Unify the mise-claude Lua plugin and the claude-env Rust CLI into one coherent system. Primary user moment: **audit/drift detection** — run one command and see exactly what's configured vs. declared, with no gaps.

## Decisions Made

| Question | Decision |
|---|---|
| Primary user moment | Audit/drift detection |
| Single source of truth | `claude-env.toml` (not `.mise.toml`) |
| Mise plugin role | Bootstrap only: install `claude-env` binary + trigger it on shell entry |
| New developer onboarding | Automatic: exec_env hook calls `claude-env install --idempotent` on shell entry |
| Distribution | `cargo install claude-env` (crates.io) |
| Migration strategy | Clean break + `claude-env migrate` command |
| Repo structure | Keep monorepo — Lua plugin at root, Rust CLI in `claude-env/` |

---

## Architecture

### Before

```
.mise.toml [tools]
  claude:mcp/context7 = "2.1.4"
  claude:skills.sh/… = "latest"
  claude:plugin/… = "latest"
         │
         ▼
  Lua hooks (~600 lines)
  backend_install.lua     — npm install, detect binary, write .mcp.json
  backend_exec_env.lua    — mcp_config, semaphores, post_install
  lib/aliases.lua         — friendly-name resolution
  lib/registry.lua        — per-package overrides
  lib/mcp_config.lua      — .mcp.json writes
         │
         ▼
  .mcp.json + .claude/settings.json
```

Problems: no audit, no lockfile, no drift detection. Duplicates registry/aliases already in claude-env Rust code.

### After

```
.mise.toml [tools]
  claude = "latest"           ← only this line remains
         │
         ▼
  Lua plugin (~50 lines)
  backend_install.lua         — cargo install claude-env
  backend_exec_env.lua        — claude-env install --idempotent --quiet
  backend_list_versions.lua   — crates.io version query
         │
         ▼ (puts claude-env binary in PATH, triggers it on shell entry)

claude-env.toml               ← single source of truth for Claude tools
  [mcp]
  context7 = "2.1.4"
  [skills]
  "vercel-labs/…" = "latest"
  [plugins]
  "foo/bar@mp" = "latest"
         │
         ▼
  claude-env CLI (Rust)
  install    — npm/npx/claude, writes .mcp.json + settings.json (idempotent)
  inspect    — audit: declared vs configured, drift detection, TUI
  migrate    — reads .mise.toml claude: entries → generates claude-env.toml  ← NEW
  update     — check for upgrades
  diff       — changelog between versions
  list       — installed tools + status
```

### Developer Experience

| Step | Command | What happens |
|---|---|---|
| 1. Bootstrap | `mise install` | `cargo install claude-env` → binary in PATH |
| 2. Auto-setup | `cd project/` (shell entry) | exec_env hook → `claude-env install --idempotent` |
| 3. Audit | `claude-env inspect` | Declared vs configured, drift highlighted |
| 4. Migrate | `claude-env migrate` | Reads old `.mise.toml` → writes `claude-env.toml` |

---

## Component Designs

### 1. Lua plugin (rewritten, ~50 lines total)

**`hooks/backend_install.lua`** (~15 lines)
- Resolve `"latest"` → actual version from crates.io
- Run `cargo install claude-env --version <version> --root <install_path>`
- Write `.installed` sentinel

**`hooks/backend_exec_env.lua`** (~10 lines)
- Add `<install_path>/bin` to PATH
- If `claude-env.toml` exists in CWD: run `claude-env install --idempotent --quiet`
- Guard prevents noise in non-Claude projects

**`hooks/backend_list_versions.lua`** (~15 lines)
- `GET https://crates.io/api/v1/crates/claude-env/versions`
- Filter yanked releases, sort semver descending, return JSON array

**Deleted entirely:**
- `lib/aliases.lua`
- `lib/registry.lua`
- `lib/mcp_config.lua`
- `lib/utils.lua`
- All npm install logic, .mcp.json write logic, skills/plugin routing, semaphore management

### 2. claude-env CLI enhancements

#### NEW: `claude-env migrate`

New subcommand. Reads `.mise.toml` from the current directory, finds all `claude:*` tool entries in `[tools]`, resolves aliases using the existing `registry.rs` alias table, and writes `claude-env.toml`.

```
$ claude-env migrate
✓ Found 3 claude: tools in .mise.toml
✓ Written claude-env.toml
→ Remove claude:mcp/*, claude:skills.sh/*, claude:plugin/* from .mise.toml
→ Keep `claude = "latest"` — that installs the claude-env binary itself
```

Input parsing: use the `toml` crate to read `.mise.toml`, iterate `[tools]` keys starting with `claude:`. Route by prefix: `claude:mcp/` → `[mcp]`, `claude:skills.sh/` → `[skills]`, `claude:plugin/` → `[plugins]`, `claude:spec/` → `[cli]`.

New file: `claude-env/src/migrate.rs`. Wire into `src/main.rs` subcommand enum.

#### ENHANCED: `claude-env install --idempotent --quiet`

- `--idempotent`: read lockfile, compare installed versions against declared versions, skip tools already at correct version and already configured in `.mcp.json`/`settings.json`. Only acts on what's missing or outdated.
- `--quiet`: suppress all output when nothing changed. Print only when work is actually done.

Fast path: lockfile read + `.mcp.json` key check. No npm calls if everything matches.

Flag parsing in `src/main.rs`; fast-path logic in `src/installer/` (shared across `mcp.rs`, `cli_tool.rs`, `skill.rs`, `plugin.rs`).

#### NEW: publish to crates.io

Add to `claude-env/Cargo.toml`:
```toml
[package]
name = "claude-env"
version = "0.1.0"
description = "Declarative Claude Code environment manager"
license = "MIT"
repository = "https://github.com/…/mise-claude"
```

CI: `cargo publish` on `v*` tag push.

### 3. Repo structure after

```
mise-claude/
├── hooks/
│   ├── backend_install.lua       # ~15 lines
│   ├── backend_exec_env.lua      # ~10 lines
│   └── backend_list_versions.lua # ~15 lines
├── metadata.lua                  # unchanged
├── mise.toml                     # unchanged
├── claude-env/
│   ├── src/
│   │   ├── migrate.rs            # NEW
│   │   ├── installer/            # ENHANCED: --idempotent --quiet flags
│   │   └── … (unchanged)
│   └── Cargo.toml                # add publish metadata
└── sample/                       # updated to use claude-env.toml
```

`lib/` is deleted entirely.

---

## Migration Path for Existing Users

After the new plugin ships, `claude:mcp/*`, `claude:skills.sh/*`, and `claude:plugin/*` entries in `.mise.toml` no longer work. Existing users:

1. Run `claude-env migrate` — generates `claude-env.toml` from their existing `.mise.toml` declarations
2. Remove the old `claude:mcp/*` / `claude:skills.sh/*` / `claude:plugin/*` lines from `.mise.toml`
3. Keep `claude = "latest"` in `.mise.toml` (installs the claude-env binary)
4. Run `claude-env install` to verify

---

## Build Sequence

Each step is independently shippable:

1. **Enhance `claude-env install`** — add `--idempotent` and `--quiet` flags (`src/install.rs`)
2. **Implement `claude-env migrate`** — new subcommand (`src/migrate.rs` + `src/main.rs`)
3. **Publish to crates.io** — add `Cargo.toml` metadata + CI workflow
4. **Rewrite Lua plugin** — strip all three hooks to ~50 lines total, delete `lib/`
5. **Update samples and README** — replace `.mise.toml` Claude tool examples with `claude-env.toml`

---

## What Stays Unchanged

- `claude-env inspect` (scanner, reconciler, renderer, TUI) — unchanged
- `claude-env update`, `diff`, `list` — unchanged
- `claude-env` install core logic (npm/npx/claude calls) — unchanged
- `claude-env` lockfile management — unchanged
- `registry.rs` + alias resolution in Rust — unchanged (migrate reuses it)
- `.mcp.json` and `settings.json` write logic — unchanged