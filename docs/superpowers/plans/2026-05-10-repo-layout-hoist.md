# Repo Layout Hoist Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move the Rust crate (`chord/Cargo.toml`, `src/`, `tests/`) up to the repo root and consolidate the e2e test infrastructure (Dockerfile + docker-compose) inside `e2e/` (renamed from `test/`). Mise plugin files (`metadata.lua`, `hooks/`) stay at root.

**Architecture:** Three commits on the existing `feat/rename-to-chord` branch (continues the rename work). Commit 1 reshapes the test infrastructure (`test/` → `e2e/`, Dockerfile + compose move under `e2e/`). Commit 2 hoists `chord/` contents to root and merges `chord/mise.toml` into the root `mise.toml`. Commit 3 rewrites `README.md` + `CLAUDE.md` for the new layout and drops a dead `renovate.json` regex.

**Tech Stack:** Rust (cargo), Lua (mise hooks unchanged), Bash (test runner), Docker, GitHub Actions YAML, TOML configs, Markdown.

**Spec:** `docs/superpowers/specs/2026-05-10-repo-layout-design.md`

---

## File Map

**Renamed (`git mv` preserves history):**
- `test/` → `e2e/`
- `test/integration.sh` → `e2e/run.sh` (carried by directory rename in Step 2 of Task 1)
- `Dockerfile.test` → `e2e/Dockerfile`
- `docker-compose.test.yml` → `e2e/compose.yml`
- `chord/Cargo.toml` → `Cargo.toml`
- `chord/Cargo.lock` → `Cargo.lock`
- `chord/src/` → `src/`
- `chord/tests/` → `tests/`

**Deleted:**
- `chord/README.md` (root README already covers the same content)
- `chord/mise.toml` (tasks merged into root `mise.toml`)
- `chord/` directory (empty after the moves; `target/` inside is gitignored — moved to root with plain `mv`; `.claude/` inside is untracked CC settings — discarded)

**Modified:**
- `.gitignore` — `chord/target/` → `target/`
- `.github/workflows/publish.yml` — drop `--manifest-path chord/Cargo.toml` from `cargo test` and `cargo publish`
- `.github/workflows/release.yml` — `-f docker-compose.test.yml` → `-f e2e/compose.yml`
- `.github/workflows/integration.yml` — `-f docker-compose.test.yml` → `-f e2e/compose.yml`
- `e2e/run.sh` — `/app/test/lib.sh` → `/app/e2e/lib.sh`; `cargo build --manifest-path /app/chord/Cargo.toml` → `cargo build`; `CHORD=/app/chord/target/release/chord` → `CHORD=/app/target/release/chord`
- `e2e/Dockerfile` — `ENTRYPOINT ["bash", "test/integration.sh"]` → `ENTRYPOINT ["bash", "e2e/run.sh"]`
- `e2e/compose.yml` — add `dockerfile: e2e/Dockerfile` (compose context stays `.`)
- `mise.toml` (root) — full rewrite: cargo dev tasks merged from `chord/mise.toml`, docker task renamed `test` → `e2e` and pointed at `e2e/compose.yml`, sample sweepers renamed `samples:clean` / `samples:test`
- `README.md` — drop `chord/src/` reference in "How it works" section
- `CLAUDE.md` — full rewrite reflecting hoisted layout, add Gradle mapping note
- `renovate.json` — drop the entire `customManagers` array

**Untouched:**
- `metadata.lua`, `hooks/**`
- `LICENSE`, `stylua.toml`
- `sample/**`
- `docs/superpowers/**` (historical record)
- `.github/security-scan-instructions.md`, `.github/VOUCHED.md`, `.github/workflows/pr-vouch.yml`

---

## Substitution Reference

Three exact string transforms used in Task 2 source-edit steps:

| Substring | Becomes | Where it appears |
|-----------|---------|------------------|
| `chord/Cargo.toml` | `Cargo.toml` | `.github/workflows/publish.yml` (twice), `e2e/run.sh` (once) |
| `/app/chord/target/release/chord` | `/app/target/release/chord` | `e2e/run.sh` (once) |
| `chord/target/` | `target/` | `.gitignore` (once) |

Verify after each Task: `grep -rn 'chord/' --include='*.toml' --include='*.yml' --include='*.sh' --exclude-dir=docs --exclude-dir=.git --exclude-dir=target` returns nothing.

For BSD `sed` on macOS use `sed -i ''`. For GNU `sed` (Linux/Docker) use `sed -i`. The plan uses BSD syntax in commands but agents should adapt to their environment.

---

## Pre-flight (Task 0)

**Files:** none modified.

- [ ] **Step 1: Confirm branch and clean working tree**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git branch --show-current
git status --short
```

Expected: branch `feat/rename-to-chord`. `git status --short` shows nothing (clean).

- [ ] **Step 2: Confirm baseline cargo tests pass**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude/chord
cargo test --quiet 2>&1 | tail -5
cd ..
```

Expected: `test result: ok. 95 passed; 0 failed` (or whatever the current passing count is). If anything fails before this work begins, STOP and escalate — the layout hoist should not run on a broken baseline.

- [ ] **Step 3: Confirm baseline e2e tests pass**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
mise run test 2>&1 | tail -5
```

Expected:
```
  Passed: 13
  Failed: 0

All tests passed!
```

If the integration suite already fails, STOP — fix the regression before running this plan.

---

## Task 1: Rename `test/` → `e2e/` and consolidate Dockerfile + compose under `e2e/`

**Files:**
- Rename: `test/` → `e2e/` (directory; carries `integration.sh` and `lib.sh`)
- Rename: `e2e/integration.sh` → `e2e/run.sh`
- Rename: `Dockerfile.test` → `e2e/Dockerfile`
- Rename: `docker-compose.test.yml` → `e2e/compose.yml`
- Modify: `e2e/run.sh` (one path reference)
- Modify: `e2e/Dockerfile` (ENTRYPOINT path)
- Modify: `e2e/compose.yml` (add explicit `dockerfile:` field)
- Modify: `mise.toml` (root) — rename docker task `test` → `e2e`, update compose path
- Modify: `.github/workflows/release.yml`, `.github/workflows/integration.yml` (compose path)

After this task, `chord/` is untouched. The cargo crate still builds via `cargo test --manifest-path chord/Cargo.toml`. The docker e2e suite passes via `mise run e2e`.

- [ ] **Step 1: Rename the directory**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git mv test e2e
```

Expected: silent. `git status` shows `R test/integration.sh -> e2e/integration.sh` and `R test/lib.sh -> e2e/lib.sh`.

- [ ] **Step 2: Rename `integration.sh` → `run.sh`**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git mv e2e/integration.sh e2e/run.sh
```

Expected: silent.

- [ ] **Step 3: Move `Dockerfile.test` and `docker-compose.test.yml` into `e2e/`**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git mv Dockerfile.test e2e/Dockerfile
git mv docker-compose.test.yml e2e/compose.yml
```

Expected: silent.

- [ ] **Step 4: Update `e2e/run.sh` `lib.sh` path**

Open `e2e/run.sh`. Find the line:

```bash
  cp /app/test/lib.sh "$tmpdir/lib.sh"
```

Replace with:

```bash
  cp /app/e2e/lib.sh "$tmpdir/lib.sh"
```

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -n "/app/test/" e2e/run.sh; echo "rc=$?"
```

Expected: no output, `rc=1`.

- [ ] **Step 5: Update `e2e/Dockerfile` ENTRYPOINT**

Open `e2e/Dockerfile`. Find the line:

```dockerfile
ENTRYPOINT ["bash", "test/integration.sh"]
```

Replace with:

```dockerfile
ENTRYPOINT ["bash", "e2e/run.sh"]
```

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -n "test/integration\|Dockerfile.test" e2e/Dockerfile; echo "rc=$?"
```

Expected: no output, `rc=1`.

- [ ] **Step 6: Update `e2e/compose.yml` to point at the renamed Dockerfile**

The current `e2e/compose.yml` doesn't specify a `dockerfile:` field — Docker compose defaults to `Dockerfile` in the build context. After moving `Dockerfile.test` into `e2e/`, compose can no longer find it at the default path (compose context is `.`, the repo root, where there is no `Dockerfile`). Make the path explicit.

Open `e2e/compose.yml`. Replace the entire contents with:

```yaml
services:
  test:
    build:
      context: .
      dockerfile: e2e/Dockerfile
    volumes:
      - .:/app
    environment:
      - MISE_YES=1
      - MISE_EXPERIMENTAL=1
```

Note: `context: .` keeps the build context as the repo root so the volume mount and Dockerfile relative paths still work. The mount `- .:/app` puts the whole repo at `/app` inside the container, where `e2e/run.sh` expects to find it.

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
cat e2e/compose.yml
```

Expected: matches the block above exactly.

- [ ] **Step 7: Update root `mise.toml` to rename the docker task to `e2e` and point at the new compose path**

Open `mise.toml` at the repo root. Find the block:

```toml
[tasks.test]
description = "Run integration tests in Docker"
run = "docker compose -f docker-compose.test.yml build && docker compose -f docker-compose.test.yml run --rm test"
```

Replace with:

```toml
[tasks.e2e]
description = "Run end-to-end integration tests in Docker"
run = "docker compose -f e2e/compose.yml build && docker compose -f e2e/compose.yml run --rm test"
```

Leave the rest of `mise.toml` unchanged (it will get a fuller rewrite in Task 2).

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -n "docker-compose.test.yml\|tasks.test" mise.toml; echo "rc=$?"
```

Expected: no output, `rc=1`.

- [ ] **Step 8: Update `.github/workflows/release.yml`**

Open `.github/workflows/release.yml`. Find the line:

```yaml
        run: docker compose -f docker-compose.test.yml run --rm test
```

Replace with:

```yaml
        run: docker compose -f e2e/compose.yml run --rm test
```

- [ ] **Step 9: Update `.github/workflows/integration.yml`**

Open `.github/workflows/integration.yml`. Find the line:

```yaml
        run: docker compose -f docker-compose.test.yml run --rm test
```

Replace with:

```yaml
        run: docker compose -f e2e/compose.yml run --rm test
```

- [ ] **Step 10: Verify no stale references remain**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -rn "Dockerfile\.test\|docker-compose\.test\.yml\|test/integration\.sh\|/app/test/lib\.sh" \
  --include='*.toml' --include='*.yml' --include='*.yaml' --include='*.sh' --include='*.md' \
  --exclude-dir=docs --exclude-dir=.git --exclude-dir=target . 2>/dev/null
echo "rc=$?"
```

Expected: no output, `rc=1`. (Historical references inside `docs/superpowers/` are intentionally untouched; the `--exclude-dir=docs` filter handles them.)

- [ ] **Step 11: Run e2e suite to confirm nothing broke**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
mise run e2e 2>&1 | tail -10
```

Expected (last lines):
```
  Passed: 13
  Failed: 0

All tests passed!
```

If any sample fails: read the failure output. Most likely cause is a missed path reference (e2e/run.sh still pointing to old `test/lib.sh`, or compose still pointing to the old Dockerfile location). Fix the missed reference and re-run.

- [ ] **Step 12: Commit**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git add -A
git commit -m "chore: rename test/ → e2e/ and consolidate Dockerfile + compose under e2e/

- test/ → e2e/, integration.sh → run.sh
- Dockerfile.test → e2e/Dockerfile (.test suffix redundant inside e2e/)
- docker-compose.test.yml → e2e/compose.yml; explicit dockerfile field added
- root mise.toml task 'test' → 'e2e' (chord/ untouched in this commit)
- CI workflows (release.yml, integration.yml) updated to new compose path"
```

---

## Task 2: Hoist `chord/` contents to repo root and merge `mise.toml`

**Files:**
- Rename: `chord/Cargo.toml` → `Cargo.toml`
- Rename: `chord/Cargo.lock` → `Cargo.lock`
- Rename: `chord/src/` → `src/`
- Rename: `chord/tests/` → `tests/`
- Delete: `chord/README.md`
- Delete: `chord/mise.toml`
- Delete: `chord/` directory (after the moves above leave it empty of tracked files)
- Modify: `.gitignore` — `chord/target/` → `target/`
- Modify: `.github/workflows/publish.yml` — drop `--manifest-path chord/Cargo.toml`
- Modify: `e2e/run.sh` — drop `chord/` from cargo build manifest path and binary path
- Modify: `mise.toml` (root) — full rewrite merging cargo dev tasks from old `chord/mise.toml`

This is the largest commit. After it lands, `chord/` no longer exists as a path anywhere outside `docs/superpowers/`.

- [ ] **Step 1: Move the cargo manifest and lockfile to root**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git mv chord/Cargo.toml ./Cargo.toml
git mv chord/Cargo.lock ./Cargo.lock
```

Expected: silent. `git status` shows two renames staged.

- [ ] **Step 2: Move `src/` and `tests/` to root**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git mv chord/src ./src
git mv chord/tests ./tests
```

Expected: silent. `git status` shows the entire `chord/src/**` and `chord/tests/**` trees as renames into `src/**` and `tests/**`.

- [ ] **Step 3: Move build cache (`target/`) to root if it exists**

`target/` is gitignored, so it doesn't show in `git status`, but it holds compiled artifacts that we want to preserve to avoid a full rebuild. Use plain `mv`, not `git mv`.

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
if [ -d chord/target ]; then mv chord/target ./target; fi
```

Expected: silent. If `chord/target/` doesn't exist (fresh checkout), this is a no-op.

- [ ] **Step 4: Delete `chord/README.md` and `chord/mise.toml`**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git rm chord/README.md chord/mise.toml
```

Expected: silent. `git status` shows two deletions.

- [ ] **Step 5: Remove the now-empty `chord/` directory**

After the moves and deletions above, `chord/` may still contain `.claude/` (untracked CC settings) on disk. Wipe it:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
rm -rf chord/
ls chord/ 2>&1
```

Expected: `ls: chord/: No such file or directory` (or equivalent on Linux).

- [ ] **Step 6: Update `.gitignore`**

Open `.gitignore`. Find the line:

```
chord/target/
```

Replace with:

```
target/
```

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
cat .gitignore
```

Expected: `target/` appears (no longer prefixed with `chord/`); rest of file unchanged.

- [ ] **Step 7: Update `.github/workflows/publish.yml`**

Open `.github/workflows/publish.yml`. Find:

```yaml
      - name: Run tests
        run: cargo test --manifest-path chord/Cargo.toml

      - name: Publish rytmyk-chord to crates.io
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
        run: cargo publish --manifest-path chord/Cargo.toml
```

Replace with:

```yaml
      - name: Run tests
        run: cargo test

      - name: Publish rytmyk-chord to crates.io
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
        run: cargo publish
```

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -n "manifest-path\|chord/Cargo.toml" .github/workflows/publish.yml; echo "rc=$?"
```

Expected: no output, `rc=1`.

- [ ] **Step 8: Update `e2e/run.sh`**

Open `e2e/run.sh`. Find the build line:

```bash
if ! cargo build --manifest-path /app/chord/Cargo.toml --release --quiet 2>&1; then
```

Replace with:

```bash
if ! cargo build --manifest-path /app/Cargo.toml --release --quiet 2>&1; then
```

Find the binary path line:

```bash
CHORD=/app/chord/target/release/chord
```

Replace with:

```bash
CHORD=/app/target/release/chord
```

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -n "/app/chord/" e2e/run.sh; echo "rc=$?"
```

Expected: no output, `rc=1`.

- [ ] **Step 9: Replace root `mise.toml` with the merged version**

Overwrite `mise.toml` at the repo root with EXACTLY:

```toml
# Dev config for local testing
# Usage: mise plugin link chord ./
#        mise install

[tools]
rust = "latest"

# ── Cargo dev tasks (fast inner loop) ──

[tasks.build]
description = "Build in debug mode"
run = "cargo build"

[tasks."build:release"]
description = "Build optimized release binary"
run = "cargo build --release"

[tasks.test]
description = "Run cargo unit + integration tests"
run = "cargo test"

[tasks.run]
description = "Run inspect (default)"
run = "cargo run -- inspect"

[tasks."run:tui"]
description = "Run inspect TUI"
run = "cargo run -- inspect --tui"

[tasks."run:install"]
description = "Run install"
run = "cargo run -- install"

[tasks.lint]
description = "Check for warnings"
run = "cargo clippy -- -D warnings"

[tasks.fmt]
description = "Format code"
run = "cargo fmt"

[tasks."fmt:check"]
description = "Check formatting"
run = "cargo fmt -- --check"

[tasks.clean]
description = "Remove build artifacts"
run = "cargo clean"

# ── End-to-end (slow, container) ──

[tasks.e2e]
description = "Run end-to-end integration tests in Docker"
run = "docker compose -f e2e/compose.yml build && docker compose -f e2e/compose.yml run --rm test"

# ── Sample sweeps ──

[tasks."samples:clean"]
description = "Run clean task in all sample modules"
run = """
for dir in $(find sample -name '.mise.toml' -exec dirname {} \\;); do
  echo "Cleaning $dir..."
  (cd "$dir" && MISE_AUTO_INSTALL=false mise run clean || true)
done
"""

[tasks."samples:test"]
description = "Install and test all sample modules"
run = """
for dir in $(find sample -name '.mise.toml' -exec dirname {} \\;); do
  echo "Testing $dir..."
  (cd "$dir" && mise run clean && mise install && mise run test || true)
done
"""
```

Note the two semantic shifts vs. before this commit:
- `mise run test` is now `cargo test` (was: docker integration). Docker is now `mise run e2e`.
- `mise run clean` is now `cargo clean` (was: sample sweeper). Sample sweeper is now `mise run samples:clean`.
- `mise run localtest` is renamed to `mise run samples:test`.

- [ ] **Step 10: Verify cargo builds + tests from the new root**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
cargo test --quiet 2>&1 | tail -5
```

Expected: `test result: ok. 95 passed; 0 failed` (matching the baseline from Task 0). If a test fails, the most likely cause is a path reference inside the Rust source — but the rename PR already swept all `chord/` references out, so this should be clean.

- [ ] **Step 11: Verify e2e suite passes from the new root**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
mise run e2e 2>&1 | tail -10
```

Expected:
```
  Passed: 13
  Failed: 0

All tests passed!
```

If a sample fails: most likely cause is `e2e/run.sh` still referencing the old `chord/` paths. Re-check Step 8.

- [ ] **Step 12: Verify no stale `chord/` path references remain**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -rn '\bchord/' \
  --include='*.toml' --include='*.yml' --include='*.yaml' --include='*.sh' --include='*.lua' --include='*.rs' \
  --exclude-dir=docs --exclude-dir=.git --exclude-dir=target . 2>/dev/null
echo "rc=$?"
```

Expected: no output, `rc=1`. (Markdown docs `README.md` + `CLAUDE.md` are intentionally not in this grep — they get rewritten in Task 3.)

- [ ] **Step 13: Commit**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git add -A
git commit -m "chore: hoist chord/ contents to repo root and merge mise.toml

- chord/Cargo.toml + Cargo.lock → root
- chord/src/ → src/, chord/tests/ → tests/
- chord/README.md and chord/mise.toml deleted (merged into root)
- chord/ directory removed (target/ moved to root, .claude/ was untracked)
- .gitignore: chord/target/ → target/
- publish.yml: cargo test/publish no longer need --manifest-path
- e2e/run.sh: build manifest and binary paths drop chord/ prefix
- root mise.toml: merged cargo dev tasks; 'test' → cargo test, 'e2e' → docker
  (was 'test'); sample sweepers renamed samples:clean / samples:test"
```

---

## Task 3: Rewrite `README.md` + `CLAUDE.md` for hoisted layout, drop dead `renovate.json` regex

**Files:**
- Modify: `README.md` — drop `chord/src/` reference in "How it works"
- Modify: `CLAUDE.md` — full rewrite for new layout
- Modify: `renovate.json` — drop entire `customManagers` array

- [ ] **Step 1: Update `README.md` "How it works" section**

Open `README.md`. Find the line:

```markdown
All actual tool management — MCP server installs, plugin marketplace fetches, skills setup, lockfile maintenance — lives in the `chord` Rust binary (`chord/src/`).
```

Replace with:

```markdown
All actual tool management — MCP server installs, plugin marketplace fetches, skills setup, lockfile maintenance — lives in the `chord` Rust binary (`src/`).
```

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -n "chord/src" README.md; echo "rc=$?"
```

Expected: no output, `rc=1`.

- [ ] **Step 2: Replace `CLAUDE.md` contents**

Overwrite `CLAUDE.md` at the repo root with EXACTLY:

```markdown
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

The `chord` Rust CLI — a declarative agent-tool environment manager — plus a small bundled [mise](https://mise.jdx.dev) backend plugin that bootstraps it. The repo root reads as a Rust crate (`Cargo.toml`, `src/`, `tests/`); `metadata.lua` and `hooks/` at the root are the mise plugin shim (mise's plugin discovery requires them at the git repo root).

The cargo package is `rytmyk-chord` (org-prefixed because the bare `chord` crate is held by an unrelated dormant project). The binary, config file, and mise plugin all use the short name `chord` thanks to `[lib]` and `[[bin]]` overrides in `Cargo.toml`.

## Local Development

```bash
mise plugin link chord ./    # the plugin lives at the repo root
mise install                 # bootstraps chord via cargo install rytmyk-chord
chord install                # installs tools declared in chord.toml
chord inspect                # audit current state
```

Common mise tasks:

- `mise run test` — `cargo test` (fast inner loop)
- `mise run e2e` — full Docker-based end-to-end suite (slow)
- `mise run lint` / `mise run fmt` — clippy and rustfmt
- `mise run samples:test` — install and test every sample module locally

## Architecture

The mise plugin implements three of mise's Lua backend hooks. All hooks live in `hooks/`. There is no shared `lib/` — each hook is self-contained, and the small semver helpers are intentionally duplicated across `backend_install.lua` and `backend_list_versions.lua` (both files note this in a comment).

- `hooks/backend_list_versions.lua` — Queries `https://crates.io/api/v1/crates/rytmyk-chord/versions` and returns non-yanked versions sorted ascending.
- `hooks/backend_install.lua` — Resolves `latest` via the same crates.io endpoint, then runs `cargo install rytmyk-chord --version <v> --root <install_path> --locked`. Writes a `.installed` sentinel that mise uses to confirm the install succeeded.
- `hooks/backend_exec_env.lua` — Adds `<install_path>/bin` to `PATH`. If `chord.toml` exists in `pwd`, runs `chord install --idempotent --quiet` so missing tools are filled in transparently on shell entry. Failures are logged to stderr but do not break the shell.

`metadata.lua` declares the plugin name, version, author, and license for mise.

## Code Conventions

- **Rust:** `cargo fmt` + `cargo clippy -- -D warnings`. Run via `mise run fmt` / `mise run lint`.
- **Lua:** LDoc-style annotations (`--- @param`, `--- @return`); format with [StyLua](https://github.com/JohnnyMorganz/StyLua) (config in `stylua.toml`); all shell interpolation goes through `shell_quote()`.

## Project Layout

- `Cargo.toml`, `Cargo.lock`, `src/`, `tests/`: the `chord` Rust crate. Cargo conventions — `src/main.rs` and `src/lib.rs` for the crate code; `tests/` (plural) for integration tests; auto-discovered by Cargo.
- `metadata.lua`, `hooks/`: the mise backend plugin. Required at root by mise's plugin discovery.
- `e2e/`: end-to-end test harness — `Dockerfile`, `compose.yml`, `run.sh` (the runner), `lib.sh` (assertion helpers). `mise run e2e` builds the container and walks every sample.
- `sample/`: 14 usage examples organized by tool category (`mcp/`, `skillssh/`, `spec/`, `plugin/`). Each sample has a `.mise.toml`, a `chord.toml`, and a `test.sh` exercising the install.
- `mise.toml`: dev tasks (cargo build/test/lint/fmt + e2e + sample sweeps).
- `docs/superpowers/`: design specs and implementation plans (historical record, kept).

For contributors arriving from JVM / Gradle backgrounds: `src/` ≈ `src/main/`, `tests/` ≈ `src/test/`, `e2e/` ≈ `src/e2e/`. Cargo's path conventions are non-negotiable, so we use Rust's native names.
```

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -c "chord/" CLAUDE.md
grep -c "Dockerfile.test\|docker-compose.test.yml" CLAUDE.md
```

Expected: both `0`.

- [ ] **Step 3: Drop dead `renovate.json` `customManagers` regex**

Open `renovate.json`. The current contents:

```json
{
  "$schema": "https://docs.renovatebot.com/renovate-schema.json",
  "extends": ["config:recommended"],
  "customManagers": [
    {
      "customType": "regex",
      "description": "Update npm-backed claude tools in mise sample configs",
      "fileMatch": ["sample/.*\\.mise\\.toml$"],
      "matchStrings": [
        "#\\s*renovate:\\s*datasource=(?<datasource>\\S+)\\s+depName=(?<depName>\\S+)\\n\"claude:[^\"]+\"\\s*=\\s*\"(?<currentValue>[^\"]+)\""
      ]
    }
  ]
}
```

Replace with EXACTLY:

```json
{
  "$schema": "https://docs.renovatebot.com/renovate-schema.json",
  "extends": ["config:recommended"]
}
```

(The `customManagers` regex looked for `"claude:..."` keys in sample `.mise.toml` files. Post-rename, samples no longer use that pattern — the only mise key is `chord = "latest"`, which is updated by the mise plugin's own version checks, not Renovate. The regex is dead.)

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
cat renovate.json
```

Expected: matches the two-line content above exactly.

- [ ] **Step 4: Final sweep — no stale path references in any tracked file**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -rn '\bchord/\|test/integration\.sh\|Dockerfile\.test\|docker-compose\.test\.yml' \
  --include='*.toml' --include='*.yml' --include='*.yaml' --include='*.sh' --include='*.lua' --include='*.rs' --include='*.md' --include='*.json' \
  --exclude-dir=docs --exclude-dir=.git --exclude-dir=target . 2>/dev/null
echo "rc=$?"
```

Expected: no output, `rc=1`. (Historical references inside `docs/superpowers/` are intentionally retained — that's the frozen record.)

- [ ] **Step 5: Commit**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git add README.md CLAUDE.md renovate.json
git commit -m "docs: rewrite README + CLAUDE.md for hoisted layout; drop dead renovate regex

- README.md: 'How it works' references src/ instead of chord/src/
- CLAUDE.md: full rewrite — project = Rust crate at root + mise plugin shim;
  add Project Layout for new tree; document mise task semantic shifts
  (test = cargo test, e2e = docker); add Gradle mapping note for JVM contributors
- renovate.json: drop customManagers — the 'claude:' regex is dead post-rename"
```

---

## Task 4: Final verification + push

**Files:** none modified.

- [ ] **Step 1: Full grep audit — no stale path references survive outside `docs/superpowers/`**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -rn '\bchord/\|test/integration\.sh\|Dockerfile\.test\|docker-compose\.test\.yml' \
  --include='*.toml' --include='*.yml' --include='*.yaml' --include='*.sh' --include='*.lua' --include='*.rs' --include='*.md' --include='*.json' \
  --exclude-dir=docs --exclude-dir=.git --exclude-dir=target . 2>/dev/null
echo "rc=$?"
```

Expected: no output, `rc=1`.

If output appears: read each hit. If it's inside `docs/superpowers/` something is wrong with the `--exclude-dir=docs` filter — investigate. Otherwise: fix the missed reference and add it as a follow-up commit.

- [ ] **Step 2: Final cargo test**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
cargo test --quiet 2>&1 | tail -5
```

Expected: `test result: ok. 95 passed; 0 failed`.

- [ ] **Step 3: Final e2e test**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
mise run e2e 2>&1 | tail -10
```

Expected:
```
  Passed: 13
  Failed: 0

All tests passed!
```

- [ ] **Step 4: Final git status check**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git status
git log --oneline origin/feat/rename-to-chord..HEAD
```

Expected: clean working tree on `feat/rename-to-chord`. Log shows three new commits since the spec commit (one per Task 1, 2, 3).

- [ ] **Step 5: Push**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git push
```

Expected: three new commits pushed to `origin/feat/rename-to-chord`.

---

## Done

Same branch as the rename PR — pushing appends the three layout-hoist commits to it. No new PR needed unless the rename PR has already been opened against `main`; in that case, the layout commits append to the same PR for review.
