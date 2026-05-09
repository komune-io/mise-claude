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
