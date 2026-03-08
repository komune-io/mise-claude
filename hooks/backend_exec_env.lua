--- Set up environment variables so MCP server binaries are available in PATH.
--- @param ctx table { tool: string, version: string, install_path: string }
--- @return table { env_vars: table<string, string> }
function PLUGIN:BackendExecEnv(ctx)
  local aliases = require("lib/aliases")

  if aliases.is_skills_sh(ctx.tool) or aliases.is_plugin(ctx.tool) then
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
