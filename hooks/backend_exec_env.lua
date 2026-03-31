local aliases = require("lib/aliases")
local utils = require("lib/utils")
local mcp_config = require("lib/mcp_config")

--- Apply per-project configuration for an npm tool.
--- Reads the manifest written by BackendInstall and ensures the project is configured.
--- Idempotent: safe to call multiple times.
--- @param cmd table the cmd module
--- @param project_root string
--- @param install_path string
local function configure_npm_project(cmd, project_root, install_path)
  if not project_root or project_root == "" then
    return
  end

  local manifest_path = install_path .. "/.mise-claude-manifest.json"
  local content = utils.read_file(manifest_path)
  if not content then
    return
  end

  local json = require("json")
  local ok, manifest = pcall(json.decode, content)
  if not ok or not manifest then
    return
  end

  -- For CLI tools with post_install: check marker file
  if manifest.post_install then
    local marker_dir = project_root .. "/.claude/.mise-claude"
    local safe_name = manifest.tool:gsub("[^%w%-_]", "-")
    local marker = marker_dir .. "/" .. safe_name .. ".done"
    if not utils.file_exists(marker) then
      local post_cmd = utils.resolve_vars(manifest.post_install, project_root)
      local env_path = manifest.bin_dir .. ":" .. (os.getenv("PATH") or "")
      cmd.exec(
        "cd "
          .. utils.shell_quote(project_root)
          .. " && PATH="
          .. utils.shell_quote(env_path)
          .. " "
          .. post_cmd
          .. " </dev/null"
      )
      utils.ensure_dir(marker_dir)
      utils.write_file(marker, "1")
    end
    return
  end

  -- For MCP tools: ensure server entry in .mcp.json
  if manifest.tool_type ~= "cli" then
    mcp_config.ensure_server(project_root, manifest.bin_name, {
      type = "stdio",
      command = manifest.bin_path,
    })
  end
end

--- Configure skills.sh tool for the current project.
--- @param cmd table the cmd module
--- @param project_root string
--- @param tool string the tool name
local function configure_skills_sh(cmd, project_root, tool)
  if not project_root or project_root == "" then
    return
  end

  local parsed = aliases.parse_skills_sh(tool)
  local safe_name = parsed.owner_repo:gsub("/", "-") .. "-" .. parsed.skill
  local marker_dir = project_root .. "/.claude/.mise-claude"
  local marker = marker_dir .. "/skills-" .. safe_name .. ".done"

  if not utils.file_exists(marker) then
    cmd.exec(
      "cd "
        .. utils.shell_quote(project_root)
        .. " && npx skills add "
        .. utils.shell_quote(parsed.owner_repo)
        .. " --skill "
        .. utils.shell_quote(parsed.skill)
        .. " -a claude-code -y"
    )
    utils.ensure_dir(marker_dir)
    utils.write_file(marker, "1")
  end
end

--- Configure a plugin tool for the current project.
--- @param cmd table the cmd module
--- @param project_root string
--- @param tool string the tool name
local function configure_plugin(cmd, project_root, tool)
  if not project_root or project_root == "" then
    return
  end

  local parsed = aliases.parse_plugin(tool)
  local plugin_id = parsed.plugin .. "@" .. parsed.marketplace

  -- Check if already in .claude/settings.json
  local settings_path = project_root .. "/.claude/settings.json"
  local content = utils.read_file(settings_path)
  if content and content:find(plugin_id, 1, true) then
    return
  end

  -- Serialize plugin install to avoid concurrent .claude/settings.json writes.
  local install_lock = "/tmp/mise-claude-install.lock"
  pcall(
    cmd.exec,
    "for i in $(seq 1 300); do mkdir "
      .. utils.shell_quote(install_lock .. ".d")
      .. " 2>/dev/null && break; sleep 0.1; done"
  )
  pcall(cmd.exec, "claude plugin marketplace add " .. utils.shell_quote(parsed.owner_repo))
  local ok, err = pcall(
    cmd.exec,
    "claude plugin install "
      .. utils.shell_quote(plugin_id)
      .. " --scope project"
  )
  pcall(cmd.exec, "rmdir " .. utils.shell_quote(install_lock .. ".d") .. " 2>/dev/null")
  if not ok then
    error(err)
  end
end

--- Set up environment variables and apply per-project configuration.
--- Configuration is wrapped in pcall — failures are logged but never break the shell.
--- @param ctx table { tool: string, version: string, install_path: string }
--- @return table { env_vars: table<string, string> }
function PLUGIN:BackendExecEnv(ctx)
  local kind = aliases.tool_kind(ctx.tool)

  -- Compute env_vars (PATH for npm tools)
  local env_vars = {}
  if kind == "npm" then
    local bin_dir = ctx.install_path .. "/node_modules/.bin"
    table.insert(env_vars, { key = "PATH", value = bin_dir })
  end

  -- Attempt per-project configuration as a side effect.
  -- This runs when the ExecEnv cache is cold (first activation after install
  -- or cache clear). It won't run on subsequent activations due to mise caching,
  -- but it covers the common case of a fresh install.
  pcall(function()
    local cmd = require("cmd")
    local project_root = utils.get_project_root(cmd)

    if kind == "npm" then
      configure_npm_project(cmd, project_root, ctx.install_path)
    elseif kind == "skills_sh" then
      configure_skills_sh(cmd, project_root, ctx.tool)
    elseif kind == "plugin" then
      configure_plugin(cmd, project_root, ctx.tool)
    end
  end)

  return { env_vars = env_vars }
end
