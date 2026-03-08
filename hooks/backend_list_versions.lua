--- Parse a version string into a table of numeric parts.
--- @param v string
--- @return number[]
local function parse_version(v)
  local parts = {}
  local base = v:match("^([%d%.]+)")
  if base then
    for p in base:gmatch("(%d+)") do
      table.insert(parts, tonumber(p))
    end
  end
  return parts
end

--- Compare two semver strings numerically.
--- @param a string
--- @param b string
--- @return boolean
local function compare_versions(a, b)
  local pa, pb = parse_version(a), parse_version(b)
  for i = 1, math.max(#pa, #pb) do
    local va, vb = pa[i] or 0, pb[i] or 0
    if va ~= vb then
      return va < vb
    end
  end
  return false
end

--- Query the npm registry for available versions of a package.
--- @param ctx table { tool: string }
--- @return table { versions: string[] }
local aliases = require("lib/aliases")
local resolve_alias = aliases.resolve_alias

function PLUGIN:BackendListVersions(ctx)
  local http = require("http")
  local json = require("json")

  if aliases.is_skills_sh(ctx.tool) or aliases.is_plugin(ctx.tool) then
    return { versions = { "latest" } }
  end

  local tool = resolve_alias(ctx.tool)
  if not tool or tool == "" then
    error("Tool name cannot be empty")
  end

  local url = "https://registry.npmjs.org/" .. tool

  local resp, err = http.get({
    url = url,
    headers = { ["Accept"] = "application/json" },
  })

  if err then
    error("Failed to fetch versions from npm: " .. err)
  end

  if resp.status_code ~= 200 then
    error("npm registry returned status " .. resp.status_code .. " for package '" .. tool .. "'")
  end

  local ok, data = pcall(json.decode, resp.body)
  if not ok or not data then
    error("Failed to parse npm registry response")
  end

  if not data.versions then
    error("No versions found for package '" .. tool .. "'")
  end

  local versions = {}
  for version, _ in pairs(data.versions) do
    -- Skip pre-release versions
    if not version:match("^%d+%.%d+%.%d+%-") then
      table.insert(versions, version)
    end
  end

  table.sort(versions, compare_versions)

  return { versions = versions }
end
