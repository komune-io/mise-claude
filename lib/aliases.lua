-- =============================================================================
-- Tool Aliases (friendly name -> npm package)
-- =============================================================================

--- Maps short alias names used in .mise.toml to actual npm package names.
--- Unaliased names pass through as-is (e.g. "shadcn" -> "shadcn").
local TOOL_ALIASES = {
	["mcp/context7"] = "@upstash/context7-mcp",
	["mcp/chrome-devtools"] = "chrome-devtools-mcp",
	["mcp/shadcn"] = "shadcn",
	["spec/gsd"] = "get-shit-done-cc",
	["spec/bmad"] = "bmad-method",
	["spec/openspec"] = "@fission-ai/openspec",
}

local SKILLS_SH_PREFIX = "skills.sh/"
local PLUGIN_PREFIX = "plugin/"

--- Resolve a tool alias to its npm package name.
--- @param name string
--- @return string
local function resolve_alias(name)
	return TOOL_ALIASES[name] or name
end

--- Determine the tool kind: "skills_sh", "plugin", or "npm".
--- @param name string
--- @return string
local function tool_kind(name)
	if name:sub(1, #SKILLS_SH_PREFIX) == SKILLS_SH_PREFIX then
		return "skills_sh"
	end
	if name:sub(1, #PLUGIN_PREFIX) == PLUGIN_PREFIX then
		return "plugin"
	end
	return "npm"
end

--- Parse a prefixed tool name (skills.sh/ or plugin/) into owner/repo and trailing component.
--- @param name string e.g. "skills.sh/vercel-labs/next-skills/next-best-practices"
--- @param prefix string e.g. "skills.sh/" or "plugin/"
--- @param label string e.g. "skills.sh" or "plugin" (for error messages)
--- @return table { owner_repo: string, name: string }
local function parse_prefixed(name, prefix, label)
	local path = name:sub(#prefix + 1)
	local owner, repo, component = path:match("^([^/]+)/([^/]+)/(.+)$")
	if not owner then
		error("Invalid " .. label .. " format: expected " .. prefix .. "<owner>/<repo>/<name>, got " .. name)
	end
	return { owner_repo = owner .. "/" .. repo, name = component }
end

--- Parse a skills.sh tool name.
--- @param name string
--- @return table { owner_repo: string, skill: string }
local function parse_skills_sh(name)
	local parsed = parse_prefixed(name, SKILLS_SH_PREFIX, "skills.sh")
	return { owner_repo = parsed.owner_repo, skill = parsed.name }
end

--- Parse a plugin tool name.
--- @param name string e.g. "plugin/anthropics/claude-code/hookify@claude-code-plugins"
--- @return table { owner_repo: string, plugin: string, marketplace: string }
local function parse_plugin(name)
	local parsed = parse_prefixed(name, PLUGIN_PREFIX, "plugin")
	local plugin, marketplace = parsed.name:match("^(.+)@(.+)$")
	if not plugin then
		error("Invalid plugin format: missing @<marketplace> in " .. name)
	end
	return { owner_repo = parsed.owner_repo, plugin = plugin, marketplace = marketplace }
end

return {
	resolve_alias = resolve_alias,
	tool_kind = tool_kind,
	parse_skills_sh = parse_skills_sh,
	parse_plugin = parse_plugin,
}
