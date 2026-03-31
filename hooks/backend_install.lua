local aliases = require("lib/aliases")
local registry = require("lib/registry")
local utils = require("lib/utils")
local mcp_config = require("lib/mcp_config")

-- =============================================================================
-- Install methods (one per tool kind)
-- =============================================================================

--- Install a skills.sh tool via `npx skills add`.
--- @param cmd table the cmd module
--- @param ctx table { tool: string, install_path: string }
--- @param project_root string
local function install_skills_sh(cmd, ctx, project_root)
  local parsed = aliases.parse_skills_sh(ctx.tool)

  -- Write sentinel so mise considers the tool installed
  utils.ensure_dir(ctx.install_path)
  utils.write_file(ctx.install_path .. "/.installed", "1")

  cmd.exec(
    "cd "
      .. utils.shell_quote(project_root)
      .. " && npx skills add "
      .. utils.shell_quote(parsed.owner_repo)
      .. " --skill "
      .. utils.shell_quote(parsed.skill)
      .. " -a claude-code -y"
  )
end

--- Install a Claude Code plugin via `claude plugin marketplace add` + `claude plugin install`.
--- @param cmd table the cmd module
--- @param ctx table { tool: string, install_path: string }
local function install_plugin(cmd, ctx)
  local parsed = aliases.parse_plugin(ctx.tool)

  -- Write sentinel so mise considers the tool installed
  utils.ensure_dir(ctx.install_path)
  utils.write_file(ctx.install_path .. "/.installed", "1")

  -- Serialize marketplace registration per repo using mkdir as an atomic lock.
  -- Concurrent `marketplace add` calls for the same repo can race and fail.
  local lock_dir = "/tmp/mise-claude-mktplace-" .. parsed.owner_repo:gsub("/", "-")
  local got_lock = pcall(cmd.exec, "mkdir " .. utils.shell_quote(lock_dir))
  if got_lock then
    cmd.exec("claude plugin marketplace add " .. utils.shell_quote(parsed.owner_repo))
    cmd.exec("touch " .. utils.shell_quote(lock_dir .. "/done"))
  else
    -- Another install is registering it — wait up to 30s for completion
    pcall(
      cmd.exec,
      "for i in $(seq 1 150); do [ -f "
        .. utils.shell_quote(lock_dir .. "/done")
        .. " ] && break; sleep 0.2; done"
    )
  end

  -- Serialize `claude plugin install` calls: concurrent writes to
  -- .claude/settings.json cause lost updates. Use flock if available,
  -- otherwise fall back to a mkdir-based spin lock.
  local install_cmd = "claude plugin install "
    .. utils.shell_quote(parsed.plugin .. "@" .. parsed.marketplace)
    .. " --scope project"
  local install_lock = "/tmp/mise-claude-install.lock"
  local has_flock = pcall(cmd.exec, "command -v flock")
  if has_flock then
    cmd.exec("flock " .. utils.shell_quote(install_lock) .. " " .. install_cmd)
  else
    -- mkdir spin lock for macOS (no flock by default)
    pcall(
      cmd.exec,
      "for i in $(seq 1 300); do mkdir "
        .. utils.shell_quote(install_lock .. ".d")
        .. " 2>/dev/null && break; sleep 0.1; done"
    )
    local ok, err = pcall(cmd.exec, install_cmd)
    pcall(cmd.exec, "rmdir " .. utils.shell_quote(install_lock .. ".d") .. " 2>/dev/null")
    if not ok then
      error(err)
    end
  end
end

--- Detect the binary name from node_modules/.bin/.
--- @param cmd table the cmd module
--- @param install_path string
--- @param config table registry entry
--- @return string binary name
local function detect_bin_name(cmd, install_path, config)
  if config.bin_name then
    return config.bin_name
  end
  local bin_dir = install_path .. "/node_modules/.bin"
  local bin_listing = cmd.exec("ls -1 " .. utils.shell_quote(bin_dir))
  for name in bin_listing:gmatch("[^\n]+") do
    local trimmed = name:match("^%s*(.-)%s*$")
    if trimmed ~= "" then
      return trimmed
    end
  end
  error("No binary found in " .. bin_dir)
end

--- Apply per-project configuration for an npm tool.
--- Idempotent: safe to call multiple times.
--- @param cmd table the cmd module
--- @param project_root string
--- @param manifest table { tool, bin_name, bin_path, bin_dir, tool_type, post_install }
local function configure_npm_project(cmd, project_root, manifest)
  if not project_root or project_root == "" then
    return
  end

  -- For CLI tools or tools whose post_install handles MCP registration,
  -- per-project config is deferred to BackendExecEnv (marker-based idempotency).
  if manifest.tool_type == "cli" or manifest.post_install then
    return
  end

  -- Register MCP server via direct file I/O
  mcp_config.ensure_server(project_root, manifest.bin_name, {
    type = "stdio",
    command = manifest.bin_path,
  })
end

--- Install an npm package (MCP server or CLI tool).
--- @param cmd table the cmd module
--- @param ctx table { tool: string, version: string, install_path: string }
--- @param project_root string
local function install_npm(cmd, ctx, project_root)
  local tool = aliases.resolve_alias(ctx.tool)
  local version = ctx.version
  local install_path = ctx.install_path

  if not tool or tool == "" then
    error("Tool name cannot be empty")
  end
  if not version or version == "" then
    error("Version cannot be empty")
  end

  local config = registry[tool] or {}

  -- Install the npm package (plus any extra transitive-dep overrides)
  local install_args = utils.shell_quote(tool .. "@" .. version)
  if config.extra_deps then
    for _, dep in ipairs(config.extra_deps) do
      install_args = install_args .. " " .. utils.shell_quote(dep)
    end
  end
  cmd.exec("npm install " .. install_args .. " --prefix " .. utils.shell_quote(install_path) .. " --no-save")

  -- Detect binary name
  local bin_name = detect_bin_name(cmd, install_path, config)
  local bin_dir = install_path .. "/node_modules/.bin"
  local bin_path = bin_dir .. "/" .. bin_name
  local tool_type = config.type or "mcp"

  -- Write manifest for use by BackendExecEnv per-project config
  local json = require("json")
  local manifest = {
    tool = tool,
    version = version,
    bin_name = bin_name,
    bin_path = bin_path,
    bin_dir = bin_dir,
    tool_type = tool_type,
    post_install = config.post_install,
  }
  utils.write_file(install_path .. "/.mise-claude-manifest.json", json.encode(manifest))

  -- Configure the current project
  configure_npm_project(cmd, project_root, manifest)
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
  local project_root = utils.get_project_root(cmd)

  if kind == "skills_sh" then
    install_skills_sh(cmd, ctx, project_root)
  elseif kind == "plugin" then
    install_plugin(cmd, ctx)
  else
    install_npm(cmd, ctx, project_root)
  end

  return {}
end
