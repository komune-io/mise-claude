# Repo Layout Hoist Design

**Date:** 2026-05-10
**Branch:** new branch off `main` (after rename PR merges) — proposed `feat/hoist-crate-to-root`
**Scope:** Move the Rust crate (`chord/Cargo.toml`, `src/`, `tests/`) up to the repo root so the project reads as a Rust crate at first glance. Co-locate the e2e test infrastructure (Dockerfile + docker-compose) inside `e2e/`. Mise plugin files (`metadata.lua`, `hooks/`) stay at the repo root because mise's plugin discovery requires it.

## Background and Goal

After the `chord` rename PR, the repo is two artifacts:

- A Rust crate at `chord/` (binary `chord`, package `rytmyk-chord`)
- A mise backend plugin at the repo root (`metadata.lua`, `hooks/`)

A reader landing on the GitHub repo today sees the Lua plugin first (the README is currently chord-branded but the file tree at root reads "Lua/mise plugin"). The Rust crate is the actual product; the mise plugin is one delivery mechanism (with homebrew, asdf, scoop, etc. as plausible future siblings).

**Goal:** repo root reads as "this is a Rust crate" (`Cargo.toml`, `src/`, `tests/` at top), with the mise plugin as a small bundled shim. Public install path (`mise plugin install chord https://github.com/rytmyk/chord`) keeps working unchanged.

## Constraint that shaped the design

Mise backend plugins **must** have `metadata.lua` and `hooks/` at the root of the git repo for `mise plugin install <name> <git-url>` to work. Subdirectory plugin discovery is not supported. Verified against mise's plugin-publishing docs.

Therefore: keeping a single repo + keeping the public install URL working **forces** the mise plugin files to live at root alongside the Rust crate files. The single-repo / mise-files-at-root layout was selected (variant "A1" in brainstorming).

## Final Layout

```
rytmyk/chord/
├── .github/                     # workflows, security-scan-instructions, VOUCHED
├── .gitignore
├── CLAUDE.md                    # rewritten (project = Rust crate + bundled mise shim)
├── Cargo.toml                   # rytmyk-chord (was chord/Cargo.toml)
├── Cargo.lock                   # was chord/Cargo.lock
├── LICENSE
├── README.md                    # current rich one — chord/README.md deleted
├── docs/                        # historical specs/plans, unchanged
├── e2e/                         # was test/ — renamed to avoid clashing with cargo's tests/
│   ├── Dockerfile               # was Dockerfile.test (.test redundant inside e2e/)
│   ├── compose.yml              # was docker-compose.test.yml
│   ├── lib.sh                   # unchanged content
│   └── run.sh                   # was integration.sh
├── hooks/                       # mise plugin hooks — STAYS at root (mise constraint)
│   ├── backend_install.lua
│   ├── backend_list_versions.lua
│   └── backend_exec_env.lua
├── metadata.lua                 # mise plugin metadata — STAYS at root (mise constraint)
├── mise.toml                    # merged: cargo dev tasks + e2e docker task + sample sweeps
├── renovate.json                # dead `claude:` customManager removed
├── sample/                      # 14 example projects, unchanged
├── src/                         # was chord/src/
├── stylua.toml                  # unchanged
├── target/                      # was chord/target/ (gitignored)
└── tests/                       # was chord/tests/ — cargo integration tests
```

**What disappears:**
- `chord/` directory
- `chord/README.md` (root README already covers it)
- `chord/mise.toml` (tasks merged into root `mise.toml`)
- `chord/.claude/` (untracked CC settings, irrelevant to git)

**What is renamed:**
- `test/` → `e2e/`
- `test/integration.sh` → `e2e/run.sh`
- `Dockerfile.test` → `e2e/Dockerfile`
- `docker-compose.test.yml` → `e2e/compose.yml`

**What stays at root because mise requires it:**
- `metadata.lua`
- `hooks/`

## Layout Rationale (Cargo conventions)

Earlier brainstorming considered a Gradle-like `src/main/`, `src/test/`, `src/e2e/` grouping. Rejected because Cargo auto-discovers `src/` and `tests/` (plural) at fixed paths relative to `Cargo.toml`. Overriding via per-target `[[test]] path = "..."` entries in `Cargo.toml` would defeat auto-discovery and confuse any Rust developer reading the project.

The Rust-idiomatic mapping conveys the same intent:

| Gradle idiom | Chord (Rust idiom) |
|--------------|---------------------|
| `src/main/`  | `src/`              |
| `src/test/`  | `tests/`            |
| `src/e2e/`   | `e2e/`              |

A one-line note in `CLAUDE.md` documents this mapping for contributors arriving from JVM backgrounds.

Wrapping `tests/` and `e2e/` in a parent dir (e.g. `test/cargo/` + `test/e2e/`) was rejected for the same Cargo-discovery reason. Tucking `e2e/` inside `tests/` (`tests/e2e/run.sh`) was rejected because it obscures the boundary between Rust integration tests and bash/docker end-to-end tests.

## `mise.toml` Merge

Today there are two `mise.toml` files with conflicting task names:
- Root: `clean` (sample sweeper), `test` (docker integration), `localtest` (sample sweeper)
- `chord/`: `clean` (cargo), `build`, `build:release`, `test` (cargo), `run`, `run:tui`, `run:install`, `lint`, `fmt`, `fmt:check`

Merged root `mise.toml`:

```toml
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

**Two semantic shifts:**
1. `mise run test` now means **`cargo test`** (fast Rust unit + integration), not docker. Docker e2e becomes `mise run e2e`.
2. `mise run clean` now means **`cargo clean`**. Sample sweeper is now `mise run samples:clean`.

CI bypasses mise tasks (it calls `docker compose ...` directly), so the rename of `test` → `e2e` only affects local dev habits. Documented in `README.md` "Local development" section.

## Touch-Point Inventory

### Files moved

- `git mv chord/Cargo.toml ./`
- `git mv chord/Cargo.lock ./`
- `git mv chord/src ./`
- `git mv chord/tests ./`
- `git mv test/integration.sh e2e/run.sh` (after `git mv test e2e`)
- `git mv test/lib.sh e2e/lib.sh` (carried by directory rename)
- `git mv Dockerfile.test e2e/Dockerfile`
- `git mv docker-compose.test.yml e2e/compose.yml`

### Files deleted

- `chord/README.md`
- `chord/mise.toml`
- `chord/` directory (empty after the moves above)

### Files modified

- `.gitignore` — `chord/target/` → `target/`
- `.github/workflows/publish.yml` — drop `--manifest-path chord/Cargo.toml` from `cargo test` and `cargo publish` commands
- `.github/workflows/release.yml` — `-f docker-compose.test.yml` → `-f e2e/compose.yml`
- `.github/workflows/integration.yml` — same docker-compose path update
- `e2e/run.sh` — `cargo build --manifest-path /app/chord/Cargo.toml --release` → `cargo build --release`; `CHORD=/app/chord/target/release/chord` → `CHORD=/app/target/release/chord`; `cp /app/test/lib.sh ...` → `cp /app/e2e/lib.sh ...`; "test/integration.sh" comment updates
- `e2e/Dockerfile` — `ENTRYPOINT ["bash", "test/integration.sh"]` → `ENTRYPOINT ["bash", "e2e/run.sh"]`
- `e2e/compose.yml` — `dockerfile: Dockerfile.test` → `dockerfile: e2e/Dockerfile` (context stays `.`)
- `mise.toml` (root) — full rewrite per merge above
- `README.md` — drop `chord/src/` reference in "How it works"; `mise plugin link chord ./` already correct (mise plugin files already at root)
- `CLAUDE.md` — full rewrite: project layout section reflects new tree, add Gradle-mapping note
- `renovate.json` — drop the entire `customManagers` array (its only entry is the dead `"claude:[^"]+"` matcher; samples no longer use that pattern). Resulting file keeps `$schema` and `extends` only.

### Files unchanged

- `metadata.lua`
- `hooks/backend_install.lua`, `hooks/backend_list_versions.lua`, `hooks/backend_exec_env.lua`
- `LICENSE`
- `stylua.toml`
- `sample/**`
- `docs/superpowers/**` (historical record)
- `.github/security-scan-instructions.md`, `.github/VOUCHED.md`, `.github/workflows/pr-vouch.yml`

## Commit Plan

One PR, three commits for review hygiene:

1. **`chore: rename test/ → e2e/ and consolidate docker compose under e2e/`**
   - `git mv test e2e`
   - `git mv e2e/integration.sh e2e/run.sh`
   - `git mv Dockerfile.test e2e/Dockerfile`
   - `git mv docker-compose.test.yml e2e/compose.yml`
   - Update path refs inside `e2e/run.sh` (`/app/test/lib.sh` → `/app/e2e/lib.sh`), `e2e/Dockerfile` (ENTRYPOINT), `e2e/compose.yml` (dockerfile field), root `mise.toml` (just the docker task line, renamed to `e2e`), `release.yml`, `integration.yml`.
   - Verify `docker compose -f e2e/compose.yml run --rm test` passes 13/13.

2. **`chore: hoist chord/ contents to repo root and merge mise.toml`**
   - `git mv chord/Cargo.toml ./`, `chord/Cargo.lock ./`, `chord/src ./`, `chord/tests ./`
   - `rm -r chord/README.md chord/mise.toml chord/` (the `.claude/` inside is untracked)
   - Merge cargo dev tasks from old `chord/mise.toml` into root `mise.toml` (Section "`mise.toml` Merge" above).
   - Update `.gitignore` (`chord/target/` → `target/`).
   - Update `.github/workflows/publish.yml` (drop `--manifest-path chord/Cargo.toml` from both `cargo test` and `cargo publish`).
   - Update `e2e/run.sh` (drop `chord/` from build manifest path and binary path).
   - Verify `cargo test` passes (95/95) and `mise run e2e` passes 13/13.

3. **`docs: rewrite README + CLAUDE.md for hoisted layout; remove dead renovate regex`**
   - `README.md`: drop the `chord/src/` reference in the "How it works" section.
   - `CLAUDE.md`: rewrite Project Layout to reflect new tree; add a one-line Gradle mapping note; mention that `mise run test` = `cargo test` and `mise run e2e` = the docker integration suite.
   - `renovate.json`: drop the entire `customManagers` array (its only entry is the dead `"claude:[^"]+"` matcher).

After all three commits land:
- `git grep -nI 'chord/\|test/integration\.sh\|Dockerfile\.test\|docker-compose\.test\.yml' -- ':!docs/superpowers/'` returns nothing.
- `cargo test` is green (95/95).
- `mise run e2e` is green (13/13).

## Verification

Per-commit gates already noted above. After the PR is fully assembled:

- `cargo build --release` from repo root succeeds without `--manifest-path`.
- `cargo test` passes 95/95.
- `mise run e2e` passes 13/13.
- Local install of the plugin from a fresh clone: `mise plugin install chord <local-clone-url>` succeeds (mise sees `metadata.lua` + `hooks/` at root). Smoke test in a sample dir.

## Risks

- **`mise run test` now means cargo test, not docker.** Anyone with muscle memory or aliases for the old behavior will be surprised the first time. Mitigation: README "Local development" section calls out the new naming explicitly, and the docker task is still one keyword away (`mise run e2e`).
- **`docker-compose.test.yml` rename to `e2e/compose.yml`.** External CI configs or shell aliases referring to the old path will break. Within the repo: 3 references (release.yml, integration.yml, root mise.toml) — all updated in commit 1. No external consumers known.
- **Cargo manifest path drop in `publish.yml`.** If commit 2 lands without commit 2's edit to `publish.yml`, the next tag-triggered publish would fail. Mitigation: both edits are in commit 2 (atomic).
- **Lua + Rust files at root looks unusual.** Some Rust contributors may wonder why `metadata.lua` and `hooks/` sit next to `Cargo.toml`. Mitigated by a short README "How it works" section already present (mentions the bundled mise plugin); CLAUDE.md rewrite reinforces it.

## Out of Scope

- Splitting the mise plugin into a separate repo (rejected as variant "A2" in brainstorming).
- Moving the mise plugin files into `plugin/mise/` (would break `mise plugin install <name> <git-url>` per mise's published constraint).
- Republishing the crate as a different package name.
- Any change to sample contents or the public chord CLI.
- GitHub repo transfer / org rename (already covered in the prior rename design; this PR is layout-only).
