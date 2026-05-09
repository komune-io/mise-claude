--- Escape s for use inside single quotes in shell.
local function shell_quote(s)
  return "'" .. s:gsub("'", "'\\''") .. "'"
end

--- Add chord to PATH and trigger idempotent install if chord.toml exists.
function PLUGIN:BackendExecEnv(ctx)
  local bin_dir = ctx.install_path .. "/bin"
  local bin = bin_dir .. "/chord"

  -- Trigger install only when chord.toml exists in the project root.
  -- Failures are swallowed so a broken config never breaks the shell.
  local ok, err = pcall(function()
    local cmd = require("cmd")
    -- Use pwd, not MISE_PROJECT_ROOT: MISE_PROJECT_ROOT can point to a parent
    -- directory when running inside `mise run` tasks, causing false positives.
    local project_root = cmd.exec("pwd"):gsub("%s+$", "")
    local f = io.open(project_root .. "/chord.toml", "r")
    if f then
      f:close()
      cmd.exec(
        "cd " .. shell_quote(project_root)
        .. " && " .. shell_quote(bin)
        .. " install --idempotent --quiet"
      )
    end
  end)
  if not ok then
    io.stderr:write("chord-plugin: chord install failed: " .. tostring(err) .. "\n")
  end

  return { env_vars = { { key = "PATH", value = bin_dir } } }
end
