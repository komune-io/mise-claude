-- Semver helpers duplicated from backend_install.lua — keep in sync.
--- Parse a semver string into a list of numeric parts.
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

--- Returns true if semver string a is less than b.
local function version_lt(a, b)
  local pa, pb = parse_version(a), parse_version(b)
  for i = 1, math.max(#pa, #pb) do
    local va, vb = pa[i] or 0, pb[i] or 0
    if va ~= vb then return va < vb end
  end
  return false
end

--- Return available claude-env versions from crates.io, sorted ascending.
function PLUGIN:BackendListVersions(_ctx)
  local http = require("http")
  local json = require("json")

  local resp, err = http.get({
    url = "https://crates.io/api/v1/crates/claude-env/versions",
    headers = { ["User-Agent"] = "mise-claude/2.0" },
  })
  if err then error("Failed to fetch versions from crates.io: " .. err) end

  local data = json.decode(resp.body)
  local versions = {}
  for _, v in ipairs(data.versions) do
    if not v.yanked then
      table.insert(versions, v.num)
    end
  end

  table.sort(versions, version_lt)
  return { versions = versions }
end
