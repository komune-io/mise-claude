# claude-env Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a standalone Rust CLI that reads `claude-env.toml`, resolves versions, installs Claude Code tools (MCP servers, skills, plugins, CLI tools), and manages a lockfile.

**Architecture:** Single Rust binary with sequential install pipeline. Modules: config parsing, lockfile management, npm registry client, tool installers (one per type), CLI command dispatch. No async — blocking I/O with `ureq` for HTTP and `std::process::Command` for shell commands.

**Tech Stack:** Rust, `clap` (CLI), `toml` + `serde` (config), `ureq` (HTTP), `serde_json` (JSON manipulation), `sha2` (integrity), `assert_cmd` + `assert_fs` (testing), `wiremock` (mock HTTP in tests)

---

## File Structure

```
claude-env/
├── Cargo.toml
├── src/
│   ├── main.rs                 # Entry point, clap CLI dispatch
│   ├── cli.rs                  # Clap command definitions
│   ├── config.rs               # Parse claude-env.toml into structs
│   ├── lockfile.rs             # Parse/write claude-env.lock
│   ├── registry.rs             # Built-in alias table + overrides
│   ├── resolver.rs             # Determine install actions (fresh/upgrade/skip)
│   ├── npm.rs                  # npm registry HTTP client (versions, integrity, changelog)
│   ├── installer/
│   │   ├── mod.rs              # Installer trait + dispatch
│   │   ├── mcp.rs              # npm install + .mcp.json write
│   │   ├── cli_tool.rs         # npm install + post_install execution
│   │   ├── skill.rs            # npx skills add
│   │   └── plugin.rs           # claude plugin marketplace add + install
│   ├── mcp_config.rs           # Read/merge/write .mcp.json
│   ├── output.rs               # Terminal output formatting (✓ ✗ ⊘, colors)
│   └── error.rs                # Error types
├── tests/
│   ├── unit/
│   │   ├── config_test.rs
│   │   ├── lockfile_test.rs
│   │   ├── registry_test.rs
│   │   ├── resolver_test.rs
│   │   └── npm_test.rs
│   ├── integration/
│   │   ├── mod.rs
│   │   ├── fixtures/           # Sample claude-env.toml files
│   │   │   ├── basic_mcp.toml
│   │   │   ├── full_config.toml
│   │   │   ├── invalid.toml
│   │   │   └── with_lockfile/
│   │   ├── shims/              # Mock scripts for npm, npx, claude
│   │   │   ├── npm
│   │   │   ├── npx
│   │   │   └── claude
│   │   ├── install_test.rs
│   │   ├── update_test.rs
│   │   └── remove_test.rs
│   └── e2e/
│       ├── Dockerfile
│       ├── docker-compose.yml
│       └── scenarios/
├── docs/
│   └── (spec lives in parent project)
└── README.md
```

---

## Task 1: Project Scaffold + Config Parsing

**Files:**
- Create: `claude-env/Cargo.toml`
- Create: `claude-env/src/main.rs`
- Create: `claude-env/src/cli.rs`
- Create: `claude-env/src/config.rs`
- Create: `claude-env/src/error.rs`
- Test: `claude-env/tests/unit/config_test.rs`

- [ ] **Step 1: Initialize Cargo project**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
cargo init claude-env
```

- [ ] **Step 2: Add dependencies to Cargo.toml**

```toml
[package]
name = "claude-env"
version = "0.1.0"
edition = "2021"
description = "Declarative Claude Code environment manager"
license = "MIT"

[dependencies]
clap = { version = "4", features = ["derive"] }
toml = "0.8"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[dev-dependencies]
assert_cmd = "2"
assert_fs = "1"
predicates = "3"
tempfile = "3"
```

- [ ] **Step 3: Write the failing test for config parsing**

Create `claude-env/tests/unit/config_test.rs`:

```rust
use claude_env::config::Config;

#[test]
fn parse_minimal_mcp_config() {
    let input = r#"
[mcp]
context7 = "2.1.4"
"#;
    let config = Config::parse(input).unwrap();
    assert_eq!(config.mcp.len(), 1);
    assert_eq!(config.mcp.get("context7").unwrap(), "2.1.4");
    assert!(config.skills.is_empty());
    assert!(config.plugins.is_empty());
    assert!(config.cli.is_empty());
}

#[test]
fn parse_full_config() {
    let input = r#"
[mcp]
context7 = "2.1.4"
chrome-devtools = "1.0.3"

[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"

[plugins]
"anthropics/claude-code/code-review@claude-code-plugins" = "latest"

[cli]
get-shit-done-cc = "1.22.4"
"#;
    let config = Config::parse(input).unwrap();
    assert_eq!(config.mcp.len(), 2);
    assert_eq!(config.skills.len(), 1);
    assert_eq!(config.plugins.len(), 1);
    assert_eq!(config.cli.len(), 1);
}

#[test]
fn parse_empty_config_is_valid() {
    let input = "";
    let config = Config::parse(input).unwrap();
    assert!(config.mcp.is_empty());
}

#[test]
fn parse_invalid_toml_returns_error() {
    let input = "[mcp\nbroken";
    let result = Config::parse(input);
    assert!(result.is_err());
}

#[test]
fn parse_unknown_section_returns_error() {
    let input = r#"
[unknown_section]
foo = "bar"
"#;
    let result = Config::parse(input);
    assert!(result.is_err());
}
```

- [ ] **Step 4: Run test to verify it fails**

```bash
cd claude-env && cargo test --test unit -- config_test
```

Expected: compilation error — `claude_env::config` doesn't exist.

- [ ] **Step 5: Implement config module**

Create `claude-env/src/config.rs`:

```rust
use serde::Deserialize;
use std::collections::BTreeMap;
use crate::error::ConfigError;

#[derive(Debug, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub mcp: BTreeMap<String, String>,
    #[serde(default)]
    pub skills: BTreeMap<String, String>,
    #[serde(default)]
    pub plugins: BTreeMap<String, String>,
    #[serde(default)]
    pub cli: BTreeMap<String, String>,
}

impl Config {
    pub fn parse(input: &str) -> Result<Self, ConfigError> {
        toml::from_str(input).map_err(ConfigError::Parse)
    }

    pub fn from_file(path: &std::path::Path) -> Result<Self, ConfigError> {
        let content = std::fs::read_to_string(path)
            .map_err(|e| ConfigError::Io(path.to_path_buf(), e))?;
        Self::parse(&content)
    }
}
```

Create `claude-env/src/error.rs`:

```rust
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("failed to parse config: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to read {0}: {1}")]
    Io(PathBuf, std::io::Error),
}
```

Create `claude-env/src/main.rs`:

```rust
pub mod cli;
pub mod config;
pub mod error;

fn main() {
    println!("claude-env: not yet implemented");
}
```

Update `claude-env/src/lib.rs` (create it for test access):

```rust
pub mod config;
pub mod error;
```

- [ ] **Step 6: Run tests to verify they pass**

```bash
cd claude-env && cargo test --test unit -- config_test
```

Expected: all 5 tests PASS.

- [ ] **Step 7: Implement CLI skeleton**

Create `claude-env/src/cli.rs`:

```rust
use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "claude-env", version, about = "Declarative Claude Code environment manager")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,

    /// Show verbose output (exact commands being run)
    #[arg(long, global = true)]
    pub verbose: bool,
}

#[derive(Subcommand)]
pub enum Command {
    /// Install tools from lockfile (or resolve + create lockfile if missing)
    Install,
    /// Check for updates and show changelogs
    Update {
        /// Specific tool to update (updates all if omitted)
        tool: Option<String>,
    },
    /// Show changelog between pinned and latest version
    Diff {
        /// Tool to diff
        tool: String,
    },
    /// Show installed tools and available updates
    List,
    /// Add a tool to config
    Add {
        /// Tool to add (e.g., "context7", "@org/pkg")
        tool: String,
    },
    /// Remove a tool from config and clean up
    Remove {
        /// Tool to remove
        tool: String,
    },
}
```

- [ ] **Step 8: Wire CLI into main.rs**

Update `claude-env/src/main.rs`:

```rust
mod cli;
pub mod config;
pub mod error;

use clap::Parser;
use cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Command::Install => {
            println!("install: not yet implemented");
        }
        Command::Update { tool } => {
            println!("update {:?}: not yet implemented", tool);
        }
        Command::Diff { tool } => {
            println!("diff {}: not yet implemented", tool);
        }
        Command::List => {
            println!("list: not yet implemented");
        }
        Command::Add { tool } => {
            println!("add {}: not yet implemented", tool);
        }
        Command::Remove { tool } => {
            println!("remove {}: not yet implemented", tool);
        }
    }
}
```

- [ ] **Step 9: Verify CLI compiles and shows help**

```bash
cd claude-env && cargo run -- --help
```

Expected: shows usage with install/update/diff/list/add/remove subcommands.

- [ ] **Step 10: Commit**

```bash
git add claude-env/
git commit -m "feat(claude-env): scaffold project with config parsing and CLI skeleton"
```

---

## Task 2: Registry Module (Alias Resolution + Overrides)

**Files:**
- Create: `claude-env/src/registry.rs`
- Test: `claude-env/tests/unit/registry_test.rs`

- [ ] **Step 1: Write failing tests for registry**

Create `claude-env/tests/unit/registry_test.rs`:

```rust
use claude_env::registry::{Registry, ToolOverride};

#[test]
fn resolve_known_alias() {
    let reg = Registry::default();
    assert_eq!(reg.resolve_alias("context7"), "@upstash/context7-mcp");
    assert_eq!(reg.resolve_alias("chrome-devtools"), "chrome-devtools-mcp");
    assert_eq!(reg.resolve_alias("shadcn"), "shadcn");
    assert_eq!(reg.resolve_alias("gsd"), "get-shit-done-cc");
    assert_eq!(reg.resolve_alias("bmad"), "bmad-method");
    assert_eq!(reg.resolve_alias("openspec"), "@fission-ai/openspec");
}

#[test]
fn unknown_alias_passes_through() {
    let reg = Registry::default();
    assert_eq!(reg.resolve_alias("@someorg/custom-mcp"), "@someorg/custom-mcp");
    assert_eq!(reg.resolve_alias("my-tool"), "my-tool");
}

#[test]
fn get_override_for_known_tool() {
    let reg = Registry::default();
    let ov = reg.get_override("shadcn").unwrap();
    assert_eq!(ov.bin_name.as_deref(), Some("shadcn"));
    assert!(ov.post_install.is_some());
    assert!(!ov.extra_deps.is_empty());
}

#[test]
fn get_override_returns_none_for_unknown() {
    let reg = Registry::default();
    assert!(reg.get_override("@someorg/custom-mcp").is_none());
}

#[test]
fn post_install_substitutes_project_root() {
    let reg = Registry::default();
    let ov = reg.get_override("bmad-method").unwrap();
    let cmd = ov.resolve_post_install("/my/project");
    assert!(cmd.unwrap().contains("/my/project"));
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd claude-env && cargo test --test unit -- registry_test
```

Expected: compilation error — `claude_env::registry` doesn't exist.

- [ ] **Step 3: Implement registry module**

Create `claude-env/src/registry.rs`:

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct ToolOverride {
    pub bin_name: Option<String>,
    pub post_install: Option<String>,
    pub extra_deps: Vec<String>,
}

impl ToolOverride {
    pub fn resolve_post_install(&self, project_root: &str) -> Option<String> {
        self.post_install
            .as_ref()
            .map(|cmd| cmd.replace("${PROJECT_ROOT}", project_root))
    }
}

pub struct Registry {
    aliases: HashMap<&'static str, &'static str>,
    overrides: HashMap<&'static str, ToolOverride>,
}

impl Default for Registry {
    fn default() -> Self {
        let mut aliases = HashMap::new();
        aliases.insert("context7", "@upstash/context7-mcp");
        aliases.insert("chrome-devtools", "chrome-devtools-mcp");
        aliases.insert("shadcn", "shadcn");
        aliases.insert("gsd", "get-shit-done-cc");
        aliases.insert("bmad", "bmad-method");
        aliases.insert("openspec", "@fission-ai/openspec");

        let mut overrides = HashMap::new();
        overrides.insert("shadcn", ToolOverride {
            bin_name: Some("shadcn".to_string()),
            post_install: Some("shadcn mcp init --client claude".to_string()),
            extra_deps: vec!["tinyexec@1.0.2".to_string()],
        });
        overrides.insert("get-shit-done-cc", ToolOverride {
            bin_name: None,
            post_install: Some("get-shit-done-cc --claude --local".to_string()),
            extra_deps: vec![],
        });
        overrides.insert("bmad-method", ToolOverride {
            bin_name: None,
            post_install: Some(
                "bmad-method install --directory ${PROJECT_ROOT} --modules bmm --tools claude-code --yes"
                    .to_string(),
            ),
            extra_deps: vec![],
        });
        overrides.insert("@fission-ai/openspec", ToolOverride {
            bin_name: None,
            post_install: Some("openspec init --tools claude".to_string()),
            extra_deps: vec![],
        });

        Self { aliases, overrides }
    }
}

impl Registry {
    pub fn resolve_alias(&self, name: &str) -> &str {
        self.aliases.get(name).copied().unwrap_or(name)
    }

    pub fn get_override(&self, package: &str) -> Option<&ToolOverride> {
        self.overrides.get(package)
    }
}
```

Add to `claude-env/src/lib.rs`:

```rust
pub mod config;
pub mod error;
pub mod registry;
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd claude-env && cargo test --test unit -- registry_test
```

Expected: all 5 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add claude-env/src/registry.rs claude-env/tests/unit/registry_test.rs claude-env/src/lib.rs
git commit -m "feat(claude-env): add registry module with alias resolution and tool overrides"
```

---

## Task 3: Lockfile Module

**Files:**
- Create: `claude-env/src/lockfile.rs`
- Test: `claude-env/tests/unit/lockfile_test.rs`

- [ ] **Step 1: Write failing tests**

Create `claude-env/tests/unit/lockfile_test.rs`:

```rust
use claude_env::lockfile::{Lockfile, LockedTool};

#[test]
fn parse_lockfile() {
    let input = r#"
[mcp.context7]
package = "@upstash/context7-mcp"
version = "2.1.4"
integrity = "sha512-abc123"

[skills."vercel-labs/next-skills/next-best-practices"]
version = "1.0.0"
resolved_at = "2026-04-20"
"#;
    let lock = Lockfile::parse(input).unwrap();
    let mcp = lock.get("mcp", "context7").unwrap();
    assert_eq!(mcp.version, "2.1.4");
    assert_eq!(mcp.package.as_deref(), Some("@upstash/context7-mcp"));
    assert_eq!(mcp.integrity.as_deref(), Some("sha512-abc123"));

    let skill = lock.get("skills", "vercel-labs/next-skills/next-best-practices").unwrap();
    assert_eq!(skill.version, "1.0.0");
    assert_eq!(skill.resolved_at.as_deref(), Some("2026-04-20"));
}

#[test]
fn serialize_lockfile_roundtrip() {
    let mut lock = Lockfile::new();
    lock.set("mcp", "context7", LockedTool {
        package: Some("@upstash/context7-mcp".to_string()),
        version: "2.1.4".to_string(),
        integrity: Some("sha512-abc123".to_string()),
        resolved_at: None,
    });

    let serialized = lock.serialize();
    let reparsed = Lockfile::parse(&serialized).unwrap();
    let tool = reparsed.get("mcp", "context7").unwrap();
    assert_eq!(tool.version, "2.1.4");
    assert_eq!(tool.integrity.as_deref(), Some("sha512-abc123"));
}

#[test]
fn empty_lockfile() {
    let lock = Lockfile::new();
    let serialized = lock.serialize();
    assert!(serialized.starts_with("# Auto-generated by claude-env."));
}

#[test]
fn lockfile_from_missing_file_returns_empty() {
    let lock = Lockfile::from_file(std::path::Path::new("/nonexistent/claude-env.lock"));
    assert!(lock.is_ok());
    assert!(lock.unwrap().is_empty());
}
```

- [ ] **Step 2: Run test to verify it fails**

```bash
cd claude-env && cargo test --test unit -- lockfile_test
```

Expected: compilation error — `claude_env::lockfile` doesn't exist.

- [ ] **Step 3: Implement lockfile module**

Create `claude-env/src/lockfile.rs`:

```rust
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedTool {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub package: Option<String>,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub integrity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolved_at: Option<String>,
}

#[derive(Debug, Default)]
pub struct Lockfile {
    sections: BTreeMap<String, BTreeMap<String, LockedTool>>,
}

impl Lockfile {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.sections.is_empty()
    }

    pub fn parse(input: &str) -> Result<Self, toml::de::Error> {
        let raw: BTreeMap<String, BTreeMap<String, LockedTool>> = if input.trim().is_empty() {
            BTreeMap::new()
        } else {
            toml::from_str(input)?
        };
        Ok(Self { sections: raw })
    }

    pub fn from_file(path: &Path) -> Result<Self, toml::de::Error> {
        match std::fs::read_to_string(path) {
            Ok(content) => Self::parse(&content),
            Err(_) => Ok(Self::new()),
        }
    }

    pub fn get(&self, section: &str, name: &str) -> Option<&LockedTool> {
        self.sections.get(section)?.get(name)
    }

    pub fn set(&mut self, section: &str, name: &str, tool: LockedTool) {
        self.sections
            .entry(section.to_string())
            .or_default()
            .insert(name.to_string(), tool);
    }

    pub fn serialize(&self) -> String {
        let mut out = String::from("# Auto-generated by claude-env. Do not edit.\n\n");
        if !self.sections.is_empty() {
            out.push_str(&toml::to_string_pretty(&self.sections).unwrap_or_default());
        }
        out
    }

    pub fn write_to_file(&self, path: &Path) -> std::io::Result<()> {
        std::fs::write(path, self.serialize())
    }
}
```

Add to `claude-env/src/lib.rs`:

```rust
pub mod config;
pub mod error;
pub mod lockfile;
pub mod registry;
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd claude-env && cargo test --test unit -- lockfile_test
```

Expected: all 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add claude-env/src/lockfile.rs claude-env/tests/unit/lockfile_test.rs claude-env/src/lib.rs
git commit -m "feat(claude-env): add lockfile parsing and serialization"
```

---

## Task 4: npm Registry Client

**Files:**
- Create: `claude-env/src/npm.rs`
- Test: `claude-env/tests/unit/npm_test.rs`

- [ ] **Step 1: Add ureq + sha2 dependencies**

Add to `claude-env/Cargo.toml` under `[dependencies]`:

```toml
ureq = { version = "3", features = ["json"] }
sha2 = "0.10"
```

- [ ] **Step 2: Write failing tests**

Create `claude-env/tests/unit/npm_test.rs`:

```rust
use claude_env::npm::{NpmClient, PackageMetadata};

#[test]
fn parse_registry_response() {
    let json = serde_json::json!({
        "versions": {
            "1.0.0": { "dist": { "shasum": "aaa", "integrity": "sha512-abc" } },
            "1.1.0": { "dist": { "shasum": "bbb", "integrity": "sha512-def" } },
            "2.0.0-beta.1": { "dist": { "shasum": "ccc", "integrity": "sha512-ghi" } },
            "2.0.0": { "dist": { "shasum": "ddd", "integrity": "sha512-jkl" } }
        }
    });
    let meta = PackageMetadata::from_json(json).unwrap();
    let versions = meta.stable_versions();
    assert_eq!(versions, vec!["1.0.0", "1.1.0", "2.0.0"]);
}

#[test]
fn latest_stable_version() {
    let json = serde_json::json!({
        "versions": {
            "1.0.0": { "dist": { "integrity": "sha512-abc" } },
            "1.1.0": { "dist": { "integrity": "sha512-def" } },
            "2.0.0-rc.1": { "dist": { "integrity": "sha512-ghi" } }
        }
    });
    let meta = PackageMetadata::from_json(json).unwrap();
    assert_eq!(meta.latest_stable(), Some("1.1.0".to_string()));
}

#[test]
fn integrity_for_version() {
    let json = serde_json::json!({
        "versions": {
            "1.0.0": { "dist": { "integrity": "sha512-abc123" } }
        }
    });
    let meta = PackageMetadata::from_json(json).unwrap();
    assert_eq!(meta.integrity_for("1.0.0"), Some("sha512-abc123".to_string()));
    assert_eq!(meta.integrity_for("9.9.9"), None);
}

#[test]
fn filter_prerelease_versions() {
    let json = serde_json::json!({
        "versions": {
            "1.0.0-alpha.1": { "dist": { "integrity": "sha512-a" } },
            "1.0.0-beta.2": { "dist": { "integrity": "sha512-b" } },
            "1.0.0": { "dist": { "integrity": "sha512-c" } }
        }
    });
    let meta = PackageMetadata::from_json(json).unwrap();
    assert_eq!(meta.stable_versions(), vec!["1.0.0"]);
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd claude-env && cargo test --test unit -- npm_test
```

Expected: compilation error — `claude_env::npm` doesn't exist.

- [ ] **Step 4: Implement npm module**

Create `claude-env/src/npm.rs`:

```rust
use serde_json::Value;
use std::collections::BTreeMap;
use crate::error::NpmError;

pub struct NpmClient {
    registry_url: String,
}

impl Default for NpmClient {
    fn default() -> Self {
        Self {
            registry_url: "https://registry.npmjs.org".to_string(),
        }
    }
}

impl NpmClient {
    pub fn with_registry(url: &str) -> Self {
        Self {
            registry_url: url.to_string(),
        }
    }

    pub fn fetch_metadata(&self, package: &str) -> Result<PackageMetadata, NpmError> {
        let url = format!("{}/{}", self.registry_url, package);
        let response: Value = ureq::get(&url)
            .call()
            .map_err(|e| NpmError::Request(package.to_string(), e.to_string()))?
            .body_mut()
            .read_json()
            .map_err(|e| NpmError::Parse(package.to_string(), e.to_string()))?;
        PackageMetadata::from_json(response)
    }
}

#[derive(Debug)]
struct VersionInfo {
    integrity: Option<String>,
}

#[derive(Debug)]
pub struct PackageMetadata {
    versions: BTreeMap<String, VersionInfo>,
}

impl PackageMetadata {
    pub fn from_json(json: Value) -> Result<Self, NpmError> {
        let versions_obj = json.get("versions")
            .and_then(|v| v.as_object())
            .ok_or_else(|| NpmError::Parse("unknown".to_string(), "missing 'versions' field".to_string()))?;

        let mut versions = BTreeMap::new();
        for (ver, data) in versions_obj {
            let integrity = data
                .get("dist")
                .and_then(|d| d.get("integrity"))
                .and_then(|i| i.as_str())
                .map(|s| s.to_string());
            versions.insert(ver.clone(), VersionInfo { integrity });
        }

        Ok(Self { versions })
    }

    pub fn stable_versions(&self) -> Vec<&str> {
        let mut result: Vec<&str> = self.versions.keys()
            .filter(|v| !is_prerelease(v))
            .map(|s| s.as_str())
            .collect();
        result.sort_by(|a, b| compare_semver(a, b));
        result
    }

    pub fn latest_stable(&self) -> Option<String> {
        self.stable_versions().last().map(|s| s.to_string())
    }

    pub fn integrity_for(&self, version: &str) -> Option<String> {
        self.versions.get(version)?.integrity.clone()
    }
}

fn is_prerelease(version: &str) -> bool {
    version.contains('-')
}

fn parse_semver_parts(v: &str) -> (u64, u64, u64) {
    let clean = v.split('-').next().unwrap_or(v);
    let parts: Vec<u64> = clean.split('.')
        .filter_map(|p| p.parse().ok())
        .collect();
    (
        parts.first().copied().unwrap_or(0),
        parts.get(1).copied().unwrap_or(0),
        parts.get(2).copied().unwrap_or(0),
    )
}

fn compare_semver(a: &str, b: &str) -> std::cmp::Ordering {
    let pa = parse_semver_parts(a);
    let pb = parse_semver_parts(b);
    pa.cmp(&pb)
}
```

Add to `claude-env/src/error.rs`:

```rust
#[derive(Error, Debug)]
pub enum NpmError {
    #[error("failed to fetch package '{0}': {1}")]
    Request(String, String),
    #[error("failed to parse metadata for '{0}': {1}")]
    Parse(String, String),
}
```

Add to `claude-env/src/lib.rs`:

```rust
pub mod config;
pub mod error;
pub mod lockfile;
pub mod npm;
pub mod registry;
```

- [ ] **Step 5: Run tests to verify they pass**

```bash
cd claude-env && cargo test --test unit -- npm_test
```

Expected: all 4 tests PASS.

- [ ] **Step 6: Commit**

```bash
git add claude-env/src/npm.rs claude-env/tests/unit/npm_test.rs claude-env/src/error.rs claude-env/src/lib.rs
git commit -m "feat(claude-env): add npm registry client with version parsing and integrity"
```

---

## Task 5: Resolver (Determine Install Actions)

**Files:**
- Create: `claude-env/src/resolver.rs`
- Test: `claude-env/tests/unit/resolver_test.rs`

- [ ] **Step 1: Write failing tests**

Create `claude-env/tests/unit/resolver_test.rs`:

```rust
use claude_env::config::Config;
use claude_env::lockfile::{Lockfile, LockedTool};
use claude_env::resolver::{resolve, Action, Plan};

#[test]
fn fresh_install_no_lockfile() {
    let config = Config::parse(r#"
[mcp]
context7 = "2.1.4"
"#).unwrap();
    let lockfile = Lockfile::new();
    let plan = resolve(&config, &lockfile, &installed_nothing);
    assert_eq!(plan.actions.len(), 1);
    assert!(matches!(plan.actions[0].action, Action::Install));
    assert_eq!(plan.actions[0].name, "context7");
    assert_eq!(plan.actions[0].version, "2.1.4");
}

#[test]
fn skip_when_lockfile_matches_installed() {
    let config = Config::parse(r#"
[mcp]
context7 = "2.1.4"
"#).unwrap();
    let mut lockfile = Lockfile::new();
    lockfile.set("mcp", "context7", LockedTool {
        package: Some("@upstash/context7-mcp".to_string()),
        version: "2.1.4".to_string(),
        integrity: Some("sha512-abc".to_string()),
        resolved_at: None,
    });
    let plan = resolve(&config, &lockfile, &|_, _| true);
    assert_eq!(plan.actions.len(), 1);
    assert!(matches!(plan.actions[0].action, Action::Skip));
}

#[test]
fn upgrade_when_config_version_differs_from_lock() {
    let config = Config::parse(r#"
[mcp]
context7 = "3.0.0"
"#).unwrap();
    let mut lockfile = Lockfile::new();
    lockfile.set("mcp", "context7", LockedTool {
        package: Some("@upstash/context7-mcp".to_string()),
        version: "2.1.4".to_string(),
        integrity: Some("sha512-abc".to_string()),
        resolved_at: None,
    });
    let plan = resolve(&config, &lockfile, &|_, _| true);
    assert_eq!(plan.actions.len(), 1);
    assert!(matches!(plan.actions[0].action, Action::Upgrade));
    assert_eq!(plan.actions[0].version, "3.0.0");
}

fn installed_nothing(_section: &str, _name: &str) -> bool {
    false
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd claude-env && cargo test --test unit -- resolver_test
```

Expected: compilation error.

- [ ] **Step 3: Implement resolver**

Create `claude-env/src/resolver.rs`:

```rust
use crate::config::Config;
use crate::lockfile::Lockfile;
use crate::registry::Registry;

#[derive(Debug, PartialEq)]
pub enum Action {
    Install,
    Upgrade,
    Skip,
}

#[derive(Debug)]
pub enum ToolType {
    Mcp,
    Cli,
    Skill,
    Plugin,
}

#[derive(Debug)]
pub struct PlannedAction {
    pub name: String,
    pub package: String,
    pub version: String,
    pub tool_type: ToolType,
    pub action: Action,
}

#[derive(Debug)]
pub struct Plan {
    pub actions: Vec<PlannedAction>,
}

/// `is_installed` callback: (section, name) -> bool
pub fn resolve(
    config: &Config,
    lockfile: &Lockfile,
    is_installed: &dyn Fn(&str, &str) -> bool,
) -> Plan {
    let registry = Registry::default();
    let mut actions = Vec::new();

    let sections: Vec<(&str, &std::collections::BTreeMap<String, String>, ToolType)> = vec![
        ("mcp", &config.mcp, ToolType::Mcp),
        ("cli", &config.cli, ToolType::Cli),
        ("skills", &config.skills, ToolType::Skill),
        ("plugins", &config.plugins, ToolType::Plugin),
    ];

    for (section, tools, tool_type) in sections {
        for (name, requested_version) in tools {
            let package = registry.resolve_alias(name).to_string();
            let locked = lockfile.get(section, name);

            let action = match locked {
                Some(lock_entry) if lock_entry.version == *requested_version => {
                    if is_installed(section, name) {
                        Action::Skip
                    } else {
                        Action::Install
                    }
                }
                Some(_) => Action::Upgrade,
                None => Action::Install,
            };

            actions.push(PlannedAction {
                name: name.clone(),
                package,
                version: requested_version.clone(),
                tool_type,
                action,
            });
        }
    }

    Plan { actions }
}
```

Add to `claude-env/src/lib.rs`:

```rust
pub mod config;
pub mod error;
pub mod lockfile;
pub mod npm;
pub mod registry;
pub mod resolver;
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd claude-env && cargo test --test unit -- resolver_test
```

Expected: all 3 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add claude-env/src/resolver.rs claude-env/tests/unit/resolver_test.rs claude-env/src/lib.rs
git commit -m "feat(claude-env): add resolver to determine install/upgrade/skip actions"
```

---

## Task 6: MCP Config Writer

**Files:**
- Create: `claude-env/src/mcp_config.rs`
- Test: `claude-env/tests/unit/mcp_config_test.rs` (inline in module)

- [ ] **Step 1: Write failing tests**

Tests inline in the module file. Create `claude-env/src/mcp_config.rs`:

```rust
// Implementation will go here

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn write_server_to_empty_mcp_json() {
        let dir = TempDir::new().unwrap();
        let project_root = dir.path();

        let entry = McpEntry {
            command: "/path/to/bin".to_string(),
            args: vec!["--stdio".to_string()],
        };
        ensure_server(project_root, "my-mcp", &entry).unwrap();

        let content = std::fs::read_to_string(project_root.join(".mcp.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert_eq!(json["mcpServers"]["my-mcp"]["command"], "/path/to/bin");
        assert_eq!(json["mcpServers"]["my-mcp"]["args"][0], "--stdio");
        assert_eq!(json["mcpServers"]["my-mcp"]["type"], "stdio");
    }

    #[test]
    fn merge_server_into_existing_mcp_json() {
        let dir = TempDir::new().unwrap();
        let project_root = dir.path();

        let existing = r#"{"mcpServers":{"existing":{"type":"stdio","command":"old"}}}"#;
        std::fs::write(project_root.join(".mcp.json"), existing).unwrap();

        let entry = McpEntry {
            command: "/new/bin".to_string(),
            args: vec![],
        };
        ensure_server(project_root, "new-server", &entry).unwrap();

        let content = std::fs::read_to_string(project_root.join(".mcp.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(json["mcpServers"]["existing"].is_object());
        assert!(json["mcpServers"]["new-server"].is_object());
    }

    #[test]
    fn skip_if_server_already_exists() {
        let dir = TempDir::new().unwrap();
        let project_root = dir.path();

        let entry = McpEntry {
            command: "/path/to/bin".to_string(),
            args: vec![],
        };
        ensure_server(project_root, "my-mcp", &entry).unwrap();
        // Second call should not error
        let added = ensure_server(project_root, "my-mcp", &entry).unwrap();
        assert!(!added);
    }

    #[test]
    fn remove_server() {
        let dir = TempDir::new().unwrap();
        let project_root = dir.path();

        let entry = McpEntry {
            command: "/path/to/bin".to_string(),
            args: vec![],
        };
        ensure_server(project_root, "my-mcp", &entry).unwrap();
        remove_server(project_root, "my-mcp").unwrap();

        let content = std::fs::read_to_string(project_root.join(".mcp.json")).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();
        assert!(json["mcpServers"]["my-mcp"].is_null());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

```bash
cd claude-env && cargo test mcp_config
```

Expected: compilation error — functions not implemented.

- [ ] **Step 3: Implement mcp_config**

Add implementation above the `#[cfg(test)]` block:

```rust
use serde_json::{json, Value};
use std::path::Path;

pub struct McpEntry {
    pub command: String,
    pub args: Vec<String>,
}

pub fn ensure_server(project_root: &Path, name: &str, entry: &McpEntry) -> std::io::Result<bool> {
    let mcp_path = project_root.join(".mcp.json");

    let mut config: Value = match std::fs::read_to_string(&mcp_path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or(json!({"mcpServers": {}})),
        Err(_) => json!({"mcpServers": {}}),
    };

    let servers = config
        .as_object_mut()
        .unwrap()
        .entry("mcpServers")
        .or_insert_with(|| json!({}));

    if servers.get(name).is_some() {
        return Ok(false);
    }

    servers.as_object_mut().unwrap().insert(
        name.to_string(),
        json!({
            "type": "stdio",
            "command": entry.command,
            "args": entry.args,
        }),
    );

    std::fs::write(&mcp_path, serde_json::to_string_pretty(&config)?)?;
    Ok(true)
}

pub fn remove_server(project_root: &Path, name: &str) -> std::io::Result<()> {
    let mcp_path = project_root.join(".mcp.json");

    let content = std::fs::read_to_string(&mcp_path)?;
    let mut config: Value = serde_json::from_str(&content).unwrap_or(json!({"mcpServers": {}}));

    if let Some(servers) = config.get_mut("mcpServers").and_then(|s| s.as_object_mut()) {
        servers.remove(name);
    }

    std::fs::write(&mcp_path, serde_json::to_string_pretty(&config)?)?;
    Ok(())
}
```

Add to `claude-env/src/lib.rs`:

```rust
pub mod mcp_config;
```

- [ ] **Step 4: Run tests to verify they pass**

```bash
cd claude-env && cargo test mcp_config
```

Expected: all 4 tests PASS.

- [ ] **Step 5: Commit**

```bash
git add claude-env/src/mcp_config.rs claude-env/src/lib.rs
git commit -m "feat(claude-env): add .mcp.json reader/writer with idempotent merge"
```

---

## Task 7: Installer Trait + MCP Installer

**Files:**
- Create: `claude-env/src/installer/mod.rs`
- Create: `claude-env/src/installer/mcp.rs`
- Test: `claude-env/tests/integration/install_test.rs`
- Create: `claude-env/tests/integration/shims/npm`

- [ ] **Step 1: Create shim for npm**

Create `claude-env/tests/integration/shims/npm` (executable):

```bash
#!/bin/bash
# Mock npm that logs invocations and creates expected directory structure
echo "$@" >> "${CLAUDE_ENV_TEST_LOG}/npm_calls.log"

# Simulate successful install: create node_modules/.bin with a binary
if [[ "$1" == "install" ]]; then
    prefix=""
    for arg in "$@"; do
        if [[ "$prev" == "--prefix" ]]; then
            prefix="$arg"
        fi
        prev="$arg"
    done
    if [[ -n "$prefix" ]]; then
        mkdir -p "$prefix/node_modules/.bin"
        # Create a fake binary named after the package (simplified)
        pkg=$(echo "$2" | sed 's/@.*//' | sed 's/.*\///')
        echo "#!/bin/bash" > "$prefix/node_modules/.bin/$pkg"
        chmod +x "$prefix/node_modules/.bin/$pkg"
    fi
fi
exit 0
```

- [ ] **Step 2: Write failing integration test**

Create `claude-env/tests/integration/install_test.rs`:

```rust
use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;

#[test]
fn install_single_mcp_tool() {
    let project = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    // Write config
    project.child("claude-env.toml").write_str(r#"
[mcp]
context7 = "2.1.4"
"#).unwrap();

    // Get path to shims
    let shims_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/integration/shims");

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("install")
        .current_dir(project.path())
        .env("PATH", format!("{}:{}", shims_dir.display(), std::env::var("PATH").unwrap()))
        .env("CLAUDE_ENV_HOME", packages.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path());

    cmd.assert().success();

    // Verify npm was called with correct args
    let npm_log = fs::read_to_string(log_dir.path().join("npm_calls.log")).unwrap();
    assert!(npm_log.contains("@upstash/context7-mcp@2.1.4"));
    assert!(npm_log.contains("--prefix"));

    // Verify .mcp.json was created
    let mcp_json = fs::read_to_string(project.path().join(".mcp.json")).unwrap();
    assert!(mcp_json.contains("context7-mcp"));
    assert!(mcp_json.contains("command"));

    // Verify lockfile was created
    let lockfile = fs::read_to_string(project.path().join("claude-env.lock")).unwrap();
    assert!(lockfile.contains("2.1.4"));
}

#[test]
fn install_idempotent_second_run_skips() {
    let project = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    project.child("claude-env.toml").write_str(r#"
[mcp]
context7 = "2.1.4"
"#).unwrap();

    let shims_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/integration/shims");

    let run = |log: &TempDir| {
        Command::cargo_bin("claude-env").unwrap()
            .arg("install")
            .current_dir(project.path())
            .env("PATH", format!("{}:{}", shims_dir.display(), std::env::var("PATH").unwrap()))
            .env("CLAUDE_ENV_HOME", packages.path())
            .env("CLAUDE_ENV_TEST_LOG", log.path())
            .assert()
            .success();
    };

    // First run installs
    run(&log_dir);

    // Second run skips
    let log_dir2 = TempDir::new().unwrap();
    run(&log_dir2);
    let npm_log = log_dir2.path().join("npm_calls.log");
    assert!(!npm_log.exists(), "npm should not be called on second run");
}
```

- [ ] **Step 3: Run tests to verify they fail**

```bash
cd claude-env && cargo test --test integration -- install_test
```

Expected: failure — installer not implemented yet.

- [ ] **Step 4: Implement installer trait**

Create `claude-env/src/installer/mod.rs`:

```rust
pub mod mcp;

use crate::error::InstallError;
use crate::resolver::PlannedAction;
use std::path::Path;

pub struct InstallContext<'a> {
    pub project_root: &'a Path,
    pub packages_dir: &'a Path,
    pub verbose: bool,
}

pub trait Installer {
    fn install(&self, action: &PlannedAction, ctx: &InstallContext) -> Result<InstallResult, InstallError>;
}

pub struct InstallResult {
    pub installed: bool,
    pub integrity: Option<String>,
}
```

- [ ] **Step 5: Implement MCP installer**

Create `claude-env/src/installer/mcp.rs`:

```rust
use crate::error::InstallError;
use crate::installer::{InstallContext, InstallResult, Installer};
use crate::mcp_config::{ensure_server, McpEntry};
use crate::registry::Registry;
use crate::resolver::PlannedAction;
use std::process::Command;

pub struct McpInstaller {
    registry: Registry,
}

impl Default for McpInstaller {
    fn default() -> Self {
        Self {
            registry: Registry::default(),
        }
    }
}

impl Installer for McpInstaller {
    fn install(&self, action: &PlannedAction, ctx: &InstallContext) -> Result<InstallResult, InstallError> {
        let pkg = &action.package;
        let version = &action.version;
        let install_dir = ctx.packages_dir.join(&action.name);

        // Build npm install command
        let mut args = vec![
            "install".to_string(),
            format!("{}@{}", pkg, version),
            "--prefix".to_string(),
            install_dir.display().to_string(),
            "--no-save".to_string(),
        ];

        // Add extra deps from registry
        if let Some(ov) = self.registry.get_override(pkg) {
            for dep in &ov.extra_deps {
                args.push(dep.clone());
            }
        }

        if ctx.verbose {
            eprintln!("  > npm {}", args.join(" "));
        }

        let output = Command::new("npm")
            .args(&args)
            .output()
            .map_err(|e| InstallError::Command("npm".to_string(), e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InstallError::Command(
                format!("npm install {}@{}", pkg, version),
                stderr.to_string(),
            ));
        }

        // Detect binary in node_modules/.bin
        let bin_dir = install_dir.join("node_modules/.bin");
        let bin_name = self.detect_binary(&bin_dir, pkg)?;
        let bin_path = bin_dir.join(&bin_name);

        // Write to .mcp.json
        let entry = McpEntry {
            command: bin_path.display().to_string(),
            args: vec![],
        };
        ensure_server(ctx.project_root, &bin_name, &entry)
            .map_err(|e| InstallError::Config(".mcp.json".to_string(), e.to_string()))?;

        Ok(InstallResult {
            installed: true,
            integrity: None, // TODO: fetch from npm registry in future
        })
    }
}

impl McpInstaller {
    fn detect_binary(&self, bin_dir: &std::path::Path, package: &str) -> Result<String, InstallError> {
        let registry = Registry::default();
        // Check registry for explicit bin_name
        if let Some(ov) = registry.get_override(package) {
            if let Some(ref name) = ov.bin_name {
                return Ok(name.clone());
            }
        }

        // Scan bin directory for first executable
        let entries = std::fs::read_dir(bin_dir)
            .map_err(|e| InstallError::Command("ls node_modules/.bin".to_string(), e.to_string()))?;

        for entry in entries.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if !name.starts_with('.') {
                return Ok(name);
            }
        }

        Err(InstallError::Command(
            format!("detect binary for {}", package),
            "no executables found in node_modules/.bin".to_string(),
        ))
    }
}
```

Add `InstallError` to `claude-env/src/error.rs`:

```rust
#[derive(Error, Debug)]
pub enum InstallError {
    #[error("{0}: {1}")]
    Command(String, String),
    #[error("failed to update {0}: {1}")]
    Config(String, String),
}
```

- [ ] **Step 6: Wire install command into main.rs**

Update `Command::Install` handler in `main.rs` to read config, resolve, and run installers:

```rust
Command::Install => {
    let project_root = std::env::current_dir().unwrap();
    let config_path = project_root.join("claude-env.toml");

    let config = match config::Config::from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(2);
        }
    };

    let packages_dir = std::env::var("CLAUDE_ENV_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".claude-env/packages"));

    let lock_path = project_root.join("claude-env.lock");
    let lockfile = lockfile::Lockfile::from_file(&lock_path).unwrap_or_default();

    let is_installed = |section: &str, name: &str| {
        // Check if package directory exists with node_modules
        let pkg_dir = packages_dir.join(name).join("node_modules");
        pkg_dir.exists()
    };

    let plan = resolver::resolve(&config, &lockfile, &is_installed);

    let ctx = installer::InstallContext {
        project_root: &project_root,
        packages_dir: &packages_dir,
        verbose: cli.verbose,
    };

    let mcp_installer = installer::mcp::McpInstaller::default();
    let mut new_lockfile = lockfile;
    let mut installed = 0u32;
    let mut skipped = 0u32;
    let mut failed = 0u32;

    for action in &plan.actions {
        match action.action {
            resolver::Action::Skip => {
                println!("  ⊘ {:<25} {} skipped", action.name, action.version);
                skipped += 1;
            }
            resolver::Action::Install | resolver::Action::Upgrade => {
                match mcp_installer.install(action, &ctx) {
                    Ok(result) => {
                        println!("  ✓ {:<25} {} installed", action.name, action.version);
                        new_lockfile.set("mcp", &action.name, lockfile::LockedTool {
                            package: Some(action.package.clone()),
                            version: action.version.clone(),
                            integrity: result.integrity,
                            resolved_at: None,
                        });
                        installed += 1;
                    }
                    Err(e) => {
                        println!("  ✗ {:<25} {} failed", action.name, action.version);
                        println!("    │ {}", e);
                        failed += 1;
                    }
                }
            }
        }
    }

    new_lockfile.write_to_file(&lock_path).unwrap();
    println!("\n  {} installed, {} failed, {} skipped", installed, failed, skipped);

    if failed > 0 {
        std::process::exit(1);
    }
}
```

- [ ] **Step 7: Run integration tests to verify they pass**

```bash
cd claude-env && cargo test --test integration -- install_test
```

Expected: both tests PASS.

- [ ] **Step 8: Commit**

```bash
git add claude-env/src/installer/ claude-env/tests/integration/ claude-env/src/main.rs claude-env/src/error.rs
git commit -m "feat(claude-env): implement MCP installer with integration tests"
```

---

## Task 8: CLI Tool Installer (with post_install)

**Files:**
- Create: `claude-env/src/installer/cli_tool.rs`
- Test: `claude-env/tests/integration/install_cli_test.rs`

- [ ] **Step 1: Write failing integration test**

Create `claude-env/tests/integration/install_cli_test.rs`:

```rust
use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;

#[test]
fn install_cli_tool_runs_post_install() {
    let project = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    project.child("claude-env.toml").write_str(r#"
[cli]
get-shit-done-cc = "1.22.4"
"#).unwrap();

    let shims_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/integration/shims");

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("install")
        .current_dir(project.path())
        .env("PATH", format!("{}:{}", shims_dir.display(), std::env::var("PATH").unwrap()))
        .env("CLAUDE_ENV_HOME", packages.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path());

    cmd.assert().success();

    // Verify npm was called
    let npm_log = fs::read_to_string(log_dir.path().join("npm_calls.log")).unwrap();
    assert!(npm_log.contains("get-shit-done-cc@1.22.4"));

    // Verify post_install was invoked (shim logs it)
    let post_log = fs::read_to_string(log_dir.path().join("post_install_calls.log")).unwrap();
    assert!(post_log.contains("get-shit-done-cc --claude --local"));

    // Verify .mcp.json was NOT created (cli type)
    assert!(!project.path().join(".mcp.json").exists());
}
```

- [ ] **Step 2: Update shim to log post_install calls**

Update `claude-env/tests/integration/shims/npm` to also handle the case where the fake binary is invoked (which acts as the post_install). Create a generic shim `claude-env/tests/integration/shims/get-shit-done-cc`:

```bash
#!/bin/bash
echo "$0 $@" >> "${CLAUDE_ENV_TEST_LOG}/post_install_calls.log"
exit 0
```

- [ ] **Step 3: Implement CLI tool installer**

Create `claude-env/src/installer/cli_tool.rs`:

```rust
use crate::error::InstallError;
use crate::installer::{InstallContext, InstallResult, Installer};
use crate::registry::Registry;
use crate::resolver::PlannedAction;
use std::process::Command;

pub struct CliToolInstaller {
    registry: Registry,
}

impl Default for CliToolInstaller {
    fn default() -> Self {
        Self {
            registry: Registry::default(),
        }
    }
}

impl Installer for CliToolInstaller {
    fn install(&self, action: &PlannedAction, ctx: &InstallContext) -> Result<InstallResult, InstallError> {
        let pkg = &action.package;
        let version = &action.version;
        let install_dir = ctx.packages_dir.join(&action.name);

        // npm install
        let mut args = vec![
            "install".to_string(),
            format!("{}@{}", pkg, version),
            "--prefix".to_string(),
            install_dir.display().to_string(),
            "--no-save".to_string(),
        ];

        if let Some(ov) = self.registry.get_override(pkg) {
            for dep in &ov.extra_deps {
                args.push(dep.clone());
            }
        }

        if ctx.verbose {
            eprintln!("  > npm {}", args.join(" "));
        }

        let output = Command::new("npm")
            .args(&args)
            .output()
            .map_err(|e| InstallError::Command("npm".to_string(), e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InstallError::Command(
                format!("npm install {}@{}", pkg, version),
                stderr.to_string(),
            ));
        }

        // Run post_install if configured
        if let Some(ov) = self.registry.get_override(pkg) {
            if let Some(cmd_str) = ov.resolve_post_install(&ctx.project_root.display().to_string()) {
                let bin_dir = install_dir.join("node_modules/.bin");
                let current_path = std::env::var("PATH").unwrap_or_default();
                let new_path = format!("{}:{}", bin_dir.display(), current_path);

                if ctx.verbose {
                    eprintln!("  > {}", cmd_str);
                }

                let output = Command::new("sh")
                    .args(["-c", &cmd_str])
                    .current_dir(ctx.project_root)
                    .env("PATH", &new_path)
                    .output()
                    .map_err(|e| InstallError::Command(cmd_str.clone(), e.to_string()))?;

                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    return Err(InstallError::Command(
                        format!("post_install: {}", cmd_str),
                        stderr.to_string(),
                    ));
                }
            }
        }

        Ok(InstallResult {
            installed: true,
            integrity: None,
        })
    }
}
```

Add to `claude-env/src/installer/mod.rs`:

```rust
pub mod cli_tool;
pub mod mcp;
```

- [ ] **Step 4: Wire CLI tool installer into main install dispatch**

Update the install command to route `ToolType::Cli` to `CliToolInstaller`:

```rust
use crate::installer::cli_tool::CliToolInstaller;

// In the install loop:
let result = match action.tool_type {
    resolver::ToolType::Mcp => mcp_installer.install(action, &ctx),
    resolver::ToolType::Cli => cli_installer.install(action, &ctx),
    _ => todo!("other installers"),
};
```

- [ ] **Step 5: Run integration test**

```bash
cd claude-env && cargo test --test integration -- install_cli_test
```

Expected: PASS.

- [ ] **Step 6: Commit**

```bash
git add claude-env/src/installer/cli_tool.rs claude-env/tests/integration/install_cli_test.rs claude-env/tests/integration/shims/
git commit -m "feat(claude-env): implement CLI tool installer with post_install support"
```

---

## Task 9: Skill Installer

**Files:**
- Create: `claude-env/src/installer/skill.rs`
- Create: `claude-env/tests/integration/shims/npx`
- Test: `claude-env/tests/integration/install_skill_test.rs`

- [ ] **Step 1: Create npx shim**

Create `claude-env/tests/integration/shims/npx` (executable):

```bash
#!/bin/bash
echo "$@" >> "${CLAUDE_ENV_TEST_LOG}/npx_calls.log"

# Simulate skills add: create .claude/skills/<skill>/SKILL.md
if [[ "$1" == "skills" && "$2" == "add" ]]; then
    owner_repo="$3"
    skill=""
    for i in "${!@}"; do
        if [[ "${!i}" == "--skill" ]]; then
            next=$((i+1))
            skill="${!next}"
        fi
    done
    if [[ -n "$skill" ]]; then
        mkdir -p ".claude/skills/$skill"
        echo "# $skill" > ".claude/skills/$skill/SKILL.md"
    fi
fi
exit 0
```

- [ ] **Step 2: Write failing integration test**

Create `claude-env/tests/integration/install_skill_test.rs`:

```rust
use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;

#[test]
fn install_skill() {
    let project = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    project.child("claude-env.toml").write_str(r#"
[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"
"#).unwrap();

    let shims_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/integration/shims");

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("install")
        .current_dir(project.path())
        .env("PATH", format!("{}:{}", shims_dir.display(), std::env::var("PATH").unwrap()))
        .env("CLAUDE_ENV_HOME", packages.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path());

    cmd.assert().success();

    // Verify npx was called correctly
    let npx_log = fs::read_to_string(log_dir.path().join("npx_calls.log")).unwrap();
    assert!(npx_log.contains("skills add vercel-labs/next-skills"));
    assert!(npx_log.contains("--skill next-best-practices"));
    assert!(npx_log.contains("-a claude-code"));
    assert!(npx_log.contains("-y"));
}
```

- [ ] **Step 3: Implement skill installer**

Create `claude-env/src/installer/skill.rs`:

```rust
use crate::error::InstallError;
use crate::installer::{InstallContext, InstallResult, Installer};
use crate::resolver::PlannedAction;
use std::process::Command;

pub struct SkillInstaller;

impl SkillInstaller {
    fn parse_skill_path(name: &str) -> Result<(&str, &str), InstallError> {
        // Format: "owner/repo/skill-name"
        let parts: Vec<&str> = name.splitn(3, '/').collect();
        if parts.len() != 3 {
            return Err(InstallError::Command(
                "parse skill".to_string(),
                format!("invalid skill format '{}', expected 'owner/repo/skill'", name),
            ));
        }
        let owner_repo = &name[..name.rfind('/').unwrap()];
        let skill = parts[2];
        Ok((owner_repo, skill))
    }
}

impl Installer for SkillInstaller {
    fn install(&self, action: &PlannedAction, ctx: &InstallContext) -> Result<InstallResult, InstallError> {
        let (owner_repo, skill) = Self::parse_skill_path(&action.name)?;

        let args = vec![
            "skills", "add", owner_repo,
            "--skill", skill,
            "-a", "claude-code",
            "-y",
        ];

        if ctx.verbose {
            eprintln!("  > npx {}", args.join(" "));
        }

        let output = Command::new("npx")
            .args(&args)
            .current_dir(ctx.project_root)
            .output()
            .map_err(|e| InstallError::Command("npx skills add".to_string(), e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InstallError::Command(
                format!("npx skills add {} --skill {}", owner_repo, skill),
                stderr.to_string(),
            ));
        }

        Ok(InstallResult {
            installed: true,
            integrity: None,
        })
    }
}
```

Add to `claude-env/src/installer/mod.rs`:

```rust
pub mod cli_tool;
pub mod mcp;
pub mod skill;
```

- [ ] **Step 4: Run integration test**

```bash
cd claude-env && cargo test --test integration -- install_skill_test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add claude-env/src/installer/skill.rs claude-env/tests/integration/install_skill_test.rs claude-env/tests/integration/shims/npx
git commit -m "feat(claude-env): implement skill installer via npx skills add"
```

---

## Task 10: Plugin Installer

**Files:**
- Create: `claude-env/src/installer/plugin.rs`
- Create: `claude-env/tests/integration/shims/claude`
- Test: `claude-env/tests/integration/install_plugin_test.rs`

- [ ] **Step 1: Create claude shim**

Create `claude-env/tests/integration/shims/claude` (executable):

```bash
#!/bin/bash
echo "$@" >> "${CLAUDE_ENV_TEST_LOG}/claude_calls.log"

# Simulate plugin install: write to .claude/settings.json
if [[ "$1" == "plugin" && "$2" == "install" ]]; then
    plugin_id="$3"
    mkdir -p .claude
    if [[ -f .claude/settings.json ]]; then
        # Append plugin (simplified)
        content=$(cat .claude/settings.json)
        content="${content%\}}"
        echo "${content}, \"${plugin_id}\": true}" > .claude/settings.json
    else
        echo "{\"${plugin_id}\": true}" > .claude/settings.json
    fi
fi
exit 0
```

- [ ] **Step 2: Write failing integration test**

Create `claude-env/tests/integration/install_plugin_test.rs`:

```rust
use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;

#[test]
fn install_plugin() {
    let project = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    project.child("claude-env.toml").write_str(r#"
[plugins]
"anthropics/claude-code/code-review@claude-code-plugins" = "latest"
"#).unwrap();

    let shims_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/integration/shims");

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("install")
        .current_dir(project.path())
        .env("PATH", format!("{}:{}", shims_dir.display(), std::env::var("PATH").unwrap()))
        .env("CLAUDE_ENV_HOME", packages.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path());

    cmd.assert().success();

    // Verify claude was called with marketplace add then plugin install
    let log = fs::read_to_string(log_dir.path().join("claude_calls.log")).unwrap();
    let lines: Vec<&str> = log.lines().collect();
    assert!(lines[0].contains("plugin marketplace add anthropics/claude-code"));
    assert!(lines[1].contains("plugin install code-review@claude-code-plugins"));
    assert!(lines[1].contains("--scope project"));
}
```

- [ ] **Step 3: Implement plugin installer**

Create `claude-env/src/installer/plugin.rs`:

```rust
use crate::error::InstallError;
use crate::installer::{InstallContext, InstallResult, Installer};
use crate::resolver::PlannedAction;
use std::process::Command;

pub struct PluginInstaller;

impl PluginInstaller {
    /// Parse "owner/repo/plugin@marketplace" format
    fn parse_plugin_path(name: &str) -> Result<PluginParts, InstallError> {
        let at_pos = name.rfind('@').ok_or_else(|| {
            InstallError::Command(
                "parse plugin".to_string(),
                format!("invalid plugin format '{}', expected 'owner/repo/plugin@marketplace'", name),
            )
        })?;

        let path_part = &name[..at_pos];
        let marketplace = &name[at_pos + 1..];

        let parts: Vec<&str> = path_part.splitn(3, '/').collect();
        if parts.len() != 3 {
            return Err(InstallError::Command(
                "parse plugin".to_string(),
                format!("invalid plugin path '{}', expected 'owner/repo/plugin'", path_part),
            ));
        }

        Ok(PluginParts {
            owner_repo: format!("{}/{}", parts[0], parts[1]),
            plugin: parts[2].to_string(),
            marketplace: marketplace.to_string(),
        })
    }
}

struct PluginParts {
    owner_repo: String,
    plugin: String,
    marketplace: String,
}

impl Installer for PluginInstaller {
    fn install(&self, action: &PlannedAction, ctx: &InstallContext) -> Result<InstallResult, InstallError> {
        let parts = Self::parse_plugin_path(&action.name)?;

        // Step 1: Register marketplace
        if ctx.verbose {
            eprintln!("  > claude plugin marketplace add {}", parts.owner_repo);
        }

        let output = Command::new("claude")
            .args(["plugin", "marketplace", "add", &parts.owner_repo])
            .current_dir(ctx.project_root)
            .output()
            .map_err(|e| InstallError::Command("claude plugin marketplace add".to_string(), e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InstallError::Command(
                format!("claude plugin marketplace add {}", parts.owner_repo),
                stderr.to_string(),
            ));
        }

        // Step 2: Install plugin
        let plugin_id = format!("{}@{}", parts.plugin, parts.marketplace);
        if ctx.verbose {
            eprintln!("  > claude plugin install {} --scope project", plugin_id);
        }

        let output = Command::new("claude")
            .args(["plugin", "install", &plugin_id, "--scope", "project"])
            .current_dir(ctx.project_root)
            .output()
            .map_err(|e| InstallError::Command("claude plugin install".to_string(), e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(InstallError::Command(
                format!("claude plugin install {}", plugin_id),
                stderr.to_string(),
            ));
        }

        Ok(InstallResult {
            installed: true,
            integrity: None,
        })
    }
}
```

Add to `claude-env/src/installer/mod.rs`:

```rust
pub mod cli_tool;
pub mod mcp;
pub mod plugin;
pub mod skill;
```

- [ ] **Step 4: Run integration test**

```bash
cd claude-env && cargo test --test integration -- install_plugin_test
```

Expected: PASS.

- [ ] **Step 5: Commit**

```bash
git add claude-env/src/installer/plugin.rs claude-env/tests/integration/install_plugin_test.rs claude-env/tests/integration/shims/claude
git commit -m "feat(claude-env): implement plugin installer via claude CLI"
```

---

## Task 11: Output Formatting

**Files:**
- Create: `claude-env/src/output.rs`

- [ ] **Step 1: Implement output module**

Create `claude-env/src/output.rs`:

```rust
pub struct Reporter {
    pub installed: u32,
    pub skipped: u32,
    pub failed: u32,
}

impl Reporter {
    pub fn new() -> Self {
        Self { installed: 0, skipped: 0, failed: 0 }
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
        println!("  \x1b[90m⊘\x1b[0m {:<25} {} skipped", name, version);
    }

    pub fn summary(&self) {
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
```

- [ ] **Step 2: Wire into main.rs install command**

Replace the inline println calls with `Reporter` calls.

- [ ] **Step 3: Verify output looks correct**

```bash
cd claude-env && cargo run -- install --help
```

- [ ] **Step 4: Commit**

```bash
git add claude-env/src/output.rs claude-env/src/main.rs
git commit -m "feat(claude-env): add colored terminal output formatting"
```

---

## Task 12: Full Install Pipeline Integration Test

**Files:**
- Create: `claude-env/tests/integration/fixtures/full_config.toml`
- Test: `claude-env/tests/integration/full_install_test.rs`

- [ ] **Step 1: Create full fixture**

Create `claude-env/tests/integration/fixtures/full_config.toml`:

```toml
[mcp]
context7 = "2.1.4"

[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"

[plugins]
"anthropics/claude-code/code-review@claude-code-plugins" = "latest"

[cli]
get-shit-done-cc = "1.22.4"
```

- [ ] **Step 2: Write integration test covering all 4 types**

Create `claude-env/tests/integration/full_install_test.rs`:

```rust
use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;

#[test]
fn full_install_all_tool_types() {
    let project = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    let fixture = fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/integration/fixtures/full_config.toml")
    ).unwrap();
    project.child("claude-env.toml").write_str(&fixture).unwrap();

    let shims_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/integration/shims");

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("install")
        .current_dir(project.path())
        .env("PATH", format!("{}:{}", shims_dir.display(), std::env::var("PATH").unwrap()))
        .env("CLAUDE_ENV_HOME", packages.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);

    // All 4 tools installed
    assert!(stdout.contains("4 installed, 0 failed, 0 skipped"));

    // MCP: .mcp.json created
    assert!(project.path().join(".mcp.json").exists());

    // Skills: npx called
    let npx_log = fs::read_to_string(log_dir.path().join("npx_calls.log")).unwrap();
    assert!(npx_log.contains("skills add"));

    // Plugins: claude called
    let claude_log = fs::read_to_string(log_dir.path().join("claude_calls.log")).unwrap();
    assert!(claude_log.contains("plugin marketplace add"));
    assert!(claude_log.contains("plugin install"));

    // CLI: npm + post_install called
    let npm_log = fs::read_to_string(log_dir.path().join("npm_calls.log")).unwrap();
    assert!(npm_log.contains("get-shit-done-cc"));

    // Lockfile created with all entries
    let lockfile = fs::read_to_string(project.path().join("claude-env.lock")).unwrap();
    assert!(lockfile.contains("[mcp.context7]"));
    assert!(lockfile.contains("[cli.get-shit-done-cc]"));
}

#[test]
fn partial_failure_exits_with_code_1() {
    let project = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    // Use a package that the shim won't handle (will fail)
    project.child("claude-env.toml").write_str(r#"
[mcp]
"@nonexistent/broken-pkg" = "0.0.1"
context7 = "2.1.4"
"#).unwrap();

    let shims_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/integration/shims");

    // Create a failing npm shim for this specific package
    // (override shim to fail on @nonexistent)
    let custom_shims = TempDir::new().unwrap();
    custom_shims.child("npm").write_str(r#"#!/bin/bash
echo "$@" >> "${CLAUDE_ENV_TEST_LOG}/npm_calls.log"
if echo "$@" | grep -q "@nonexistent"; then
    echo "ERR! 404 Not Found" >&2
    exit 1
fi
# Normal success path
prefix=""
for arg in "$@"; do
    if [[ "$prev" == "--prefix" ]]; then prefix="$arg"; fi
    prev="$arg"
done
if [[ -n "$prefix" ]]; then
    mkdir -p "$prefix/node_modules/.bin"
    echo "#!/bin/bash" > "$prefix/node_modules/.bin/context7-mcp"
    chmod +x "$prefix/node_modules/.bin/context7-mcp"
fi
exit 0
"#).unwrap();
    std::fs::set_permissions(
        custom_shims.path().join("npm"),
        std::os::unix::fs::PermissionsExt::from_mode(0o755),
    ).unwrap();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("install")
        .current_dir(project.path())
        .env("PATH", format!("{}:{}:{}", custom_shims.path().display(), shims_dir.display(), std::env::var("PATH").unwrap()))
        .env("CLAUDE_ENV_HOME", packages.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path());

    cmd.assert().code(1);
}
```

- [ ] **Step 3: Run test**

```bash
cd claude-env && cargo test --test integration -- full_install
```

Expected: both PASS.

- [ ] **Step 4: Commit**

```bash
git add claude-env/tests/integration/
git commit -m "test(claude-env): add full pipeline integration tests with partial failure"
```

---

## Task 13: E2E Docker Setup

**Files:**
- Create: `claude-env/tests/e2e/Dockerfile`
- Create: `claude-env/tests/e2e/docker-compose.yml`
- Create: `claude-env/tests/e2e/scenarios/mcp_install.sh`

- [ ] **Step 1: Create Dockerfile**

Create `claude-env/tests/e2e/Dockerfile`:

```dockerfile
FROM rust:1.79-slim AS builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM node:20-slim
RUN npm install -g @anthropic-ai/claude-code
COPY --from=builder /app/target/release/claude-env /usr/local/bin/claude-env
WORKDIR /workspace
COPY tests/e2e/scenarios/ /scenarios/
ENTRYPOINT ["/bin/bash"]
```

- [ ] **Step 2: Create docker-compose.yml**

Create `claude-env/tests/e2e/docker-compose.yml`:

```yaml
services:
  e2e:
    build:
      context: ../..
      dockerfile: tests/e2e/Dockerfile
    command: ["/scenarios/run_all.sh"]
    environment:
      - CLAUDE_ENV_HOME=/tmp/claude-env-packages
```

- [ ] **Step 3: Create a basic E2E scenario**

Create `claude-env/tests/e2e/scenarios/mcp_install.sh`:

```bash
#!/bin/bash
set -euo pipefail

echo "=== E2E: MCP Install ==="

cd /tmp
mkdir -p mcp-test && cd mcp-test

cat > claude-env.toml <<'EOF'
[mcp]
context7 = "latest"
EOF

claude-env install

# Verify
if [[ ! -f .mcp.json ]]; then
    echo "FAIL: .mcp.json not created"
    exit 1
fi

if ! grep -q "context7" .mcp.json; then
    echo "FAIL: context7 not in .mcp.json"
    exit 1
fi

echo "PASS: MCP install"
```

Create `claude-env/tests/e2e/scenarios/run_all.sh`:

```bash
#!/bin/bash
set -euo pipefail

PASS=0
FAIL=0

for scenario in /scenarios/*.sh; do
    [[ "$(basename "$scenario")" == "run_all.sh" ]] && continue
    echo ""
    if bash "$scenario"; then
        PASS=$((PASS + 1))
    else
        FAIL=$((FAIL + 1))
    fi
done

echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
[[ $FAIL -eq 0 ]]
```

- [ ] **Step 4: Verify Docker build works**

```bash
cd claude-env && docker compose -f tests/e2e/docker-compose.yml build
```

- [ ] **Step 5: Commit**

```bash
git add claude-env/tests/e2e/
git commit -m "test(claude-env): add E2E Docker test infrastructure"
```

---

## Task 14: `claude-env list` Command

**Files:**
- Modify: `claude-env/src/main.rs`
- Test: `claude-env/tests/integration/list_test.rs`

- [ ] **Step 1: Write failing test**

Create `claude-env/tests/integration/list_test.rs`:

```rust
use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;

#[test]
fn list_shows_installed_tools() {
    let project = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();

    project.child("claude-env.toml").write_str(r#"
[mcp]
context7 = "2.1.4"
chrome-devtools = "1.0.3"
"#).unwrap();

    project.child("claude-env.lock").write_str(r#"
[mcp.context7]
package = "@upstash/context7-mcp"
version = "2.1.4"
integrity = "sha512-abc"

[mcp.chrome-devtools]
package = "chrome-devtools-mcp"
version = "1.0.3"
integrity = "sha512-def"
"#).unwrap();

    // Simulate installed packages
    std::fs::create_dir_all(packages.path().join("context7/node_modules")).unwrap();
    std::fs::create_dir_all(packages.path().join("chrome-devtools/node_modules")).unwrap();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("list")
        .current_dir(project.path())
        .env("CLAUDE_ENV_HOME", packages.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8_lossy(&output.get_output().stdout);
    assert!(stdout.contains("context7"));
    assert!(stdout.contains("2.1.4"));
    assert!(stdout.contains("chrome-devtools"));
    assert!(stdout.contains("1.0.3"));
}
```

- [ ] **Step 2: Implement list command**

In `main.rs`, implement `Command::List`:

```rust
Command::List => {
    let project_root = std::env::current_dir().unwrap();
    let config_path = project_root.join("claude-env.toml");
    let lock_path = project_root.join("claude-env.lock");

    let config = config::Config::from_file(&config_path).unwrap_or_default();
    let lockfile = lockfile::Lockfile::from_file(&lock_path).unwrap_or_default();

    let packages_dir = std::env::var("CLAUDE_ENV_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".claude-env/packages"));

    println!("  {:<25} {:<12} {}", "TOOL", "VERSION", "STATUS");
    println!("  {}", "─".repeat(50));

    let sections = [
        ("mcp", &config.mcp),
        ("cli", &config.cli),
        ("skills", &config.skills),
        ("plugins", &config.plugins),
    ];

    for (section, tools) in sections {
        for (name, requested) in tools {
            let locked_ver = lockfile.get(section, name)
                .map(|l| l.version.as_str())
                .unwrap_or("?");
            let installed = packages_dir.join(name).join("node_modules").exists();
            let status = if installed { "✓ installed" } else { "✗ missing" };
            println!("  {:<25} {:<12} {}", name, locked_ver, status);
        }
    }
}
```

- [ ] **Step 3: Run test**

```bash
cd claude-env && cargo test --test integration -- list_test
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add claude-env/src/main.rs claude-env/tests/integration/list_test.rs
git commit -m "feat(claude-env): implement list command showing installed tools and status"
```

---

## Task 15: `claude-env remove` Command

**Files:**
- Modify: `claude-env/src/main.rs`
- Test: `claude-env/tests/integration/remove_test.rs`

- [ ] **Step 1: Write failing test**

Create `claude-env/tests/integration/remove_test.rs`:

```rust
use assert_cmd::Command;
use assert_fs::prelude::*;
use assert_fs::TempDir;
use std::fs;

#[test]
fn remove_mcp_tool() {
    let project = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();

    project.child("claude-env.toml").write_str(r#"
[mcp]
context7 = "2.1.4"
chrome-devtools = "1.0.3"
"#).unwrap();

    project.child(".mcp.json").write_str(r#"
{"mcpServers":{"context7-mcp":{"type":"stdio","command":"/path"},"chrome-devtools-mcp":{"type":"stdio","command":"/path2"}}}
"#).unwrap();

    // Create package dir
    std::fs::create_dir_all(packages.path().join("context7/node_modules")).unwrap();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.args(["remove", "context7"])
        .current_dir(project.path())
        .env("CLAUDE_ENV_HOME", packages.path());

    cmd.assert().success();

    // Config no longer contains context7
    let config = fs::read_to_string(project.path().join("claude-env.toml")).unwrap();
    assert!(!config.contains("context7"));
    assert!(config.contains("chrome-devtools"));

    // .mcp.json no longer contains context7
    let mcp = fs::read_to_string(project.path().join(".mcp.json")).unwrap();
    assert!(!mcp.contains("context7"));

    // Package directory removed
    assert!(!packages.path().join("context7").exists());
}
```

- [ ] **Step 2: Implement remove command**

```rust
Command::Remove { tool } => {
    let project_root = std::env::current_dir().unwrap();
    let config_path = project_root.join("claude-env.toml");
    let packages_dir = std::env::var("CLAUDE_ENV_HOME")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|_| dirs::home_dir().unwrap().join(".claude-env/packages"));

    // Remove from config file
    let content = std::fs::read_to_string(&config_path).unwrap();
    let mut new_lines: Vec<&str> = Vec::new();
    for line in content.lines() {
        if !line.contains(&tool) || line.starts_with('[') {
            new_lines.push(line);
        }
    }
    std::fs::write(&config_path, new_lines.join("\n")).unwrap();

    // Remove from .mcp.json
    let registry = registry::Registry::default();
    let package = registry.resolve_alias(&tool);
    let bin_name = package.strip_prefix('@')
        .and_then(|s| s.split('/').last())
        .unwrap_or(package);
    let _ = mcp_config::remove_server(&project_root, bin_name);

    // Remove package directory
    let pkg_dir = packages_dir.join(&tool);
    if pkg_dir.exists() {
        std::fs::remove_dir_all(&pkg_dir).unwrap();
    }

    println!("  Removed {}", tool);
}
```

- [ ] **Step 3: Run test**

```bash
cd claude-env && cargo test --test integration -- remove_test
```

Expected: PASS.

- [ ] **Step 4: Commit**

```bash
git add claude-env/src/main.rs claude-env/tests/integration/remove_test.rs
git commit -m "feat(claude-env): implement remove command (config + .mcp.json + packages cleanup)"
```

---

## Task 16: README and Final Polish

**Files:**
- Create: `claude-env/README.md`
- Modify: `claude-env/Cargo.toml` (add metadata)

- [ ] **Step 1: Write README**

Create `claude-env/README.md`:

```markdown
# claude-env

Declarative Claude Code environment manager. Declare your MCP servers, skills, plugins, and CLI tools in one file — `claude-env install` handles the rest.

## Install

```bash
cargo install claude-env
```

## Quick Start

Create `claude-env.toml` in your project root:

```toml
[mcp]
context7 = "2.1.4"

[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"

[plugins]
"anthropics/claude-code/code-review@claude-code-plugins" = "latest"

[cli]
get-shit-done-cc = "1.22.4"
```

Then run:

```bash
claude-env install
```

## Commands

| Command | Description |
|---------|-------------|
| `claude-env install` | Install from lockfile (or resolve + create lockfile) |
| `claude-env update` | Check for updates, show changelogs |
| `claude-env update <tool>` | Update a single tool |
| `claude-env diff <tool>` | Show changelog between versions |
| `claude-env list` | Show installed tools and status |
| `claude-env add <tool>` | Add a tool to config |
| `claude-env remove <tool>` | Remove tool and clean up |

## How It Works

1. Reads `claude-env.toml` for declared tools
2. Compares against `claude-env.lock` to determine what needs installing
3. Installs each tool sequentially (no concurrency issues)
4. Writes config files (`.mcp.json`, `.claude/settings.json`)
5. Updates `claude-env.lock` with resolved versions

Packages are cached globally at `~/.claude-env/packages/`.
```

- [ ] **Step 2: Update Cargo.toml metadata**

Add repository, keywords, categories:

```toml
[package]
repository = "https://github.com/komune-io/claude-env"
keywords = ["claude", "mcp", "ai", "tooling"]
categories = ["command-line-utilities", "development-tools"]
```

- [ ] **Step 3: Commit**

```bash
git add claude-env/README.md claude-env/Cargo.toml
git commit -m "docs(claude-env): add README and Cargo.toml metadata"
```

---

## Summary

| Task | What it delivers |
|------|-----------------|
| 1 | Project scaffold + config parsing + CLI skeleton |
| 2 | Registry (alias resolution + tool overrides) |
| 3 | Lockfile parse/serialize |
| 4 | npm registry client (versions, integrity) |
| 5 | Resolver (install/upgrade/skip decisions) |
| 6 | .mcp.json reader/writer |
| 7 | MCP installer + first integration test |
| 8 | CLI tool installer with post_install |
| 9 | Skill installer |
| 10 | Plugin installer |
| 11 | Output formatting (colors, symbols) |
| 12 | Full pipeline integration test |
| 13 | E2E Docker setup |
| 14 | `list` command |
| 15 | `remove` command |
| 16 | README + final polish |

After Task 12, you have a working `claude-env install`. Tasks 13-16 are polish and UX commands. The `update` and `diff` commands (which require changelog fetching from npm/GitHub APIs) are natural follow-up work once the core install pipeline is solid.
