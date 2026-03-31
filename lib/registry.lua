-- =============================================================================
-- Tool Registry (keyed by npm package name)
-- =============================================================================

--- Known CLI tools: type = "cli" skips .mcp.json, post_install runs after npm install.
--- Tools not listed here default to type = "mcp" (added to .mcp.json).
local TOOL_REGISTRY = {
  ["get-shit-done-cc"] = {
    type = "cli",
    post_install = "get-shit-done-cc --claude --local",
  },
  ["bmad-method"] = {
    type = "cli",
    post_install = "bmad-method install --directory ${PROJECT_ROOT} --modules bmm --tools claude-code --yes",
  },
  ["@fission-ai/openspec"] = {
    type = "cli",
    post_install = "openspec init --tools claude",
  },
  ["chrome-devtools-mcp"] = {
    bin_name = "chrome-devtools-mcp",
  },
  ["shadcn"] = {
    bin_name = "shadcn",
    post_install = "shadcn mcp init --client claude",
    -- tinyexec 1.0.3 ships dist/main.mjs but declares main: dist/main.js
    extra_deps = { "tinyexec@1.0.2" },
  },
}

return TOOL_REGISTRY
