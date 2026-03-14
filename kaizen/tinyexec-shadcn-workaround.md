# Remove tinyexec pinning workaround for shadcn

## Context

`tinyexec@1.0.3` ships `dist/main.mjs` but declares `"main": "./dist/main.js"` — a broken release.
This breaks `shadcn` (via `@antfu/ni` → `tinyexec@^1.0.2` → resolves to `1.0.3`).

We added `extra_deps = { "tinyexec@1.0.2" }` in `TOOL_REGISTRY["shadcn"]` to force the working version.

## Action

When `tinyexec` releases a fix (likely `1.0.4`):

1. Remove `extra_deps` from the shadcn entry in `hooks/backend_install.lua`
2. Remove the `extra_deps` handling in `install_npm()` if no other tool uses it
3. Run `mise run test` to confirm the mcp test passes without the pin
