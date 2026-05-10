# PR #2 Mandatory Fix-Pack Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Apply the eight mandatory fixes from `docs/superpowers/specs/2026-05-10-pr2-mandatory-fixes-design.md` so PR #2 (`feat/rename-to-chord`) can ship without behavioral bugs, stale rename artifacts, or weak release wiring.

**Architecture:** Eight small, independent fixes applied in order. Each task is self-contained: failing test → minimal fix → green test → commit. Workflow / Dockerfile changes have no unit-test coverage and rely on CI runs + visual diff verification.

**Tech Stack:** Rust (resolver, installer, env var), Lua (mise plugin hooks), GitHub Actions YAML, Dockerfile.

---

## Files Touched (summary)

- Modify: `src/resolver.rs`
- Modify: `tests/unit/resolver_test.rs`
- Modify: `src/main.rs`
- Modify: `tests/integration/install_test.rs`, `tests/integration/remove_test.rs`, `tests/integration/install_skill_test.rs`, `tests/integration/install_cli_test.rs`, `tests/integration/install_plugin_test.rs`, `tests/integration/list_test.rs`, `tests/integration/full_install_test.rs`
- Modify: `Cargo.toml`
- Modify: `metadata.lua`
- Modify: `README.md`
- Modify: `hooks/backend_install.lua`, `hooks/backend_list_versions.lua`
- Modify: `src/installer/mcp.rs`
- Create: `tests/unit/mcp_installer_test.rs` (or merge into existing file — see Task 5)
- Modify: `.github/workflows/publish.yml`
- Modify: `.github/workflows/release.yml`
- Modify: `e2e/Dockerfile`

---

## Task 1: Resolver `latest` thrash fix

**Files:**
- Modify: `src/resolver.rs:43-83`
- Modify: `tests/unit/resolver_test.rs` (append)

### Background

Currently `resolver.rs:65-70` compares `entry.version` to `requested_version` byte-for-byte. When `chord.toml` declares `tool = "latest"` and the lockfile holds the resolved concrete version (e.g. `"2.1.4"`), `"2.1.4" != "latest"` is always true → `Action::Upgrade` on every resolve → npm reinstall on every shell entry via `backend_exec_env.lua`.

The fix is to recognize `"latest"` and `"*"` as wildcards: if the lockfile holds any concrete version and the tool is installed, return `Skip`.

- [ ] **Step 1.1: Write the failing test**

Append the following tests to `tests/unit/resolver_test.rs`:

```rust
#[test]
fn latest_with_concrete_locked_version_and_installed_skips() {
    let config = Config::parse(
        r#"
        [mcp]
        context7 = "latest"
        "#,
    )
    .unwrap();

    let mut lockfile = Lockfile::new();
    lockfile.set(
        "mcp",
        "context7",
        LockedTool {
            package: None,
            version: "2.1.4".to_string(),
            integrity: None,
            resolved_at: None,
        },
    );

    let plan = resolve(&config, &lockfile, &|_section, _name| true);

    assert_eq!(plan.actions.len(), 1);
    assert_eq!(
        plan.actions[0].action,
        Action::Skip,
        "latest + concrete lockfile + installed should Skip, not Upgrade"
    );
}

#[test]
fn latest_with_concrete_locked_version_but_not_installed_installs() {
    let config = Config::parse(
        r#"
        [mcp]
        context7 = "latest"
        "#,
    )
    .unwrap();

    let mut lockfile = Lockfile::new();
    lockfile.set(
        "mcp",
        "context7",
        LockedTool {
            package: None,
            version: "2.1.4".to_string(),
            integrity: None,
            resolved_at: None,
        },
    );

    let plan = resolve(&config, &lockfile, &|_section, _name| false);

    assert_eq!(plan.actions[0].action, Action::Install);
}

#[test]
fn wildcard_star_behaves_like_latest() {
    let config = Config::parse(
        r#"
        [mcp]
        context7 = "*"
        "#,
    )
    .unwrap();

    let mut lockfile = Lockfile::new();
    lockfile.set(
        "mcp",
        "context7",
        LockedTool {
            package: None,
            version: "2.1.4".to_string(),
            integrity: None,
            resolved_at: None,
        },
    );

    let plan = resolve(&config, &lockfile, &|_section, _name| true);

    assert_eq!(plan.actions[0].action, Action::Skip);
}

#[test]
fn concrete_version_mismatch_still_upgrades() {
    // Regression guard: the new wildcard branch must not affect concrete pins.
    let config = Config::parse(
        r#"
        [mcp]
        context7 = "3.0.0"
        "#,
    )
    .unwrap();

    let mut lockfile = Lockfile::new();
    lockfile.set(
        "mcp",
        "context7",
        LockedTool {
            package: None,
            version: "2.1.4".to_string(),
            integrity: None,
            resolved_at: None,
        },
    );

    let plan = resolve(&config, &lockfile, &|_section, _name| true);

    assert_eq!(plan.actions[0].action, Action::Upgrade);
}
```

- [ ] **Step 1.2: Run tests to verify they fail**

Run: `cargo test --test unit -- resolver_test::latest_with_concrete_locked_version_and_installed_skips resolver_test::latest_with_concrete_locked_version_but_not_installed_installs resolver_test::wildcard_star_behaves_like_latest resolver_test::concrete_version_mismatch_still_upgrades`

Expected: 3 fail (`Upgrade` returned where `Skip` or `Install` expected), 1 pass (`concrete_version_mismatch_still_upgrades` — regression guard already passes).

- [ ] **Step 1.3: Apply the resolver fix**

In `src/resolver.rs`, replace the `match locked { ... }` block (lines 65-70) with the wildcard-aware ladder. The full edited function should read:

```rust
pub fn resolve(
    config: &Config,
    lockfile: &Lockfile,
    is_installed: &dyn Fn(&str, &str) -> bool,
) -> Plan {
    let registry = Registry::default();
    let mut actions: Vec<PlannedAction> = Vec::new();

    let sections: &[(&str, fn() -> ToolType, &std::collections::BTreeMap<String, String>)] = &[
        ("mcp", || ToolType::Mcp, &config.mcp),
        ("cli", || ToolType::Cli, &config.cli),
        ("skills", || ToolType::Skill, &config.skills),
        ("plugins", || ToolType::Plugin, &config.plugins),
    ];

    for (section, make_type, map) in sections {
        for (name, requested_version) in *map {
            let package = registry.resolve_alias(name).to_string();
            let locked = lockfile.get(section, name);
            let installed = is_installed(section, name);
            let is_wildcard = requested_version == "latest" || requested_version == "*";

            let action = match locked {
                None => Action::Install,
                Some(_) if is_wildcard => {
                    if installed { Action::Skip } else { Action::Install }
                }
                Some(entry) if entry.version != *requested_version => Action::Upgrade,
                Some(_) if !installed => Action::Install,
                Some(_) => Action::Skip,
            };

            actions.push(PlannedAction {
                name: name.clone(),
                package,
                version: requested_version.clone(),
                tool_type: make_type(),
                action,
            });
        }
    }

    Plan { actions }
}
```

- [ ] **Step 1.4: Run all resolver tests to verify green**

Run: `cargo test --test unit -- resolver_test`

Expected: All resolver tests pass (existing 6 + new 4 = 10 tests).

- [ ] **Step 1.5: Commit**

```bash
git add src/resolver.rs tests/unit/resolver_test.rs
git commit -m "fix(resolver): treat 'latest'/'*' as wildcards to stop reinstall thrash

Lockfile stores the resolved concrete version while chord.toml may say
'latest'. The previous version-string comparison always returned Upgrade
in that case, triggering an npm reinstall on every shell entry via
backend_exec_env.lua. Now wildcard requests Skip when a concrete version
is already locked and installed."
```

---

## Task 2: `CLAUDE_ENV_HOME` → `CHORD_HOME`

**Files:**
- Modify: `src/main.rs:320`
- Modify: `tests/integration/install_test.rs` (3 sites)
- Modify: `tests/integration/remove_test.rs` (2 sites)
- Modify: `tests/integration/install_skill_test.rs` (1 site)
- Modify: `tests/integration/install_cli_test.rs` (1 site)
- Modify: `tests/integration/install_plugin_test.rs` (1 site)
- Modify: `tests/integration/list_test.rs` (3 sites)
- Modify: `tests/integration/full_install_test.rs` (2 sites)

### Background

The crate, binary, plugin, and config files all renamed to `chord`, but `packages_dir()` in `src/main.rs:319-328` still reads the legacy `CLAUDE_ENV_HOME` env var. Integration tests pass this same legacy name. Project is pre-1.0; cut over without a fallback. Historical plan / spec markdown files keep the old name — leave those untouched.

- [ ] **Step 2.1: Edit `src/main.rs:320`**

Change:

```rust
fn packages_dir() -> PathBuf {
    std::env::var("CLAUDE_ENV_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".chord")
                .join("packages")
        })
}
```

to:

```rust
fn packages_dir() -> PathBuf {
    std::env::var("CHORD_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".chord")
                .join("packages")
        })
}
```

- [ ] **Step 2.2: Update integration tests in bulk**

Run from repo root:

```bash
sed -i '' 's/CLAUDE_ENV_HOME/CHORD_HOME/g' \
  tests/integration/install_test.rs \
  tests/integration/remove_test.rs \
  tests/integration/install_skill_test.rs \
  tests/integration/install_cli_test.rs \
  tests/integration/install_plugin_test.rs \
  tests/integration/list_test.rs \
  tests/integration/full_install_test.rs
```

(macOS BSD sed syntax. On Linux, use `sed -i '...' ...` without the empty backup-extension arg.)

- [ ] **Step 2.3: Verify no stragglers in active code**

Run: `git grep -nE 'CLAUDE_ENV_HOME' -- 'src/' 'tests/' 'hooks/' 'e2e/' 'sample/' '*.lua' '*.toml' README.md CLAUDE.md`

Expected: zero matches. (Hits in `docs/superpowers/` are historical and stay.)

- [ ] **Step 2.4: Run full test suite**

Run: `mise run test`

Expected: green.

- [ ] **Step 2.5: Commit**

```bash
git add src/main.rs tests/integration/
git commit -m "fix: rename CLAUDE_ENV_HOME env var to CHORD_HOME

Completes the rename pass for the packages-dir override. Project is
pre-1.0; no compatibility shim for the old name."
```

---

## Task 3: Fix broken `rytmyk/chord` URLs

**Files:**
- Modify: `Cargo.toml:7`
- Modify: `metadata.lua:6`
- Modify: `README.md:12`

### Background

Three files point at `https://github.com/rytmyk/chord`, which does not exist. The actual upstream is `https://github.com/komune-io/mise-claude`. The spec only flagged `Cargo.toml`; `metadata.lua` (mise plugin homepage) and `README.md` (install command example) carry the same dead URL. All three are user-visible: `Cargo.toml` ships to crates.io; `metadata.lua` shows in `mise plugins ls --metadata`; `README.md` is the first thing a new user reads.

- [ ] **Step 3.1: Edit `Cargo.toml:7`**

Change:

```toml
repository = "https://github.com/rytmyk/chord"
```

to:

```toml
repository = "https://github.com/komune-io/mise-claude"
```

- [ ] **Step 3.2: Edit `metadata.lua:6`**

Change:

```lua
  homepage = "https://github.com/rytmyk/chord",
```

to:

```lua
  homepage = "https://github.com/komune-io/mise-claude",
```

- [ ] **Step 3.3: Edit `README.md:12`**

Change:

```
mise plugin install chord https://github.com/rytmyk/chord
```

to:

```
mise plugin install chord https://github.com/komune-io/mise-claude
```

- [ ] **Step 3.4: Verify no stragglers**

Run: `git grep -nE 'rytmyk/chord' -- 'src/' 'tests/' 'hooks/' 'e2e/' 'sample/' '*.lua' '*.toml' '*.yml' README.md CLAUDE.md`

Expected: zero matches. (Hits in `docs/superpowers/` are historical and stay.)

- [ ] **Step 3.5: Verify Cargo manifest parses**

Run: `cargo metadata --no-deps --format-version 1 --quiet > /dev/null`

Expected: exit 0, no output. Confirms `Cargo.toml` is still valid TOML.

- [ ] **Step 3.6: Commit**

```bash
git add Cargo.toml metadata.lua README.md
git commit -m "fix: replace dead rytmyk/chord URLs with komune-io/mise-claude

The rytmyk/chord GitHub path does not exist. Updates Cargo.toml
repository metadata, mise plugin homepage, and the README install
command so all three point at the current upstream."
```

---

## Task 4: Stabilize User-Agent strings in mise hooks

**Files:**
- Modify: `hooks/backend_install.lua:35`
- Modify: `hooks/backend_list_versions.lua:31`

### Background

Both hooks ship `User-Agent: "rytmyk-chord/2.0"`, but `Cargo.toml` is at `0.1.0`. The embedded version is wrong and will keep drifting. Drop the version segment entirely — the crates.io API does not require it.

- [ ] **Step 4.1: Edit `hooks/backend_install.lua:35`**

Change:

```lua
    headers = { ["User-Agent"] = "rytmyk-chord/2.0" },
```

to:

```lua
    headers = { ["User-Agent"] = "rytmyk-chord (mise-plugin)" },
```

- [ ] **Step 4.2: Edit `hooks/backend_list_versions.lua:31`**

Change:

```lua
    headers = { ["User-Agent"] = "rytmyk-chord/2.0" },
```

to:

```lua
    headers = { ["User-Agent"] = "rytmyk-chord (mise-plugin)" },
```

- [ ] **Step 4.3: Verify no stragglers**

Run: `git grep -nE 'rytmyk-chord/2\.0' -- hooks/`

Expected: zero matches.

- [ ] **Step 4.4: Lua syntax sanity check**

Run: `lua -e 'loadfile("hooks/backend_install.lua")()' 2>&1 | head -5 || true`

Expected: error about missing `PLUGIN`/`cmd`/`http` globals (Lua plugin sandbox provides those) — but **no** syntax error. Tolerated runtime errors: `attempt to index a nil value`. A literal syntax error means the edit broke the file.

(If `lua` is not on PATH, skip — `mise run lint` covers Lua via StyLua in step 4.5.)

- [ ] **Step 4.5: Format check**

Run: `mise run fmt`

Expected: no diff after the run.

- [ ] **Step 4.6: Commit**

```bash
git add hooks/backend_install.lua hooks/backend_list_versions.lua
git commit -m "fix(hooks): drop version segment from rytmyk-chord User-Agent

UA said 'rytmyk-chord/2.0' but the crate is at 0.1.0. crates.io does
not require a version; use a static identifier so the string never
rots."
```

---

## Task 5: Deterministic `detect_binary`

**Files:**
- Modify: `src/installer/mcp.rs:46-66`
- Modify: `tests/unit.rs` (register new test module)
- Create: `tests/unit/mcp_installer_test.rs`

### Background

`detect_binary` picks the first non-dot entry from `read_dir(bin_dir)`. `read_dir` order is filesystem-dependent. If an npm package ships more than one binary, the chosen one differs across hosts and runs. Sort lexically and pick the first.

- [ ] **Step 5.1: Refactor `detect_binary` to take a directory listing as a slice**

To make the function unit-testable without filesystem fixtures, extract the selection logic into a pure helper. In `src/installer/mcp.rs`, replace the existing `detect_binary` (lines 46-66) with:

```rust
fn detect_binary(bin_dir: &Path, package: &str, registry: &Registry) -> Result<String, InstallError> {
    if let Some(ov) = registry.get_override(package) {
        if let Some(ref name) = ov.bin_name {
            return Ok(name.clone());
        }
    }

    let entries = std::fs::read_dir(bin_dir).map_err(|e| {
        InstallError::Command(
            "detect_binary".to_string(),
            format!("cannot read {}: {}", bin_dir.display(), e),
        )
    })?;

    let mut names: Vec<String> = entries
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| !n.starts_with('.'))
        .collect();

    pick_first_binary(&mut names).ok_or_else(|| {
        InstallError::Command(
            "detect_binary".to_string(),
            format!("no binary found in {}", bin_dir.display()),
        )
    })
}

/// Sort the candidate binary names lexically and return the first.
///
/// Public (not `pub(crate)`) so the external integration-test crate can
/// import it; the deterministic-ordering behavior is unit-tested without
/// a filesystem fixture.
pub fn pick_first_binary(names: &mut Vec<String>) -> Option<String> {
    names.sort();
    names.first().cloned()
}
```

- [ ] **Step 5.2: Add a unit-test module file**

Create `tests/unit/mcp_installer_test.rs`:

```rust
use chord::installer::mcp::pick_first_binary;

#[test]
fn pick_first_binary_returns_lexically_first_name() {
    let mut names = vec!["b-bin".to_string(), "a-bin".to_string(), "c-bin".to_string()];
    assert_eq!(pick_first_binary(&mut names), Some("a-bin".to_string()));
}

#[test]
fn pick_first_binary_is_stable_regardless_of_input_order() {
    let mut a = vec!["x".to_string(), "y".to_string(), "z".to_string()];
    let mut b = vec!["z".to_string(), "y".to_string(), "x".to_string()];
    assert_eq!(pick_first_binary(&mut a), pick_first_binary(&mut b));
}

#[test]
fn pick_first_binary_returns_none_for_empty_input() {
    let mut empty: Vec<String> = vec![];
    assert_eq!(pick_first_binary(&mut empty), None);
}
```

- [ ] **Step 5.3: Register the test module**

Append to `tests/unit.rs`:

```rust
#[path = "unit/mcp_installer_test.rs"]
mod mcp_installer_test;
```

- [ ] **Step 5.4: Run the new tests**

Run: `cargo test --test unit -- mcp_installer_test`

Expected: 3 tests pass.

- [ ] **Step 5.5: Run the full test suite**

Run: `cargo test --locked`

Expected: green.

- [ ] **Step 5.6: Commit**

```bash
git add src/installer/mcp.rs tests/unit.rs tests/unit/mcp_installer_test.rs
git commit -m "fix(installer/mcp): pick lexically-first binary for determinism

read_dir order is filesystem-dependent. Sorting before selection makes
binary detection identical across hosts and runs when a package ships
more than one entry in node_modules/.bin."
```

---

## Task 6: `publish.yml` hardening

**Files:**
- Modify: `.github/workflows/publish.yml`

### Background

Current workflow runs `cargo test` (no `--locked`), then `cargo publish`. A drifted `Cargo.lock` or a manifest error surfaces only during the real publish. Add `--locked` to the test run, gate on `clippy`, and require a successful `cargo publish --dry-run` before the live publish.

- [ ] **Step 6.1: Replace the workflow body**

Replace the entire contents of `.github/workflows/publish.yml` with:

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
        with:
          components: clippy

      - name: Run tests (locked)
        run: cargo test --locked

      - name: Lint
        run: cargo clippy --all-targets --locked -- -D warnings

      - name: Publish dry-run
        run: cargo publish --dry-run --locked

      - name: Publish rytmyk-chord to crates.io
        env:
          CARGO_REGISTRY_TOKEN: ${{ secrets.CARGO_REGISTRY_TOKEN }}
        run: cargo publish
```

- [ ] **Step 6.2: Validate YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/publish.yml'))"`

Expected: exit 0, no output. Confirms the file is valid YAML.

- [ ] **Step 6.3: Commit**

```bash
git add .github/workflows/publish.yml
git commit -m "ci(publish): gate publish on --locked tests, clippy, and dry-run

Catches Cargo.lock drift, lint regressions, and manifest errors before
the real cargo publish runs."
```

---

## Task 7: `release.yml` tag ordering

**Files:**
- Modify: `.github/workflows/release.yml`

### Background

Current workflow does `git tag` → `git push origin v$VERSION` → `gh release create`. If the release step fails (token, network, duplicate), the tag is already pushed and orphans on the remote. `gh release create --target $SHA` creates the tag at the target SHA as part of the release call. If that call fails, no tag is pushed.

- [ ] **Step 7.1: Rewrite the `release` job**

Replace the contents of `.github/workflows/release.yml` with:

```yaml
name: Release

on:
  workflow_dispatch:
    inputs:
      version:
        description: 'Version to release (e.g. 0.2.0)'
        required: true
        type: string

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v5
      - name: Run integration tests
        run: docker compose -f e2e/compose.yml run --rm e2e

  release:
    needs: test
    runs-on: ubuntu-latest
    permissions:
      contents: write
    steps:
      - uses: actions/checkout@v5

      # gh release create --target <sha> creates the tag and the release
      # atomically. If creation fails, no tag is pushed, so a retry is safe.
      - name: Create tag and release
        env:
          GH_TOKEN: ${{ github.token }}
        run: |
          gh release create "v${{ inputs.version }}" \
            --target "$GITHUB_SHA" \
            --generate-notes
```

- [ ] **Step 7.2: Validate YAML syntax**

Run: `python3 -c "import yaml; yaml.safe_load(open('.github/workflows/release.yml'))"`

Expected: exit 0.

- [ ] **Step 7.3: Commit**

```bash
git add .github/workflows/release.yml
git commit -m "ci(release): create tag via gh release to avoid orphan tags

Previously git tag + git push ran before gh release create; a failure in
the release step left the tag pushed with no release attached. gh
release create --target \$SHA does both atomically."
```

---

## Task 8: Pin `claude-code` in e2e Dockerfile

**Files:**
- Modify: `e2e/Dockerfile:6`

### Background

`npm install -g @anthropic-ai/claude-code` is unpinned. Upstream breakage silently turns CI red. The current latest published version (resolved at plan-write time via `npm view @anthropic-ai/claude-code version`) is **2.1.138**. Renovate (`renovate.json` covers npm) will bump it as new versions ship.

- [ ] **Step 8.1: Edit `e2e/Dockerfile:6`**

Change:

```dockerfile
RUN npm install -g @anthropic-ai/claude-code
```

to:

```dockerfile
RUN npm install -g @anthropic-ai/claude-code@2.1.138
```

- [ ] **Step 8.2: Commit**

```bash
git add e2e/Dockerfile
git commit -m "ci(e2e): pin @anthropic-ai/claude-code to 2.1.138

Unpinned installs hide upstream breakage in CI. Renovate (npm scope)
will bump the pin as new versions ship."
```

---

## Final verification

After all eight commits land on `feat/rename-to-chord`:

- [ ] **V.1: Test suite green**

Run: `cargo test --locked`

Expected: all tests pass.

- [ ] **V.2: Lint green**

Run: `cargo clippy --all-targets --locked -- -D warnings`

Expected: exit 0, no warnings.

- [ ] **V.3: Format clean**

Run: `cargo fmt --check`

Expected: exit 0, no diff.

- [ ] **V.4: No stale rename or UA artifacts**

Run:

```bash
git grep -nE 'CLAUDE_ENV_HOME|rytmyk-chord/2\.0|github\.com/rytmyk/chord' \
  -- 'src/' 'tests/' 'hooks/' 'e2e/' 'sample/' '*.lua' '*.toml' '*.yml' README.md CLAUDE.md
```

Expected: zero matches. (`docs/superpowers/` hits stay — they're historical.)

- [ ] **V.5: Manual resolver smoke**

Build chord, then run install twice in a sandbox with `context7 = "latest"`. The second run must `skip`, proving the thrash fix.

```bash
REPO=/Users/adrien/Dev/komune/experimentation/wasm/mise-claude
cargo build --release --manifest-path "$REPO/Cargo.toml"

TMP=$(mktemp -d)
export CHORD_HOME=$(mktemp -d)
cd "$TMP"
cat > chord.toml <<'EOF'
[mcp]
context7 = "latest"
EOF

"$REPO/target/release/chord" install   # first run: should install
"$REPO/target/release/chord" install   # second run: should skip
```

Expected: second `chord install` reports `skip` for `context7`, not `installed` or `upgraded`. (Requires `npm` on PATH and crates.io / npm reachable. If running offline, skip this step and rely on the unit tests added in Task 1.)

- [ ] **V.6: Push branch**

```bash
git push origin feat/rename-to-chord
```

Then watch the **Integration Tests** workflow on GitHub for green. The release / publish workflows do not fire on a branch push.

---

## Risk recap (from spec)

- **Resolver semantics:** `"latest"` + concrete locked + installed now returns `Skip`. A user who wants to force re-resolution today has no flag — they must delete `chord.lock`. A future `chord install --refresh` is out of scope.
- **Release atomicity:** `gh release create --target $SHA` creates tag + release atomically. If a later workflow step fails (none currently exist after this step), the release is left in place; manual cleanup required.
- **Pin staleness:** if Renovate stalls, CI keeps running against `2.1.138` which is the safe failure mode.
