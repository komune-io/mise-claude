-- Semver helpers duplicated in backend_list_versions.lua — keep in sync.
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

--- Escape s for use inside single quotes in shell.
local function shell_quote(s)
  return "'" .. s:gsub("'", "'\\''") .. "'"
end

--- Fetch the latest non-yanked claude-env version from crates.io.
local function fetch_latest_version()
  local http = require("http")
  local json = require("json")
  local resp, err = http.get({
    url = "https://crates.io/api/v1/crates/claude-env/versions",
    headers = { ["User-Agent"] = "mise-claude/2.0" },
  })
  if err then error("Failed to fetch claude-env versions: " .. err) end
  local data = json.decode(resp.body)
  local versions = {}
  for _, v in ipairs(data.versions) do
    if not v.yanked then table.insert(versions, v.num) end
  end
  table.sort(versions, version_lt)
  if #versions == 0 then error("No versions found for claude-env on crates.io") end
  -- versions is sorted ascending; last element is the latest release
  return versions[#versions]
end

--- Install the claude-env binary via cargo.
function PLUGIN:BackendInstall(ctx)
  local cmd = require("cmd")

  local version = ctx.version
  if version == "latest" then
    version = fetch_latest_version()
  end

  cmd.exec(
    "cargo install claude-env"
    .. " --version " .. shell_quote(version)
    .. " --root " .. shell_quote(ctx.install_path)
    .. " --locked"
  )

  -- Write a sentinel file that mise checks to determine if the tool is
  -- installed. The actual idempotency check is done by cargo install --locked.
  local f = io.open(ctx.install_path .. "/.installed", "w")
  if f then
    f:write("1")
    f:close()
  end

  return {}
end
