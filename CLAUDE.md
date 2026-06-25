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
- **Module seam:** `src/core/` is pure domain logic and must not import `ratatui`, `crossterm`, or `clap`; `src/shell/` holds the CLI + TUI. `tests/architecture.rs` enforces this — a UI import from `core` fails the test suite.

## Project Layout

- `Cargo.toml`, `Cargo.lock`, `src/`, `tests/`: the `chord` Rust crate. Cargo conventions — `src/main.rs` and `src/lib.rs` for the crate code; `tests/` (plural) for integration tests; auto-discovered by Cargo.
- `metadata.lua`, `hooks/`: the mise backend plugin. Required at root by mise's plugin discovery.
- `e2e/`: end-to-end test harness — `Dockerfile`, `compose.yml`, `run.sh` (the runner), `lib.sh` (assertion helpers). `mise run e2e` builds the container and walks every sample.
- `sample/`: 14 usage examples organized by tool category (`mcp/`, `skillssh/`, `spec/`, `plugin/`). Each sample has a `.mise.toml`, a `chord.toml`, and a `test.sh` exercising the install.
- `mise.toml`: dev tasks (cargo build/test/lint/fmt + e2e + sample sweeps).
- `docs/superpowers/`: design specs and implementation plans (historical record, kept).

For contributors arriving from JVM / Gradle backgrounds: `src/` ≈ `src/main/`, `tests/` ≈ `src/test/`, `e2e/` ≈ `src/e2e/`. Cargo's path conventions are non-negotiable, so we use Rust's native names.

## Agent skills

### Issue tracker

Local markdown under `.scratch/<feature-slug>/` (no GitHub Issues). See `docs/agents/issue-tracker.md`.

### Triage labels

Canonical role strings, used as `Status:` lines in issue files. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context — `CONTEXT.md` + `docs/adr/` at the repo root (will be created lazily by `/grill-with-docs`). See `docs/agents/domain.md`.
