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
    -- Skip silently if the chord binary is missing from this install dir.
    -- This handles legacy mise installs that predate the claude-env → chord
    -- rename: those dirs carry only the .installed sentinel without ever
    -- running `cargo install`. Firing chord install per shell entry would
    -- spam stderr for every stale tool the user has registered.
    local f_bin = io.open(bin, "r")
    if not f_bin then return end
    f_bin:close()

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
