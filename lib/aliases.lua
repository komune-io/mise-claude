-- =============================================================================
-- Tool Aliases (friendly name → npm package)
-- =============================================================================

--- Maps short alias names used in .mise.toml to actual npm package names.
--- Unaliased names pass through as-is (e.g. "shadcn" → "shadcn").
local TOOL_ALIASES = {
  ["mcp/context7"]        = "@upstash/context7-mcp",
  ["mcp/chrome-devtools"] = "chrome-devtools-mcp",
  ["mcp/shadcn"]          = "shadcn",
  ["spec/gsd"]            = "get-shit-done-cc",
  ["spec/bmad"]           = "bmad-method",
  ["spec/openspec"]       = "@fission-ai/openspec",
}

--- Resolve a tool alias to its npm package name.
--- @param name string
--- @return string
local function resolve_alias(name)
  return TOOL_ALIASES[name] or name
end

--- Check if a tool name is a skills.sh tool.
--- @param name string
--- @return boolean
local function is_skills_sh(name)
  return name:sub(1, 10) == "skills.sh/"
end

--- Parse a skills.sh tool name into owner/repo and skill.
--- @param name string e.g. "skills.sh/vercel-labs/next-skills/next-best-practices"
--- @return table { owner_repo: string, skill: string }
local function parse_skills_sh(name)
  local path = name:sub(11) -- strip "skills.sh/"
  local owner, repo, skill = path:match("^([^/]+)/([^/]+)/(.+)$")
  if not owner then
    error("Invalid skills.sh format: expected skills.sh/<owner>/<repo>/<skill>, got " .. name)
  end
  return { owner_repo = owner .. "/" .. repo, skill = skill }
end

--- Check if a tool name is a Claude Code plugin.
--- @param name string
--- @return boolean
local function is_plugin(name)
  return name:sub(1, 7) == "plugin/"
end

--- Parse a plugin tool name into owner/repo and plugin.
--- @param name string e.g. "plugin/anthropics/claude-code/commit-commands"
--- @return table { owner_repo: string, plugin: string }
local function parse_plugin(name)
  local path = name:sub(8) -- strip "plugin/"
  local owner, repo, plugin = path:match("^([^/]+)/([^/]+)/(.+)$")
  if not owner then
    error("Invalid plugin format: expected plugin/<owner>/<repo>/<plugin>, got " .. name)
  end
  return { owner_repo = owner .. "/" .. repo, plugin = plugin }
end

return {
  TOOL_ALIASES = TOOL_ALIASES,
  resolve_alias = resolve_alias,
  is_skills_sh = is_skills_sh,
  parse_skills_sh = parse_skills_sh,
  is_plugin = is_plugin,
  parse_plugin = parse_plugin,
}