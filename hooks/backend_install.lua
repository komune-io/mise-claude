local aliases = require("lib/aliases")
local resolve_alias = aliases.resolve_alias

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
  ["shadcn"] = {
    bin_name = "shadcn",
    mcp_args = { "mcp" },
    post_install = "shadcn mcp init --client claude",
  },
}

-- =============================================================================
-- JSON helpers
-- =============================================================================

--- Escape a string for safe JSON output.
--- @param s string
--- @return string
local function escape_json(s)
  return s:gsub("\\", "\\\\"):gsub('"', '\\"'):gsub("\n", "\\n"):gsub("\r", "\\r"):gsub("\t", "\\t")
end

--- Format the mcpServers data as pretty-printed JSON.
--- @param data table { mcpServers: table }
--- @return string
local function format_mcp_json(data)
  local lines = { "{" }
  table.insert(lines, '  "mcpServers": {')

  -- Collect and sort server names for stable output
  local servers = {}
  for name in pairs(data.mcpServers) do
    table.insert(servers, name)
  end
  table.sort(servers)

  for i, name in ipairs(servers) do
    local s = data.mcpServers[name]
    local trailing = (i < #servers) and "," or ""

    table.insert(lines, '    "' .. escape_json(name) .. '": {')
    table.insert(lines, '      "type": "' .. escape_json(s.type or "stdio") .. '",')
    table.insert(lines, '      "command": "' .. escape_json(s.command) .. '",')

    -- Format args array
    if s.args and #s.args > 0 then
      local arg_strs = {}
      for _, a in ipairs(s.args) do
        table.insert(arg_strs, '"' .. escape_json(a) .. '"')
      end
      table.insert(lines, "      \"args\": [" .. table.concat(arg_strs, ", ") .. "],")
    else
      table.insert(lines, '      "args": [],')
    end

    -- Format env object
    local env_keys = {}
    if s.env then
      for k in pairs(s.env) do
        table.insert(env_keys, k)
      end
    end
    table.sort(env_keys)

    if #env_keys > 0 then
      table.insert(lines, '      "env": {')
      for j, k in ipairs(env_keys) do
        local env_trailing = (j < #env_keys) and "," or ""
        table.insert(
          lines,
          '        "' .. escape_json(k) .. '": "' .. escape_json(s.env[k]) .. '"' .. env_trailing
        )
      end
      table.insert(lines, "      }")
    else
      table.insert(lines, '      "env": {}')
    end

    table.insert(lines, "    }" .. trailing)
  end

  table.insert(lines, "  }")
  table.insert(lines, "}")

  return table.concat(lines, "\n") .. "\n"
end

-- =============================================================================
-- File I/O helpers
-- =============================================================================

local function read_file(path)
  local f = io.open(path, "r")
  if not f then
    return nil
  end
  local content = f:read("*a")
  f:close()
  return content
end

local function write_file(path, content)
  local f = io.open(path, "w")
  if not f then
    error("Cannot write to " .. path)
  end
  f:write(content)
  f:close()
end


-- =============================================================================
-- Install Hook
-- =============================================================================

--- Install an npm-based Claude tool and configure it.
--- MCP tools get added to .mcp.json. CLI tools run an optional post_install command.
--- @param ctx table { tool: string, version: string, install_path: string }
--- @return table
function PLUGIN:BackendInstall(ctx)
  local cmd = require("cmd")
  local json = require("json")

  if aliases.is_skills_sh(ctx.tool) then
    local parsed = aliases.parse_skills_sh(ctx.tool)
    local project_root = os.getenv("MISE_PROJECT_ROOT")
    if not project_root or project_root == "" then
      project_root = cmd.exec("pwd"):gsub("%s+$", "")
    end
    cmd.exec(
      "cd "
        .. project_root
        .. " && npx skills add "
        .. parsed.owner_repo
        .. " --skill "
        .. parsed.skill
        .. " -a claude-code -y"
    )
    return {}
  end

  if aliases.is_plugin(ctx.tool) then
    local parsed = aliases.parse_plugin(ctx.tool)
    -- marketplace add may fail if already registered or if a parallel install is adding it
    pcall(cmd.exec, "claude plugin marketplace add " .. parsed.owner_repo)
    -- Parse marketplace list to find the registered name for this repo
    local list_output = cmd.exec("claude plugin marketplace list")
    local marketplace_name = nil
    for line in list_output:gmatch("[^\n]+") do
      -- Match lines like "  ❯ marketplace-name"
      local name = line:match("\226\157\175%s+(.+)$")
      if name then
        marketplace_name = name:match("^%s*(.-)%s*$") -- trim
      end
      -- Match source lines like "  Source: GitHub (owner/repo)"
      if marketplace_name then
        local repo = line:match("Source:%s+GitHub%s+%(([^)]+)%)")
        if repo then
          if repo == parsed.owner_repo then
            break
          else
            marketplace_name = nil
          end
        end
      end
    end
    if not marketplace_name then
      error("Could not find marketplace for " .. parsed.owner_repo)
    end
    cmd.exec("claude plugin install " .. parsed.plugin .. "@" .. marketplace_name .. " --scope project")
    return {}
  end

  local tool = resolve_alias(ctx.tool)
  local version = ctx.version
  local install_path = ctx.install_path

  if not tool or tool == "" then
    error("Tool name cannot be empty")
  end
  if not version or version == "" then
    error("Version cannot be empty")
  end

  -- 1. Install the npm package
  local npm_cmd = "npm install "
    .. tool
    .. "@"
    .. version
    .. " --prefix "
    .. install_path
    .. " --no-save"

  cmd.exec(npm_cmd)

  -- 2. Detect binary name from node_modules/.bin/
  local bin_dir = install_path .. "/node_modules/.bin"
  local config = TOOL_REGISTRY[tool] or {}

  -- Use explicit bin_name from registry if available, otherwise auto-detect (first binary)
  local bin_name = config.bin_name
  if not bin_name then
    local bin_listing = cmd.exec("ls -1 " .. bin_dir)
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

  -- 3. Determine project root
  local project_root = os.getenv("MISE_PROJECT_ROOT")
  if not project_root or project_root == "" then
    project_root = cmd.exec("pwd"):gsub("%s+$", "")
  end

  -- 4. Determine tool type from registry (default: MCP server)
  local tool_type = config.type or "mcp"

  -- 5. Resolve ${VAR} references in a string
  local function resolve_vars(s)
    return s:gsub("%${([^}]+)}", function(var)
      if var == "PROJECT_ROOT" then
        return project_root
      end
      return os.getenv(var) or ""
    end)
  end

  -- 6. Run post_install command if configured (applies to both CLI and MCP tools)
  if config.post_install then
    local post_cmd = resolve_vars(config.post_install)
    local env_path = bin_dir .. ":" .. (os.getenv("PATH") or "")
    cmd.exec("cd " .. project_root .. " && PATH='" .. env_path .. "' " .. post_cmd)
  end

  -- 7. For CLI tools, skip .mcp.json generation
  if tool_type == "cli" then
    return {}
  end

  -- 8. Read existing .mcp.json or create new structure
  local mcp_path = project_root .. "/.mcp.json"
  local mcp_data = { mcpServers = {} }

  local mcp_content = read_file(mcp_path)
  if mcp_content and mcp_content ~= "" then
    local ok, parsed = pcall(json.decode, mcp_content)
    if ok and parsed then
      mcp_data = parsed
    end
  end

  if not mcp_data.mcpServers then
    mcp_data.mcpServers = {}
  end

  -- 9. Add/update the MCP server entry (keyed by binary name)
  mcp_data.mcpServers[bin_name] = {
    type = "stdio",
    command = bin_path,
    args = config.mcp_args or {},
    env = {},
  }

  -- 10. Write .mcp.json with pretty formatting
  write_file(mcp_path, format_mcp_json(mcp_data))

  return {}
end
