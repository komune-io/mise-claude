# Repo Cleanup Design

**Date:** 2026-05-09
**Branch:** `feat/claude-env-design`
**Scope:** Post-merge hygiene. Six independent file-level changes. No behavior changes.

## Background

After merging the standalone `claude-env` Rust CLI into the `mise-claude` Lua backend plugin (PR #1), several files drifted from the new architecture or were left behind:

- Root `CLAUDE.md` describes the pre-merge plugin (npm-installing MCP servers, `TOOL_REGISTRY`, `lib/aliases.lua`) — none of which exists anymore.
- `.github/security-scan-instructions.md` references the same dead code paths.
- `claude-env/tests/e2e/` was the standalone repo's smoke harness; superseded by the root `Dockerfile.test` + `test/integration.sh` setup.
- `.github/VOUCHED.td` has a typo extension (`.td` should be `.md`).
- `.superpowers/` (brainstorm session state) is untracked and not in `.gitignore`.
- `GITHUB.md` is an untracked working-tree doc; will be removed (not committed).

## Goals

- Documentation matches what the code actually does.
- Repo contains no dead test infrastructure.
- `git status` is clean on a fresh checkout (no spurious untracked files).

## Non-Goals

- No code changes to Lua hooks, claude-env Rust source, samples, or CI workflows beyond renaming one file path.
- No reorganization of `docs/superpowers/` history (old plans/specs kept as-is).
- No changes to `.gitignore` rules other than adding `.superpowers/`.

## Changes

### 1. Rewrite `CLAUDE.md` (root)

Replace stale content. New structure:

- **What This Is** — A mise backend plugin (Lua) that bootstraps the `claude-env` Rust CLI via `cargo install`. All Claude Code tool management (MCP servers, skills, plugins, CLI tools) is delegated to `claude-env`.
- **Local Development** — `mise plugin link claude ./` then `mise install` then `claude-env install` to install tools declared in `claude-env.toml`.
- **Architecture** — Three Lua hooks in `hooks/`:
  - `backend_list_versions.lua` — fetches non-yanked `claude-env` versions from crates.io.
  - `backend_install.lua` — runs `cargo install claude-env --version <v> --root <path> --locked` and writes a `.installed` sentinel.
  - `backend_exec_env.lua` — adds `<install_path>/bin` to `PATH`; if `claude-env.toml` exists in `pwd`, triggers `claude-env install --idempotent --quiet`.

  `metadata.lua` declares the plugin name, version, and metadata for mise.
- **Code Conventions** — Lua with LDoc-style annotations (`--- @param`, `--- @return`); format with StyLua (config in `stylua.toml`).
- **Project Layout**:
  - Root: the mise backend plugin (`hooks/`, `metadata.lua`)
  - `claude-env/`: the Rust CLI (its own `Cargo.toml`, `mise.toml`, `README.md`, tests)
  - `sample/`: usage examples (`mcp/`, `spec/`, `skillssh/`, `plugin/`) — each has a `.mise.toml`, a `claude-env.toml`, and a `test.sh`
  - `test/integration.sh` + `Dockerfile.test`: end-to-end harness that builds claude-env, walks every sample, and asserts artifacts

### 2. Rewrite `.github/security-scan-instructions.md`

New surface to flag during security review:

- **Bootstrap surface (Lua hooks):**
  - `backend_install.lua` interpolates the user-supplied version and `install_path` into a `cargo install` shell command. Verify `shell_quote()` is applied to every interpolated value.
  - `backend_exec_env.lua` interpolates `pwd` and the binary path into a shell command that runs `claude-env install`. Verify quoting; verify `pwd` cannot be a hostile value (it comes from the user's shell, not the config).
- **Delegated surface (claude-env binary):** out of scope for this plugin's scan; claude-env has its own security model and is audited separately.
- **Supply chain:** `cargo install --locked` pins the resolved dependency tree; verify the `--locked` flag is present in any future change to `backend_install.lua`.

### 3. Delete `claude-env/tests/e2e/`

`rm -rf claude-env/tests/e2e/`. Removes:

- `Dockerfile` — separate Rust build + Docker harness
- `docker-compose.yml` — separate compose entry
- `scenarios/run_all.sh`, `scenarios/mcp_install.sh` — single smoke test

Not invoked from `.github/workflows/`, root `mise.toml`, or `claude-env/mise.toml`. Superseded by `test/integration.sh` (which exercises the same MCP install path through `sample/mcp/`).

### 4. Rename `.github/VOUCHED.td` → `.github/VOUCHED.md`

- `git mv .github/VOUCHED.td .github/VOUCHED.md`
- Edit `.github/workflows/pr-vouch.yml` — replace `.github/VOUCHED.td` with `.github/VOUCHED.md` in the `paths:` trigger.

### 5. Add `.superpowers/` to `.gitignore`

Append `.superpowers/` line. Existing brainstorm session state on disk is unaffected.

### 6. Remove `GITHUB.md`

`rm GITHUB.md`. Untracked working-tree file; user opted not to commit it.

## Commit Plan

Six commits, one per change, on the existing `feat/claude-env-design` branch:

1. `docs(claude.md): rewrite to match current bootstrap-only architecture`
2. `docs(security): rewrite scan instructions for current attack surface`
3. `chore: remove unused claude-env/tests/e2e/ harness`
4. `chore: rename VOUCHED.td → VOUCHED.md and update workflow`
5. `chore: gitignore .superpowers/ brainstorm session state`
6. (no commit) — delete untracked `GITHUB.md` from working tree

## Verification

- After #1, #2 — visual review of each file.
- After #6 — `ls GITHUB.md` returns "No such file or directory".
- After #3 — `cd claude-env && cargo test` still passes.
- After #4 — `cat .github/workflows/pr-vouch.yml | grep VOUCHED` shows `.md`, not `.td`.
- After #5 — `git status` no longer reports `.superpowers/` as untracked.
- After all changes — `mise run test` reports 13/13 passing (same baseline as current branch tip).

## Risks

- Low across the board. Doc rewrites are reversible. The `e2e/` deletion has no callers. The rename is two-line. The gitignore change has no behavioral effect on tooling. The `GITHUB.md` removal only deletes an untracked file (nothing in git history is touched).
- One subtle risk: the `pr-vouch.yml` workflow trigger on `.github/VOUCHED.td` will not match the renamed file until the workflow file is updated in the same commit. Doing rename + workflow edit atomically (single commit) avoids any window where the trigger is broken.
