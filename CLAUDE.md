# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A [mise](https://mise.jdx.dev) backend plugin (Lua) that manages Claude Code tooling declaratively. Users declare MCP servers, spec tools, skills, and plugins in `.mise.toml` — `mise install` handles the rest.

## Local Development

```bash
mise plugin link claude ./
mise install claude:@upstash/context7-mcp@latest
mise ls
```

## Architecture

The plugin implements mise's Lua backend hook system. All hooks live in `hooks/` and share helpers from `lib/`.

- `lib/aliases.lua` — Maps friendly names (e.g. `mcp/context7`) to npm packages, and handles `skills.sh/` and `plugin/` prefix parsing.
- `hooks/backend_list_versions.lua` — Queries the npm registry for versions. Skills and plugins only support `"latest"`.
- `hooks/backend_install.lua` — Core install logic. Routes by tool type:
  - **`skills.sh/*`** — Runs `npx skills add` with the parsed owner/repo/skill.
  - **`plugin/*`** — Runs `claude plugin marketplace add` then `claude plugin install`.
  - **npm tools** — Runs `npm install`, then branches:
    - `type = "mcp"` (default) — Detects the binary in `node_modules/.bin/`, merges entry into `.mcp.json`.
    - `type = "cli"` — Skips `.mcp.json`.
    - Both types run `post_install` if configured in `TOOL_REGISTRY`.
- `hooks/backend_exec_env.lua` — Adds `node_modules/.bin` to PATH for installed tools.

The `TOOL_REGISTRY` table in `backend_install.lua` defines per-package overrides (`type`, `bin_name`, `mcp_args`, `post_install`). Unlisted packages default to MCP type.

## Code Conventions

- Lua with LDoc-style annotations (`--- @param`, `--- @return`)
- Format with [StyLua](https://github.com/JohnnyMorganz/StyLua) (config in `stylua.toml`)

## Project Layout

- Root: the mise backend plugin itself (`hooks/`, `lib/`, `metadata.lua`)
- `sample/`: usage examples organized by tool type (`mcp/`, `spec/`, `skillssh/`, `plugin/`)
