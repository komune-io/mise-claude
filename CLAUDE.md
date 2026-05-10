# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A [mise](https://mise.jdx.dev) backend plugin (Lua) that bootstraps the `chord` Rust CLI via `cargo install rytmyk-chord`. All agent-tool management — MCP servers, skills, plugins, CLI tools — is delegated to `chord`. The plugin itself does nothing more than ensure the binary is installed and triggered on shell entry.

The cargo package is `rytmyk-chord` (org-prefixed because the bare `chord` crate is held by an unrelated dormant project). The binary, config file, and mise plugin all use the short name `chord` thanks to `[lib]` and `[[bin]]` overrides in `chord/Cargo.toml`.

## Local Development

```bash
mise plugin link chord ./
mise install               # bootstraps chord via cargo install rytmyk-chord
chord install              # installs tools declared in chord.toml
chord inspect              # audit current state
```

## Architecture

The plugin implements three of mise's Lua backend hooks. All hooks live in `hooks/`. There is no shared `lib/` — each hook is self-contained, and the small semver helpers are intentionally duplicated across `backend_install.lua` and `backend_list_versions.lua` (both files note this in a comment).

- `hooks/backend_list_versions.lua` — Queries `https://crates.io/api/v1/crates/rytmyk-chord/versions` and returns non-yanked versions sorted ascending.
- `hooks/backend_install.lua` — Resolves `latest` via the same crates.io endpoint, then runs `cargo install rytmyk-chord --version <v> --root <install_path> --locked`. Writes a `.installed` sentinel that mise uses to confirm the install succeeded.
- `hooks/backend_exec_env.lua` — Adds `<install_path>/bin` to `PATH`. If `chord.toml` exists in `pwd`, runs `chord install --idempotent --quiet` so missing tools are filled in transparently on shell entry. Failures are logged to stderr but do not break the shell.

`metadata.lua` declares the plugin name, version, author, and license for mise.

## Code Conventions

- Lua with LDoc-style annotations (`--- @param`, `--- @return`)
- Format with [StyLua](https://github.com/JohnnyMorganz/StyLua) (config in `stylua.toml`)
- All shell interpolation goes through `shell_quote()` (defined in each hook that needs it)

## Project Layout

- Root: the mise backend plugin (`hooks/`, `metadata.lua`, `mise.toml` for dev tasks, `e2e/` for the end-to-end test suite)
- `chord/`: the Rust CLI as a subcrate (its own `Cargo.toml`, `mise.toml`, `README.md`, `src/`, `tests/`)
- `sample/`: usage examples organized by tool category (`mcp/`, `skillssh/`, `spec/`, `plugin/`). Each sample has a `.mise.toml`, a `chord.toml`, and a `test.sh` exercising the install.
- `e2e/run.sh`: end-to-end harness — builds chord, walks every sample, runs `chord install`, and asserts artifacts via each sample's `test.sh`. Run via `mise run e2e` (uses `e2e/Dockerfile` + `e2e/compose.yml`).
- `docs/superpowers/`: design specs and implementation plans (historical record, kept).
