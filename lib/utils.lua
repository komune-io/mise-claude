-- =============================================================================
-- Shared utility functions
-- =============================================================================

local M = {}

--- Escape a string for use inside single quotes in shell commands.
--- @param s string
--- @return string
function M.shell_quote(s)
  return "'" .. s:gsub("'", "'\\''") .. "'"
end

--- Resolve ${VAR} references in a string.
--- @param s string
--- @param project_root string
--- @return string
function M.resolve_vars(s, project_root)
  return s:gsub("%${([^}]+)}", function(var)
    if var == "PROJECT_ROOT" then
      return project_root
    end
    return os.getenv(var) or ""
  end)
end

--- Resolve the project root directory.
--- Always uses pwd to reflect the actual directory where the user invoked mise.
--- MISE_PROJECT_ROOT is not used because it can point to a parent directory
--- when running inside `mise run` tasks.
--- @param cmd table|nil the cmd module (optional)
--- @return string
function M.get_project_root(cmd)
  if cmd then
    return cmd.exec("pwd"):gsub("%s+$", "")
  end
  return ""
end

--- Ensure a directory exists, creating it if needed.
--- @param path string
function M.ensure_dir(path)
  os.execute("mkdir -p " .. M.shell_quote(path))
end

--- Check if a file exists.
--- @param path string
--- @return boolean
function M.file_exists(path)
  local f = io.open(path, "r")
  if f then
    f:close()
    return true
  end
  return false
end

--- Read the contents of a file.
--- @param path string
--- @return string|nil
function M.read_file(path)
  local f = io.open(path, "r")
  if not f then
    return nil
  end
  local content = f:read("*a")
  f:close()
  return content
end

--- Write content to a file.
--- @param path string
--- @param content string
function M.write_file(path, content)
  local f = io.open(path, "w")
  if not f then
    error("Cannot write to " .. path)
  end
  f:write(content)
  f:close()
end

return M
