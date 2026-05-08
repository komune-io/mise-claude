--- Escape s for use inside single quotes in shell.
local function shell_quote(s)
  return "'" .. s:gsub("'", "'\\''") .. "'"
end

--- Add claude-env to PATH and trigger idempotent install if claude-env.toml exists.
function PLUGIN:BackendExecEnv(ctx)
  local bin_dir = ctx.install_path .. "/bin"
  local bin = bin_dir .. "/claude-env"

  -- Trigger install only when claude-env.toml exists in the project root.
  -- Failures are swallowed so a broken config never breaks the shell.
  pcall(function()
    local cmd = require("cmd")
    local project_root = cmd.exec("pwd"):gsub("%s+$", "")
    local f = io.open(project_root .. "/claude-env.toml", "r")
    if f then
      f:close()
      cmd.exec(
        "cd " .. shell_quote(project_root)
        .. " && " .. shell_quote(bin)
        .. " install --idempotent --quiet"
      )
    end
  end)

  return { env_vars = { { key = "PATH", value = bin_dir } } }
end
