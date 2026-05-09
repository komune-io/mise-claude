# Repo Cleanup Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Bring repo files into alignment with the post-merge architecture: rewrite drifted docs, delete unused test infrastructure, fix one filename typo, gitignore brainstorm session state, drop one untracked working-tree file.

**Architecture:** Six independent file-level changes on the existing `feat/claude-env-design` branch. No code behavior changes. Each task is one commit (except Task 6 which is a non-commit verification).

**Tech Stack:** Markdown, Lua plugin (`hooks/`, `metadata.lua`), Rust CLI (`claude-env/`), GitHub Actions YAML, `.gitignore`.

**Spec:** `docs/superpowers/specs/2026-05-09-repo-cleanup-design.md`

---

## File Map

- **Modify** `CLAUDE.md` — full rewrite (Task 1)
- **Modify** `.github/security-scan-instructions.md` — full rewrite (Task 2)
- **Delete** `claude-env/tests/e2e/` — directory removal, no callers (Task 3)
- **Rename** `.github/VOUCHED.td` → `.github/VOUCHED.md` (Task 4)
- **Modify** `.github/workflows/pr-vouch.yml` — one line in `paths:` trigger (Task 4, same commit)
- **Modify** `.gitignore` — append one line (Task 5)
- **Delete (already done)** `GITHUB.md` — verification only (Task 6)
- **Final** Run full integration suite (Task 7)

---

## Task 1: Rewrite root `CLAUDE.md`

**Files:**
- Modify: `CLAUDE.md` (replace entire contents)

**Why this rewrite:** The current file describes the pre-merge architecture (`lib/aliases.lua`, `TOOL_REGISTRY`, npm install routing, `skills.sh/*` and `plugin/*` prefix parsing inside Lua). None of that exists anymore — the Lua plugin now does only `cargo install claude-env`, and all tool management is delegated to the Rust binary.

**Reference (read these first to confirm content is accurate):**
- `hooks/backend_install.lua`
- `hooks/backend_exec_env.lua`
- `hooks/backend_list_versions.lua`
- `metadata.lua`
- `README.md` (top-level — already current)

- [ ] **Step 1: Replace `CLAUDE.md` contents**

Overwrite `CLAUDE.md` with exactly:

```markdown
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A [mise](https://mise.jdx.dev) backend plugin (Lua) that bootstraps the `claude-env` Rust CLI via `cargo install`. All Claude Code tool management — MCP servers, skills, plugins, CLI tools — is delegated to `claude-env`. The plugin itself does nothing more than ensure the binary is installed and triggered on shell entry.

## Local Development

```bash
mise plugin link claude ./
mise install               # bootstraps claude-env via cargo install
claude-env install         # installs tools declared in claude-env.toml
claude-env inspect         # audit current state
```

## Architecture

The plugin implements three of mise's Lua backend hooks. All hooks live in `hooks/`. There is no shared `lib/` — each hook is self-contained, and the small semver helpers are intentionally duplicated across `backend_install.lua` and `backend_list_versions.lua` (both files note this in a comment).

- `hooks/backend_list_versions.lua` — Queries `https://crates.io/api/v1/crates/claude-env/versions` and returns non-yanked versions sorted ascending.
- `hooks/backend_install.lua` — Resolves `latest` via the same crates.io endpoint, then runs `cargo install claude-env --version <v> --root <install_path> --locked`. Writes a `.installed` sentinel that mise uses to confirm the install succeeded.
- `hooks/backend_exec_env.lua` — Adds `<install_path>/bin` to `PATH`. If `claude-env.toml` exists in `pwd`, runs `claude-env install --idempotent --quiet` so missing tools are filled in transparently on shell entry. Failures are logged to stderr but do not break the shell.

`metadata.lua` declares the plugin name, version, author, and license for mise.

## Code Conventions

- Lua with LDoc-style annotations (`--- @param`, `--- @return`)
- Format with [StyLua](https://github.com/JohnnyMorganz/StyLua) (config in `stylua.toml`)
- All shell interpolation goes through `shell_quote()` (defined in each hook that needs it)

## Project Layout

- Root: the mise backend plugin (`hooks/`, `metadata.lua`, `mise.toml` for dev tasks, `Dockerfile.test` + `docker-compose.test.yml` for the test runner)
- `claude-env/`: the Rust CLI as a subcrate (its own `Cargo.toml`, `mise.toml`, `README.md`, `src/`, `tests/`)
- `sample/`: usage examples organized by tool category (`mcp/`, `skillssh/`, `spec/`, `plugin/`). Each sample has a `.mise.toml`, a `claude-env.toml`, and a `test.sh` exercising the install.
- `test/integration.sh`: end-to-end harness — builds claude-env, walks every sample, runs `claude-env install`, and asserts artifacts via each sample's `test.sh`.
- `docs/superpowers/`: design specs and implementation plans (historical record, kept).
```

- [ ] **Step 2: Verify file contents**

Run: `head -30 CLAUDE.md`
Expected: shows the new "What This Is" section with "bootstraps the `claude-env` Rust CLI"

Run: `grep -c "TOOL_REGISTRY\|aliases.lua\|post_install\|npx skills add" CLAUDE.md`
Expected: `0`

- [ ] **Step 3: Commit**

```bash
git add CLAUDE.md
git commit -m "docs(claude.md): rewrite to match current bootstrap-only architecture"
```

---

## Task 2: Rewrite `.github/security-scan-instructions.md`

**Files:**
- Modify: `.github/security-scan-instructions.md` (replace entire contents)

**Why this rewrite:** Current file references `lib/aliases.lua`, `TOOL_REGISTRY`, `npx skills add`, `claude plugin marketplace add`, `post_install`, mkdir-based locking, MCP path validation — all of which lived in the pre-merge plugin and no longer exist. The current attack surface is much smaller: a `cargo install` command and a delegated invocation of the claude-env binary.

**Reference:**
- `hooks/backend_install.lua` (look at how `version`, `install_path` flow into `cargo install`)
- `hooks/backend_exec_env.lua` (look at how `pwd` and `bin` path flow into the install trigger)
- `hooks/backend_list_versions.lua`

- [ ] **Step 1: Replace `.github/security-scan-instructions.md` contents**

Overwrite the file with exactly:

```markdown
In addition to the standard security checks, pay special attention to:

## Bootstrap Surface (Lua hooks)

This repository is a mise backend plugin whose only responsibility is to install and run the `claude-env` Rust binary. The Lua hooks shell out, so quoting matters.

- **`hooks/backend_install.lua`** — Interpolates `version` and `install_path` (from mise) into a `cargo install claude-env --version <v> --root <path> --locked` command. Verify every interpolated value passes through `shell_quote()`. Verify `--locked` is present in any future change to this command (it pins the dependency tree resolved at publish time).
- **`hooks/backend_exec_env.lua`** — Interpolates `pwd` (read via `cmd.exec("pwd")`) and the binary path into a `cd <pwd> && <bin> install --idempotent --quiet` command. Verify both values pass through `shell_quote()`. The presence check for `claude-env.toml` is via `io.open`, not shell — that path is fine.
- **`hooks/backend_list_versions.lua`** — Pure HTTP GET to crates.io plus JSON decode. No shell. Verify the URL stays hard-coded (no interpolation).

## Delegated Surface (claude-env binary)

Out of scope for this plugin's scan. `claude-env` has its own threat model — package install routing, lockfile handling, MCP config writes, plugin marketplace fetches — and is reviewed separately as part of its crates.io publish pipeline. Do not duplicate findings here.

## Supply Chain

- The `cargo install --locked` flag pins the dependency tree resolved at publish time. Loss of `--locked` means transitive deps could shift between installs. Flag any change that removes it.
- The crates.io HTTP fetch in `backend_list_versions.lua` and `backend_install.lua` trusts the registry's response shape. A malformed response would error out; this is acceptable.
```

- [ ] **Step 2: Verify file contents**

Run: `grep -c "TOOL_REGISTRY\|aliases.lua\|npx skills\|post_install\|marketplace add" .github/security-scan-instructions.md`
Expected: `0`

Run: `grep -c "shell_quote\|--locked\|backend_install\.lua" .github/security-scan-instructions.md`
Expected: at least `3`

- [ ] **Step 3: Commit**

```bash
git add .github/security-scan-instructions.md
git commit -m "docs(security): rewrite scan instructions for current attack surface"
```

---

## Task 3: Delete `claude-env/tests/e2e/`

**Files:**
- Delete: `claude-env/tests/e2e/Dockerfile`
- Delete: `claude-env/tests/e2e/docker-compose.yml`
- Delete: `claude-env/tests/e2e/scenarios/run_all.sh`
- Delete: `claude-env/tests/e2e/scenarios/mcp_install.sh`

**Why deletion is safe:**
- Not referenced from `.github/workflows/integration.yml`, `.github/workflows/publish.yml`, `.github/workflows/release.yml` — only the root `docker-compose.test.yml` is invoked.
- Not referenced from root `mise.toml` or `claude-env/mise.toml`.
- Not loaded as a Rust test module — `claude-env/tests/integration.rs` and `claude-env/tests/unit.rs` use explicit `#[path = "..."]` declarations and neither lists anything under `e2e/`.
- The single scenario (`mcp_install.sh`) tests the same MCP install path that root `test/integration.sh` already exercises through `sample/mcp/`.
- Only references in the repo are inside the historical plan `docs/superpowers/plans/2026-04-20-claude-env-implementation.md` (frozen historical record — leave it alone).

- [ ] **Step 1: Verify nothing references `tests/e2e/` outside the historical plan**

Run: `grep -rn "tests/e2e\|e2e/Dockerfile\|e2e/scenarios" .github/ claude-env/Cargo.toml claude-env/mise.toml mise.toml docker-compose.test.yml Dockerfile.test test/ README.md CLAUDE.md 2>/dev/null`
Expected: no output

If you see output: stop and investigate. Something is still consuming the dir.

- [ ] **Step 2: Delete the directory**

Run: `git rm -r claude-env/tests/e2e/`
Expected: shows four files deleted (`Dockerfile`, `docker-compose.yml`, `scenarios/run_all.sh`, `scenarios/mcp_install.sh`)

- [ ] **Step 3: Verify Rust tests still build**

Run: `cd claude-env && cargo test --no-run --quiet 2>&1 | tail -5 && cd ..`
Expected: tests compile (last line "Finished" or similar; no errors mentioning `e2e`)

- [ ] **Step 4: Commit**

```bash
git commit -m "chore: remove unused claude-env/tests/e2e/ harness

Single MCP smoke scenario superseded by the root test/integration.sh
runner that exercises the same path via sample/mcp/. Not invoked by
any workflow, mise task, or Rust test module."
```

---

## Task 4: Rename `.github/VOUCHED.td` → `.github/VOUCHED.md` and update workflow

**Files:**
- Rename: `.github/VOUCHED.td` → `.github/VOUCHED.md`
- Modify: `.github/workflows/pr-vouch.yml` (one line in `paths:` trigger)

**Why one commit:** The workflow trigger pattern must move with the file. If renamed in two commits, the trigger would briefly point at a non-existent path on `main`.

- [ ] **Step 1: Rename the file**

Run: `git mv .github/VOUCHED.td .github/VOUCHED.md`
Expected: rename staged silently

- [ ] **Step 2: Update the workflow trigger**

Edit `.github/workflows/pr-vouch.yml`:

Find:
```yaml
      - .github/VOUCHED.td
```

Replace with:
```yaml
      - .github/VOUCHED.md
```

- [ ] **Step 3: Verify only the path changed**

Run: `git diff --staged .github/workflows/pr-vouch.yml`
Expected: exactly one `-` line (with `.td`) and one `+` line (with `.md`); no other changes.

Run: `grep -rn "VOUCHED\.td" .github/ README.md CLAUDE.md docs/superpowers/specs/2026-05-09-repo-cleanup-design.md 2>/dev/null`
Expected: no output (the spec was written with `.td` references — but it documents the typo as a problem, so check the actual current spec content; it should reference both `.td` (problem) and `.md` (resolution)). If only `.td` survives anywhere except in historical plans, fix it.

Note: historical references in `docs/superpowers/specs/2026-05-09-repo-cleanup-design.md` describing the typo are fine to leave (they describe the change being made).

- [ ] **Step 4: Commit**

```bash
git add .github/workflows/pr-vouch.yml
git commit -m "chore: rename VOUCHED.td → VOUCHED.md and update workflow trigger"
```

---

## Task 5: Add `.superpowers/` to `.gitignore`

**Files:**
- Modify: `.gitignore` (append one line)

**Why:** `git status` currently lists 22 untracked files inside `.superpowers/brainstorm/*/` (HTML mockups, server PIDs, logs from prior brainstorming sessions). These should never enter the repo. Ignoring the top-level dir is the simplest fix.

- [ ] **Step 1: Inspect current `.gitignore`**

Run: `cat .gitignore`
Expected: shows existing entries (`.idea/`, `.mcp.json`, `claude-env/target/`, sample artifact patterns).

- [ ] **Step 2: Append `.superpowers/` line**

Append to `.gitignore` (preserve trailing newline):

```
.superpowers/
```

The final file should look like (existing content + new line):

```
.idea/
.mcp.json
claude-env/target/

# Install artifacts in samples
sample/**/.claude/settings.json
sample/**/.agents/
sample/**/skills-lock.json
sample/skillssh/.claude/skills/

# Brainstorming session state (HTML mockups, server logs, PIDs)
.superpowers/
```

(Add the comment line above `.superpowers/` so future readers understand why.)

- [ ] **Step 3: Verify `.superpowers/` is now ignored**

Run: `git status --untracked-files=all`
Expected: no `.superpowers/...` entries in the untracked list.

Run: `git check-ignore -v .superpowers/brainstorm`
Expected: shows `.gitignore:N:.superpowers/  .superpowers/brainstorm` (confirming ignore rule matches).

- [ ] **Step 4: Commit**

```bash
git add .gitignore
git commit -m "chore: gitignore .superpowers/ brainstorm session state"
```

---

## Task 6: Verify `GITHUB.md` is gone

**Files:**
- Verify deletion: `GITHUB.md` (already removed in a prior conversation turn)

**Why this is verification only:** The user explicitly chose to delete `GITHUB.md` rather than commit it. The `rm` was already executed before this plan was written. This task exists so the executor confirms state and does not accidentally re-create or restore the file.

- [ ] **Step 1: Confirm file is absent**

Run: `ls GITHUB.md 2>&1`
Expected: `ls: GITHUB.md: No such file or directory` (exit code 1)

If file exists: `rm GITHUB.md` (no commit needed — file was never tracked).

- [ ] **Step 2: Confirm file was never in git history**

Run: `git log --all -- GITHUB.md`
Expected: no output (file was never committed).

- [ ] **Step 3: No commit**

Nothing to commit. Move on to Task 7.

---

## Task 7: Final integration verification

**Files:** none (verification only)

**Why:** Confirm no doc/cleanup change broke the test runner or shifted any sample's behavior.

- [ ] **Step 1: Run the full integration suite**

Run: `mise run test`
Expected (final lines):
```
  Passed: 13
  Failed: 0

All tests passed!
```

If anything fails:
- The doc rewrites (Tasks 1, 2) cannot affect tests — fail there means an unrelated regression.
- The `claude-env/tests/e2e/` deletion (Task 3) cannot affect `test/integration.sh` — fail there means investigate.
- The `VOUCHED.md` rename (Task 4) only touches `.github/` — cannot affect tests.
- The `.gitignore` change (Task 5) cannot affect tests.

If a sample fails: read the sample's `test.sh` and the actual `claude-env install` output — root cause before retrying.

- [ ] **Step 2: Final git status check**

Run: `git status`
Expected: clean working tree on `feat/claude-env-design`, branch ahead of `origin/feat/claude-env-design` by 5 commits (Tasks 1, 2, 3, 4, 5).

- [ ] **Step 3: Push**

```bash
git push
```

Expected: 5 new commits pushed to `origin/feat/claude-env-design`.

---

## Done

PR #1 already exists for this branch — pushing appends the cleanup commits to it. No new PR needed.
