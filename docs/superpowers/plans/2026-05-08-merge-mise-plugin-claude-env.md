# Merge mise-claude Plugin + claude-env CLI — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Slim the Lua mise plugin to a ~50-line bootstrap that installs the `claude-env` binary via `cargo install`, and promote `claude-env` + `claude-env.toml` as the sole Claude tooling manager with audit/drift detection.

**Architecture:** The Lua plugin loses all Claude-specific logic (npm installs, .mcp.json writes, registry, aliases) and becomes a thin bootstrap: `backend_install.lua` runs `cargo install claude-env`, `backend_exec_env.lua` calls `claude-env install --idempotent --quiet` on shell entry, `backend_list_versions.lua` queries crates.io. The Rust CLI gains two new capabilities: `--quiet`/`--idempotent` flags on `install` (for silent shell-hook use) and a `migrate` subcommand that reads `.mise.toml` `claude:*` entries and writes `claude-env.toml`.

**Tech Stack:** Rust (clap 4, toml 0.8, tempfile for tests), Lua (mise backend hook system), GitHub Actions (cargo publish).

---

## File Map

### Created
- `claude-env/src/migrate.rs` — parse `.mise.toml`, route `claude:*` entries, write `claude-env.toml`
- `claude-env/tests/unit/reporter_test.rs` — unit tests for quiet Reporter
- `claude-env/tests/unit/migrate_test.rs` — unit tests for migrate logic
- `.github/workflows/publish.yml` — cargo publish on `v*` tag push
- `sample/mcp/claude-env.toml`
- `sample/skillssh/claude-env.toml`
- `sample/plugin/context7/claude-env.toml`
- `sample/plugin/visual-explainer/claude-env.toml`
- `sample/plugin/chrome-dev-tools/claude-env.toml`
- `sample/spec/bmad/claude-env.toml`
- `sample/spec/gsd/claude-env.toml`

### Modified
- `claude-env/src/cli.rs` — add `quiet`/`idempotent` to `Install`; add `Migrate` variant
- `claude-env/src/output.rs` — add `quiet: bool` to `Reporter`
- `claude-env/src/main.rs` — wire flags through; add `Migrate` arm
- `claude-env/src/lib.rs` — expose `pub mod migrate`
- `claude-env/tests/unit.rs` — register new test modules
- `hooks/backend_install.lua` — full rewrite (~30 lines)
- `hooks/backend_exec_env.lua` — full rewrite (~15 lines)
- `hooks/backend_list_versions.lua` — full rewrite (~30 lines)
- `sample/*/. mise.toml` — remove `claude:*` entries, add `claude = "latest"`
- `README.md` — update workflow description

### Deleted
- `lib/aliases.lua`
- `lib/registry.lua`
- `lib/mcp_config.lua`
- `lib/utils.lua`

---

## Task 1: Add --quiet and --idempotent flags to claude-env install

**Files:**
- Modify: `claude-env/src/cli.rs`
- Modify: `claude-env/src/output.rs`
- Modify: `claude-env/src/main.rs`
- Create: `claude-env/tests/unit/reporter_test.rs`
- Modify: `claude-env/tests/unit.rs`

- [ ] **Step 1: Write the failing test**

Create `claude-env/tests/unit/reporter_test.rs`:

```rust
use claude_env::output::Reporter;

#[test]
fn quiet_reporter_skip_increments_counter_without_panic() {
    let mut reporter = Reporter::new_quiet();
    reporter.skip("context7", "2.1.4");
    assert_eq!(reporter.skipped, 1);
    assert_eq!(reporter.installed, 0);
    assert_eq!(reporter.failed, 0);
}

#[test]
fn quiet_reporter_exit_code_is_zero_when_all_skipped() {
    let mut reporter = Reporter::new_quiet();
    reporter.skip("context7", "2.1.4");
    assert_eq!(reporter.exit_code(), 0);
}

#[test]
fn quiet_reporter_exit_code_is_one_when_failed() {
    let mut reporter = Reporter::new_quiet();
    reporter.failure("context7", "2.1.4", "npm not found");
    assert_eq!(reporter.exit_code(), 1);
}

#[test]
fn default_reporter_is_not_quiet() {
    let reporter = Reporter::new();
    // Just verify it constructs without panic — output goes to stdout
    assert_eq!(reporter.installed, 0);
}
```

- [ ] **Step 2: Register the test module in `claude-env/tests/unit.rs`**

Add after the last `#[path...]` line:

```rust
#[path = "unit/reporter_test.rs"]
mod reporter_test;
```

- [ ] **Step 3: Run test to verify it fails**

```bash
cd claude-env && cargo test reporter_test 2>&1 | head -20
```

Expected: error — `Reporter::new_quiet` does not exist.

- [ ] **Step 4: Add `quiet` field to `Reporter` in `claude-env/src/output.rs`**

Replace the entire file:

```rust
pub struct Reporter {
    pub installed: u32,
    pub skipped: u32,
    pub failed: u32,
    quiet: bool,
}

impl Reporter {
    pub fn new() -> Self {
        Self { installed: 0, skipped: 0, failed: 0, quiet: false }
    }

    pub fn new_quiet() -> Self {
        Self { installed: 0, skipped: 0, failed: 0, quiet: true }
    }

    pub fn success(&mut self, name: &str, version: &str, detail: &str) {
        self.installed += 1;
        println!("  \x1b[32m✓\x1b[0m {:<25} {} {}", name, version, detail);
    }

    pub fn failure(&mut self, name: &str, version: &str, error: &str) {
        self.failed += 1;
        println!("  \x1b[31m✗\x1b[0m {:<25} {} failed", name, version);
        for line in error.lines() {
            println!("    \x1b[90m│\x1b[0m {}", line);
        }
    }

    pub fn skip(&mut self, name: &str, version: &str) {
        self.skipped += 1;
        if !self.quiet {
            println!("  \x1b[90m⊘\x1b[0m {:<25} {} skipped", name, version);
        }
    }

    pub fn summary(&self) {
        if self.quiet && self.installed == 0 && self.failed == 0 {
            return;
        }
        println!();
        println!(
            "  {} installed, {} failed, {} skipped",
            self.installed, self.failed, self.skipped
        );
    }

    pub fn exit_code(&self) -> i32 {
        if self.failed > 0 { 1 } else { 0 }
    }
}

impl Default for Reporter {
    fn default() -> Self {
        Self::new()
    }
}
```

- [ ] **Step 5: Add flags to `Install` in `claude-env/src/cli.rs`**

Replace the `Install` variant:

```rust
/// Install all tools declared in `claude-env.toml`.
Install {
    /// Suppress output when nothing changed (for shell hook use).
    #[arg(long)]
    quiet: bool,

    /// Document intent: all installs are idempotent by default (resolver skips already-installed tools).
    #[arg(long)]
    idempotent: bool,
},
```

- [ ] **Step 6: Wire flags through in `claude-env/src/main.rs`**

Replace the `Command::Install` match arm:

```rust
Command::Install { quiet, idempotent } => {
    run_install(cli.verbose, quiet || idempotent);
}
```

Replace the `run_install` signature and reporter construction:

```rust
fn run_install(verbose: bool, quiet: bool) {
    // ... existing body unchanged except:
    let mut reporter = if quiet {
        Reporter::new_quiet()
    } else {
        Reporter::new()
    };
    // rest of body unchanged
```

- [ ] **Step 7: Run tests to verify they pass**

```bash
cd claude-env && cargo test reporter_test 2>&1
```

Expected: all 4 tests pass.

- [ ] **Step 8: Run full test suite to check for regressions**

```bash
cd claude-env && cargo test 2>&1 | tail -10
```

Expected: all tests pass, 0 failures.

- [ ] **Step 9: Commit**

```bash
git add claude-env/src/cli.rs claude-env/src/output.rs claude-env/src/main.rs \
        claude-env/tests/unit/reporter_test.rs claude-env/tests/unit.rs
git commit -m "feat(claude-env): add --quiet and --idempotent flags to install command"
```

---

## Task 2: Implement claude-env migrate

**Files:**
- Create: `claude-env/src/migrate.rs`
- Modify: `claude-env/src/lib.rs`
- Modify: `claude-env/src/cli.rs`
- Modify: `claude-env/src/main.rs`
- Create: `claude-env/tests/unit/migrate_test.rs`
- Modify: `claude-env/tests/unit.rs`

- [ ] **Step 1: Write the failing tests**

Create `claude-env/tests/unit/migrate_test.rs`:

```rust
use claude_env::migrate::migrate;
use std::fs;
use tempfile::TempDir;

#[test]
fn migrate_mcp_tool() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:mcp/context7\" = \"2.1.4\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(config.mcp.get("context7").map(String::as_str), Some("2.1.4"));
    assert!(config.skills.is_empty());
    assert!(config.plugins.is_empty());
    assert!(config.cli.is_empty());
}

#[test]
fn migrate_skills_sh_tool() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:skills.sh/vercel-labs/next-skills/next-best-practices\" = \"latest\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(
        config.skills.get("vercel-labs/next-skills/next-best-practices").map(String::as_str),
        Some("latest"),
    );
}

#[test]
fn migrate_plugin_tool() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:plugin/upstash/context7/context7-plugin@context7-marketplace\" = \"latest\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(
        config.plugins.get("upstash/context7/context7-plugin@context7-marketplace").map(String::as_str),
        Some("latest"),
    );
}

#[test]
fn migrate_spec_tool_goes_to_cli_section() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:spec/gsd\" = \"1.22.4\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(config.cli.get("gsd").map(String::as_str), Some("1.22.4"));
}

#[test]
fn migrate_cli_tool_goes_to_cli_section() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:cli/my-tool\" = \"3.0.0\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(config.cli.get("my-tool").map(String::as_str), Some("3.0.0"));
}

#[test]
fn migrate_ignores_non_claude_tools() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\nnode = \"22\"\n\"claude:mcp/context7\" = \"2.1.4\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(config.mcp.len(), 1);
    assert_eq!(config.mcp.get("context7").map(String::as_str), Some("2.1.4"));
}

#[test]
fn migrate_no_claude_tools_returns_error() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join(".mise.toml"), "[tools]\nnode = \"22\"\n").unwrap();
    assert!(migrate(dir.path()).is_err());
}

#[test]
fn migrate_missing_mise_toml_returns_error() {
    let dir = TempDir::new().unwrap();
    assert!(migrate(dir.path()).is_err());
}

#[test]
fn migrate_multiple_sections() {
    let dir = TempDir::new().unwrap();
    fs::write(
        dir.path().join(".mise.toml"),
        "[tools]\n\"claude:mcp/context7\" = \"2.1.4\"\n\"claude:skills.sh/vercel-labs/next-skills/next\" = \"latest\"\n",
    )
    .unwrap();
    let config = migrate(dir.path()).unwrap();
    assert_eq!(config.mcp.len(), 1);
    assert_eq!(config.skills.len(), 1);
}
```

- [ ] **Step 2: Register test module in `claude-env/tests/unit.rs`**

Add:

```rust
#[path = "unit/migrate_test.rs"]
mod migrate_test;
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd claude-env && cargo test migrate_test 2>&1 | head -20
```

Expected: compile error — `claude_env::migrate` does not exist.

- [ ] **Step 4: Create `claude-env/src/migrate.rs`**

```rust
use std::path::Path;

use crate::config::Config;

/// Parse `.mise.toml` `claude:*` tool entries and produce a `Config`
/// suitable for writing as `claude-env.toml`.
///
/// Returns an error if `.mise.toml` is missing, unparseable, or contains
/// no `claude:` entries.
pub fn migrate(project_dir: &Path) -> Result<Config, Box<dyn std::error::Error>> {
    let path = project_dir.join(".mise.toml");
    let content = std::fs::read_to_string(&path)
        .map_err(|_| "no .mise.toml found in current directory")?;

    let raw: toml::Value = toml::from_str(&content)
        .map_err(|e| format!("failed to parse .mise.toml: {e}"))?;

    let tools = raw
        .get("tools")
        .and_then(|t| t.as_table())
        .ok_or("no [tools] section in .mise.toml")?;

    let mut config = Config::default();

    for (key, value) in tools {
        let version = match value {
            toml::Value::String(s) => s.clone(),
            _ => continue,
        };

        if let Some(rest) = key.strip_prefix("claude:mcp/") {
            config.mcp.insert(rest.to_string(), version);
        } else if let Some(rest) = key.strip_prefix("claude:skills.sh/") {
            config.skills.insert(rest.to_string(), version);
        } else if let Some(rest) = key.strip_prefix("claude:plugin/") {
            config.plugins.insert(rest.to_string(), version);
        } else if let Some(rest) = key.strip_prefix("claude:spec/") {
            config.cli.insert(rest.to_string(), version);
        } else if let Some(rest) = key.strip_prefix("claude:cli/") {
            config.cli.insert(rest.to_string(), version);
        }
    }

    let total = config.mcp.len() + config.skills.len() + config.plugins.len() + config.cli.len();
    if total == 0 {
        return Err("no claude: tool entries found in .mise.toml [tools]".into());
    }

    Ok(config)
}

/// Serialize `config` and write it to `<project_dir>/claude-env.toml`.
pub fn write_claude_env_toml(
    config: &Config,
    project_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = project_dir.join("claude-env.toml");
    let content = toml::to_string_pretty(config)?;
    std::fs::write(path, content)?;
    Ok(())
}
```

- [ ] **Step 5: Add `pub mod migrate` to `claude-env/src/lib.rs`**

Add after the last `pub mod` line:

```rust
pub mod migrate;
```

- [ ] **Step 6: Add `Migrate` variant to `claude-env/src/cli.rs`**

Add inside the `Command` enum, after `Remove { ... }`:

```rust
/// Migrate Claude tool declarations from .mise.toml to claude-env.toml.
///
/// Reads the current directory's .mise.toml, finds all `claude:mcp/*`,
/// `claude:skills.sh/*`, `claude:plugin/*`, and `claude:spec/*` entries,
/// and writes a `claude-env.toml` with equivalent declarations.
Migrate,
```

- [ ] **Step 7: Add `Migrate` arm to `claude-env/src/main.rs`**

Add the import at the top (with the existing `use` statements):

```rust
use claude_env::migrate;
```

Add the match arm inside `match cli.command { ... }`:

```rust
Command::Migrate => {
    let project_dir = PathBuf::from(".");
    let total;
    let config = match migrate::migrate(&project_dir) {
        Ok(c) => {
            total = c.mcp.len() + c.cli.len() + c.skills.len() + c.plugins.len();
            c
        }
        Err(e) => {
            eprintln!("error: {e}");
            process::exit(2);
        }
    };
    if let Err(e) = migrate::write_claude_env_toml(&config, &project_dir) {
        eprintln!("error: failed to write claude-env.toml: {e}");
        process::exit(2);
    }
    println!("✓ Found {} claude: tools in .mise.toml", total);
    println!("✓ Written claude-env.toml");
    println!("→ Remove claude:mcp/*, claude:skills.sh/*, claude:plugin/*, claude:spec/* from .mise.toml");
    println!("→ Keep `claude = \"latest\"` — that installs the claude-env binary itself");
}
```

- [ ] **Step 8: Run tests to verify they pass**

```bash
cd claude-env && cargo test migrate_test 2>&1
```

Expected: all 9 tests pass.

- [ ] **Step 9: Run full test suite to check for regressions**

```bash
cd claude-env && cargo test 2>&1 | tail -10
```

Expected: all tests pass, 0 failures.

- [ ] **Step 10: Commit**

```bash
git add claude-env/src/migrate.rs claude-env/src/lib.rs claude-env/src/cli.rs \
        claude-env/src/main.rs claude-env/tests/unit/migrate_test.rs claude-env/tests/unit.rs
git commit -m "feat(claude-env): add migrate subcommand to convert .mise.toml to claude-env.toml"
```

---

## Task 3: Add cargo publish CI

**Files:**
- Create: `.github/workflows/publish.yml`

- [ ] **Step 1: Create `.github/workflows/publish.yml`**

```yaml
name: Publish to crates.io

on:
  push:
    tags:
      - 'v*'

jobs:
  publish:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5

      - name: Install Rust stable
        uses: dtolnay/rust-toolchain@stable

      - name: Run tests
        run: cargo test --manifest-path claude-env/Cargo.toml

      - name: Publish claude-env to crates.io
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
        run: cargo publish --manifest-path claude-env/Cargo.toml
```

- [ ] **Step 2: Commit**

```bash
git add .github/workflows/publish.yml
git commit -m "ci: add cargo publish workflow on v* tag push"
```

Note: add `CARGO_REGISTRY_TOKEN` to the repository secrets in GitHub settings before pushing the first tag.

---

## Task 4: Rewrite Lua plugin hooks and delete lib/

**Files:**
- Rewrite: `hooks/backend_list_versions.lua`
- Rewrite: `hooks/backend_install.lua`
- Rewrite: `hooks/backend_exec_env.lua`
- Delete: `lib/` (all four files)

- [ ] **Step 1: Rewrite `hooks/backend_list_versions.lua`**

Replace entire file:

```lua
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
```

- [ ] **Step 2: Rewrite `hooks/backend_install.lua`**

Replace entire file:

```lua
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

  -- Sentinel so mise considers the tool installed.
  local f = io.open(ctx.install_path .. "/.installed", "w")
  if f then f:write("1") f:close() end

  return {}
end
```

- [ ] **Step 3: Rewrite `hooks/backend_exec_env.lua`**

Replace entire file:

```lua
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
```

- [ ] **Step 4: Delete the lib/ directory**

```bash
rm -rf lib/
```

Run from the repo root (the directory containing `hooks/`, `metadata.lua`, `claude-env/`).

- [ ] **Step 5: Verify the plugin still has correct metadata**

```bash
cat metadata.lua
```

Expected: `metadata.lua` unchanged, references no lib/ files.

- [ ] **Step 6: Commit**

```bash
git add hooks/backend_list_versions.lua hooks/backend_install.lua hooks/backend_exec_env.lua
git rm lib/aliases.lua lib/registry.lua lib/mcp_config.lua lib/utils.lua
git commit -m "refactor(mise-plugin): slim to bootstrap-only — delegate all installs to claude-env"
```

---

## Task 5: Update samples and README

**Files:**
- Create: `sample/mcp/claude-env.toml`
- Modify: `sample/mcp/.mise.toml`
- Create: `sample/skillssh/claude-env.toml`
- Modify: `sample/skillssh/.mise.toml`
- Create: `sample/plugin/context7/claude-env.toml`
- Modify: `sample/plugin/context7/.mise.toml`
- Create: `sample/plugin/visual-explainer/claude-env.toml`
- Modify: `sample/plugin/visual-explainer/.mise.toml`
- Create: `sample/plugin/chrome-dev-tools/claude-env.toml`
- Modify: `sample/plugin/chrome-dev-tools/.mise.toml`
- Create: `sample/spec/bmad/claude-env.toml`
- Modify: `sample/spec/bmad/.mise.toml`
- Create: `sample/spec/gsd/claude-env.toml`
- Modify: `sample/spec/gsd/.mise.toml`
- Modify: `README.md`

- [ ] **Step 1: Update the mcp sample**

Create `sample/mcp/claude-env.toml`:

```toml
[mcp]
context7 = "2.1.4"
chrome-devtools = "0.20.0"
shadcn = "4.0.6"
```

Replace `sample/mcp/.mise.toml` with (keep tasks, replace tools):

```toml
[tools]
claude-code = "latest"
claude = "latest"

[tasks.test]
description = "Run integration test"
run = "bash test.sh"

[tasks.clean]
description = "Remove install artifacts and uninstall tools"
run = """
mise uninstall claude@latest || true
rm -rfv .mcp.json claude-env.lock
"""
```

- [ ] **Step 2: Update the skillssh sample**

Create `sample/skillssh/claude-env.toml`:

```toml
[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"
"vercel-labs/next-skills/next-cache-components" = "latest"
"vercel-labs/next-skills/next-upgrade" = "latest"
"vercel-labs/agent-skills/web-design-guidelines" = "latest"
```

Replace `sample/skillssh/.mise.toml`:

```toml
[tools]
claude-code = "latest"
claude = "latest"

[tasks.test]
description = "Run integration test"
run = "bash test.sh"

[tasks.clean]
description = "Remove install artifacts and uninstall tools"
run = """
mise uninstall claude@latest || true
rm -rfv .agents/ .claude/skills/ skills-lock.json claude-env.lock
"""
```

- [ ] **Step 3: Update the plugin samples**

Create `sample/plugin/context7/claude-env.toml`:

```toml
[plugins]
"upstash/context7/context7-plugin@context7-marketplace" = "latest"
```

Replace `sample/plugin/context7/.mise.toml`:

```toml
[tools]
claude-code = "latest"
claude = "latest"

[tasks.test]
description = "Run integration test"
run = "bash test.sh"

[tasks.clean]
description = "Remove install artifacts and uninstall tools"
run = """
mise uninstall claude@latest || true
rm -rfv .claude/settings.json claude-env.lock
"""
```

Create `sample/plugin/visual-explainer/claude-env.toml`:

```toml
[plugins]
"nicobailon/visual-explainer/visual-explainer@visual-explainer-marketplace" = "latest"
```

Replace `sample/plugin/visual-explainer/.mise.toml`:

```toml
[tools]
claude-code = "latest"
claude = "latest"

[tasks.test]
description = "Run integration test"
run = "bash test.sh"

[tasks.clean]
description = "Remove install artifacts and uninstall tools"
run = """
mise uninstall claude@latest || true
rm -rfv .claude/settings.json claude-env.lock
"""
```

Create `sample/plugin/chrome-dev-tools/claude-env.toml`:

```toml
[plugins]
"ChromeDevTools/chrome-devtools-mcp/chrome-devtools-mcp@chrome-devtools-plugins" = "latest"
```

Replace `sample/plugin/chrome-dev-tools/.mise.toml`:

```toml
[tools]
claude-code = "latest"
claude = "latest"

[tasks.test]
description = "Run integration test"
run = "bash test.sh"

[tasks.clean]
description = "Remove install artifacts and uninstall tools"
run = """
mise uninstall claude@latest || true
rm -rfv .claude/settings.json claude-env.lock
"""
```

- [ ] **Step 4: Update the spec samples**

Create `sample/spec/bmad/claude-env.toml`:

```toml
[cli]
bmad = "6.1.0"
```

Replace `sample/spec/bmad/.mise.toml` (keep `java`, `kotlin`, `gradle`, remove claude:spec):

```toml
[tools]
java = "25"
kotlin = "2.3.10"
gradle = "9.3.1"
claude-code = "latest"
claude = "latest"

[tasks.test]
description = "Run integration test"
run = "bash test.sh"

[tasks.clean]
description = "Remove install artifacts and uninstall tools"
run = """
mise uninstall claude@latest || true
rm -rfv .claude/ _bmad/ node_modules/ claude-env.lock
"""
```

Create `sample/spec/gsd/claude-env.toml`:

```toml
[cli]
gsd = "1.22.4"
```

Replace `sample/spec/gsd/.mise.toml` (same pattern — keep java/kotlin/gradle, remove claude:spec).

- [ ] **Step 5: Update README.md**

Find the section describing tool declaration and replace with the new workflow. The updated quick-start should read:

```markdown
## Quick start

**1. Bootstrap claude-env via mise:**
\`\`\`toml
# .mise.toml
[tools]
claude = "latest"
\`\`\`
\`\`\`bash
mise install
\`\`\`

**2. Declare Claude tools in `claude-env.toml`:**
\`\`\`toml
[mcp]
context7 = "2.1.4"

[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"

[plugins]
"upstash/context7/context7-plugin@context7-marketplace" = "latest"

[cli]
gsd = "1.22.4"
\`\`\`

**3. Install (auto-runs on shell entry after step 1):**
\`\`\`bash
claude-env install
\`\`\`

**4. Audit your environment:**
\`\`\`bash
claude-env inspect
\`\`\`

**Migrating from the old mise plugin?**
\`\`\`bash
claude-env migrate   # reads .mise.toml claude:* entries → writes claude-env.toml
\`\`\`
```

- [ ] **Step 6: Commit**

```bash
git add sample/ README.md
git commit -m "docs: update samples and README to use claude-env.toml instead of .mise.toml"
```
