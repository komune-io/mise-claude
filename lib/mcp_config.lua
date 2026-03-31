-- =============================================================================
-- .mcp.json read/write via direct file I/O
-- =============================================================================

local utils = require("lib/utils")

local M = {}

--- Read and parse the .mcp.json file from a project root.
--- @param project_root string
--- @return table parsed config with at least { mcpServers = {} }
function M.read_config(project_root)
  local json = require("json")
  local path = project_root .. "/.mcp.json"
  local content = utils.read_file(path)
  if not content or content == "" then
    return { mcpServers = {} }
  end
  local ok, data = pcall(json.decode, content)
  if not ok or not data then
    return { mcpServers = {} }
  end
  if not data.mcpServers then
    data.mcpServers = {}
  end
  return data
end

--- Write the config back to .mcp.json.
--- @param project_root string
--- @param config table
function M.write_config(project_root, config)
  local json = require("json")
  local path = project_root .. "/.mcp.json"
  local content = json.encode(config)
  utils.write_file(path, content)
end

--- Ensure an MCP server entry exists in .mcp.json.
--- Adds the entry if missing; writes back only if changed.
--- Uses a lock file to prevent concurrent write corruption.
--- @param project_root string
--- @param name string server name (e.g. "context7-mcp")
--- @param server_config table { command: string, args: string[], type: string }
--- @return boolean true if the entry was added (file was changed)
function M.ensure_server(project_root, name, server_config)
  local lock_path = project_root .. "/.mcp.json.lock"

  -- Acquire lock via mkdir (atomic on all platforms).
  -- Remove stale locks older than 30s to avoid permanent penalty after crashes.
  os.execute(
    "find " .. utils.shell_quote(lock_path) .. " -maxdepth 0 -mmin +0.5 -exec rmdir {} \\; 2>/dev/null"
  )
  local max_attempts = 50
  for i = 1, max_attempts do
    local ok = os.execute("mkdir " .. utils.shell_quote(lock_path) .. " 2>/dev/null")
    if ok then
      break
    end
    if i == max_attempts then
      break
    end
    os.execute("sleep 0.1")
  end

  local changed = false
  local success, err = pcall(function()
    local config = M.read_config(project_root)
    if not config.mcpServers[name] then
      config.mcpServers[name] = server_config
      M.write_config(project_root, config)
      changed = true
    end
  end)

  -- Release lock
  os.execute("rmdir " .. utils.shell_quote(lock_path) .. " 2>/dev/null")

  if not success then
    error(err)
  end
  return changed
end

return M
