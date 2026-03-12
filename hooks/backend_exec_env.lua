--- Set up environment variables so MCP server binaries are available in PATH.
--- @param ctx table { tool: string, version: string, install_path: string }
--- @return table { env_vars: table<string, string> }
function PLUGIN:BackendExecEnv(ctx)
  local aliases = require("lib/aliases")

  if aliases.tool_kind(ctx.tool) ~= "npm" then
    return { env_vars = {} }
  end

  local bin_dir = ctx.install_path .. "/node_modules/.bin"

  return {
    env_vars = {
      {
        key = "PATH",
        value = bin_dir,
      },
    },
  }
end
