local aliases = require("lib/aliases")

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
	},
}

-- =============================================================================
-- Shell helpers
-- =============================================================================

--- Escape a string for use inside single quotes in shell commands.
--- @param s string
--- @return string
local function shell_quote(s)
	return "'" .. s:gsub("'", "'\\''") .. "'"
end

-- =============================================================================
-- Shared helpers
-- =============================================================================

--- Resolve the project root directory.
--- @param cmd table the cmd module
--- @return string
local function get_project_root(cmd)
	local root = os.getenv("MISE_PROJECT_ROOT")
	if root and root ~= "" then
		return root
	end
	return cmd.exec("pwd"):gsub("%s+$", "")
end

--- Resolve ${VAR} references in a string.
--- @param s string
--- @param project_root string
--- @return string
local function resolve_vars(s, project_root)
	return s:gsub("%${([^}]+)}", function(var)
		if var == "PROJECT_ROOT" then
			return project_root
		end
		return os.getenv(var) or ""
	end)
end

-- =============================================================================
-- Install methods (one per tool kind)
-- =============================================================================

--- Install a skills.sh tool via `npx skills add`.
--- @param cmd table the cmd module
--- @param ctx table { tool: string }
local function install_skills_sh(cmd, ctx)
	local parsed = aliases.parse_skills_sh(ctx.tool)
	local project_root = get_project_root(cmd)
	cmd.exec(
		"cd "
			.. shell_quote(project_root)
			.. " && npx skills add "
			.. shell_quote(parsed.owner_repo)
			.. " --skill "
			.. shell_quote(parsed.skill)
			.. " -a claude-code -y"
	)
end

--- Install a Claude Code plugin via `claude plugin marketplace add` + `claude plugin install`.
--- @param cmd table the cmd module
--- @param ctx table { tool: string }
local function install_plugin(cmd, ctx)
	local parsed = aliases.parse_plugin(ctx.tool)

	-- Serialize marketplace registration per repo using mkdir as an atomic lock.
	-- Concurrent `marketplace add` calls for the same repo can race and fail.
	local lock_dir = "/tmp/mise-claude-mktplace-" .. parsed.owner_repo:gsub("/", "-")
	local got_lock = pcall(cmd.exec, "mkdir " .. shell_quote(lock_dir))
	if got_lock then
		cmd.exec("claude plugin marketplace add " .. shell_quote(parsed.owner_repo))
		cmd.exec("touch " .. shell_quote(lock_dir .. "/done"))
	else
		-- Another install is registering it — wait up to 30s for completion
		pcall(
			cmd.exec,
			"for i in $(seq 1 150); do [ -f "
				.. shell_quote(lock_dir .. "/done")
				.. " ] && break; sleep 0.2; done"
		)
	end

	-- Serialize `claude plugin install` calls: concurrent writes to
	-- .claude/settings.json cause lost updates. Use flock if available,
	-- otherwise fall back to a mkdir-based spin lock.
	local install_cmd = "claude plugin install "
		.. shell_quote(parsed.plugin .. "@" .. parsed.marketplace)
		.. " --scope project"
	local install_lock = "/tmp/mise-claude-install.lock"
	local has_flock = pcall(cmd.exec, "command -v flock")
	if has_flock then
		cmd.exec("flock " .. shell_quote(install_lock) .. " " .. install_cmd)
	else
		-- mkdir spin lock for macOS (no flock by default)
		pcall(
			cmd.exec,
			"for i in $(seq 1 300); do mkdir "
				.. shell_quote(install_lock .. ".d")
				.. " 2>/dev/null && break; sleep 0.1; done"
		)
		local ok, err = pcall(cmd.exec, install_cmd)
		pcall(cmd.exec, "rmdir " .. shell_quote(install_lock .. ".d") .. " 2>/dev/null")
		if not ok then
			error(err)
		end
	end
end

--- Install an npm package (MCP server or CLI tool).
--- @param cmd table the cmd module
--- @param ctx table { tool: string, version: string, install_path: string }
local function install_npm(cmd, ctx)
	local tool = aliases.resolve_alias(ctx.tool)
	local version = ctx.version
	local install_path = ctx.install_path

	if not tool or tool == "" then
		error("Tool name cannot be empty")
	end
	if not version or version == "" then
		error("Version cannot be empty")
	end

	-- Install the npm package
	cmd.exec(
		"npm install "
			.. shell_quote(tool .. "@" .. version)
			.. " --prefix "
			.. shell_quote(install_path)
			.. " --no-save"
	)

	-- Detect binary name from node_modules/.bin/
	local bin_dir = install_path .. "/node_modules/.bin"
	local config = TOOL_REGISTRY[tool] or {}
	local bin_name = config.bin_name
	if not bin_name then
		local bin_listing = cmd.exec("ls -1 " .. shell_quote(bin_dir))
		for name in bin_listing:gmatch("[^\n]+") do
			local trimmed = name:match("^%s*(.-)%s*$")
			if trimmed ~= "" then
				bin_name = trimmed
				break
			end
		end
	end
	if not bin_name then
		error("No binary found in " .. bin_dir .. " after installing " .. tool)
	end

	local bin_path = bin_dir .. "/" .. bin_name
	local project_root = get_project_root(cmd)
	local tool_type = config.type or "mcp"

	-- Run post_install command if configured
	if config.post_install then
		local post_cmd = resolve_vars(config.post_install, project_root)
		local env_path = bin_dir .. ":" .. (os.getenv("PATH") or "")
		cmd.exec("cd " .. shell_quote(project_root) .. " && PATH=" .. shell_quote(env_path) .. " " .. post_cmd)
	end

	-- For CLI tools or tools whose post_install handles MCP registration, we're done
	if tool_type == "cli" or config.post_install then
		return
	end

	-- Register MCP server via `claude mcp add`
	local mcp_cmd = "cd "
		.. shell_quote(project_root)
		.. " && claude mcp add --scope project "
		.. shell_quote(bin_name)
		.. " -- "
		.. shell_quote(bin_path)
	cmd.exec(mcp_cmd)
end

-- =============================================================================
-- Install Hook
-- =============================================================================

--- Route install to the appropriate method based on tool kind.
--- @param ctx table { tool: string, version: string, install_path: string }
--- @return table
function PLUGIN:BackendInstall(ctx)
	local cmd = require("cmd")
	local kind = aliases.tool_kind(ctx.tool)

	if kind == "skills_sh" then
		install_skills_sh(cmd, ctx)
	elseif kind == "plugin" then
		install_plugin(cmd, ctx)
	else
		install_npm(cmd, ctx)
	end

	return {}
end
