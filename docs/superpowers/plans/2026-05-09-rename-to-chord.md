# Rename to `chord` Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Rename the project end-to-end: `claude-env` Rust subcrate → `chord/` directory + crate `rytmyk-chord` + binary `chord`; mise plugin `claude` → `chord`; config `claude-env.toml` → `chord.toml`; lockfile `claude-env.lock` → `chord.lock`; cache `~/.claude-env/` → `~/.chord/`. Repo will eventually move to `rytmyk/chord` (GitHub UI step, manual).

**Architecture:** Six commits, one per layer (Rust crate → Lua plugin → samples → CI/Docker → docs → repo URL). Each commit is independently buildable. The Cargo trick — `[lib] name = "chord"` and `[[bin]] name = "chord"` while `[package] name = "rytmyk-chord"` — gives a clean user-facing identifier despite the org-prefixed package name.

**Tech Stack:** Rust (cargo, clap, tests via assert_cmd), Lua (mise hooks), Bash (test runner), GitHub Actions YAML, TOML configs, Markdown.

**Spec:** `docs/superpowers/specs/2026-05-09-rename-to-chord-design.md`

---

## File Map

**Renamed:**
- `claude-env/` → `chord/` (subcrate directory, `git mv` preserves history)
- `claude-env/README.md` → `chord/README.md` (carried by directory rename)
- Each `sample/*/claude-env.toml` → `sample/*/chord.toml` (`git mv`, 14 files — see Task 3 list)

**Modified:**
- `chord/Cargo.toml` (package name, lib/bin names, description, repository URL)
- All `chord/src/**/*.rs` (~7 source files; sed `claude-env`/`claude_env`/`.claude-env` → `chord`)
- All `chord/tests/**/*.rs` (~21 test files; same sed)
- `metadata.lua` (PLUGIN.name)
- `hooks/backend_install.lua`, `hooks/backend_list_versions.lua`, `hooks/backend_exec_env.lua`
- Each `sample/*/.mise.toml` (replace `claude = "latest"` with `chord = "latest"`; do NOT touch `claude-code = "latest"`)
- `.github/workflows/publish.yml`, `.github/workflows/release.yml`, `.github/workflows/integration.yml` (manifest paths)
- `Dockerfile.test` (no path changes expected — verify)
- `test/integration.sh` (build path, env var, install command, file references)
- `README.md` (root, full rewrite)
- `CLAUDE.md` (root, full rewrite)
- `chord/README.md` (full rewrite)
- `.github/security-scan-instructions.md` (URL refs)

**Untouched (deliberately):**
- `sample/*/test.sh` files (they assert artifacts like `.mcp.json`, `.claude/settings.json` — those are Claude Code's own files, not ours)
- `sample/*/.mise.toml` line `claude-code = "latest"` (third-party npm package)
- `chord/src/inspect/scanner.rs` reads of `~/.claude.json` (Claude Code's own state file)
- `docs/superpowers/specs/` and `docs/superpowers/plans/` historical files
- `LICENSE`, `.gitignore`, `renovate.json`, `stylua.toml`, `docker-compose.test.yml` (no name refs)

---

## Substitution Reference

Three exact string transforms used throughout:

| Substring | Becomes | Where it appears |
|-----------|---------|------------------|
| `claude-env` | `chord` | doc comments, file paths, CLI name, branding text, `.toml`/`.lock` filenames |
| `claude_env` | `chord` | Rust `use` paths, lib identifiers, function name `write_claude_env_toml` |
| `.claude-env` (cache dir, **with dot**) | `.chord` | one occurrence in `chord/src/main.rs` line 325 |

These are safe to apply via `sed` because no other identifiers in the codebase contain these as substrings (verified). The order matters: do `claude-env` AFTER `.claude-env` to avoid double-replacement, OR use a single sed with both rules — `sed -E` handles them in order written.

For BSD `sed` on macOS use `sed -i ''` (with empty backup arg). For GNU `sed` (Linux/Docker) use `sed -i`. The plan uses BSD syntax in commands but agents should adapt to their environment.

---

## Pre-flight (Task 0)

**Files:** none modified.

- [ ] **Step 1: Confirm starting branch and base**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git branch --show-current
```

Expected: `feat/rename-to-chord`

```bash
git log --oneline -3
```

Expected: top of log shows the spec commits (`docs: switch rename target...`, `docs: add rename-to-tune design spec`, `docs: add repo cleanup implementation plan`).

- [ ] **Step 2: Confirm baseline tests pass before any rename**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude/claude-env && cargo test --quiet 2>&1 | tail -10 && cd ..
```

Expected: all tests pass, last line shows `test result: ok. N passed; 0 failed`.

If any test already fails before this work begins, STOP and escalate — the rename should not run on a broken baseline.

---

## Task 1: Rename Rust subcrate `claude-env/` → `chord/` (crate `rytmyk-chord`, binary `chord`)

**Files:**
- Rename: `claude-env/` → `chord/` (entire directory tree, via `git mv`)
- Modify: `chord/Cargo.toml` (package, lib, bin sections)
- Modify: every `chord/src/**/*.rs` and `chord/tests/**/*.rs` (sed)

This is the largest commit. Substitutions only — no logic changes.

- [ ] **Step 1: Rename the directory**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git mv claude-env chord
```

Expected: silent success. `git status` shows the rename staged.

- [ ] **Step 2: Update `chord/Cargo.toml`**

Replace the existing `[package]` section in `chord/Cargo.toml` with:

```toml
[package]
name = "rytmyk-chord"
version = "0.1.0"
edition = "2021"
description = "Declarative agent-tool environment manager"
license = "MIT"
repository = "https://github.com/rytmyk/chord"
keywords = ["agent", "claude", "mcp", "ai", "tooling"]
categories = ["command-line-utilities", "development-tools"]

[lib]
name = "chord"

[[bin]]
name = "chord"
path = "src/main.rs"
```

Then keep the existing `[dependencies]` and `[dev-dependencies]` sections unchanged.

Verify after editing:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
cat chord/Cargo.toml
```

Expected: shows the new `[package]`, `[lib]`, `[[bin]]` sections plus the original `[dependencies]` and `[dev-dependencies]`.

- [ ] **Step 3: Apply substitutions to all Rust source files**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
find chord/src chord/tests -type f -name '*.rs' -print0 | xargs -0 sed -i '' \
  -e 's|\.claude-env|.chord|g' \
  -e 's|claude-env|chord|g' \
  -e 's|claude_env|chord|g'
```

Order in the sed expression matters: the `.claude-env` rule must come before `claude-env` to avoid `chord` (without dot) replacing `.claude-env` mid-sed.

- [ ] **Step 4: Manually fix the migrate user-message**

The sed above leaves a now-incorrect message in `chord/src/main.rs` (the migrate command instructs the user to keep `claude = "latest"` in their `.mise.toml`, but the mise plugin name is now `chord`).

Open `chord/src/main.rs`, find the line that currently reads (around line 94 after substitution):

```rust
            println!("→ Keep `claude = \"latest\"` — that installs the chord binary itself");
```

Replace with:

```rust
            println!("→ Keep `chord = \"latest\"` — that installs the chord binary itself");
```

(Note: the substitution earlier turned `claude-env binary` into `chord binary`, which is correct. Only the `claude` plugin-name reference needs this manual fix.)

- [ ] **Step 5: Verify no `claude-env` / `claude_env` / `.claude-env` references survive in chord/**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -rn "claude-env\|claude_env\|\.claude-env" chord/src chord/tests chord/Cargo.toml 2>/dev/null
```

Expected: no output (exit code 1).

If anything survives: investigate, fix, re-run the grep. Do not proceed until clean.

Note: the README in `chord/README.md` still has old refs — leave those for Task 5 (docs commit).

- [ ] **Step 6: Build and run all tests**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude/chord && cargo test --quiet 2>&1 | tail -10 && cd ..
```

Expected: all tests pass, same count as the baseline from Pre-flight Step 2. Last line `test result: ok`.

If a test fails: read the failure, fix the cause (likely a missed substitution or a fixture file that needs updating), re-run.

- [ ] **Step 7: Commit**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git add -A chord/
git commit -m "chore: rename Rust subcrate claude-env/ → chord/ and crate to rytmyk-chord

Cargo package: claude-env → rytmyk-chord (org-prefixed for crates.io).
Binary and lib both named 'chord' via [[bin]] and [lib] overrides.
String identifiers swept across all sources and tests:
  claude-env  → chord  (file paths, CLI name, doc comments, branding)
  claude_env  → chord  (Rust use paths and lib identifier)
  .claude-env → .chord (cache dir constant)
Migrate command output filename and user message updated."
```

---

## Task 2: Rename mise plugin internals (`claude` → `chord`)

**Files:**
- Modify: `metadata.lua`
- Modify: `hooks/backend_install.lua`
- Modify: `hooks/backend_list_versions.lua`
- Modify: `hooks/backend_exec_env.lua`

These four files plus the README are all that reference the plugin's identity.

- [ ] **Step 1: Update `metadata.lua`**

Open `/Users/adrien/Dev/komune/experimentation/wasm/mise-claude/metadata.lua`. Replace the `name = "claude"` line so the file becomes:

```lua
PLUGIN = {
  name = "chord",
  description = "Bootstrap chord (Rytmyk's agent-tool environment manager) via mise",
  author = "rytmyk",
  version = "0.1.0",
  homepage = "https://github.com/rytmyk/chord",
  license = "MIT",
}
```

- [ ] **Step 2: Update `hooks/backend_install.lua`**

Apply two substitutions to the file. The relevant lines after substitution should look like:

- The crates.io URL line:
  ```lua
    url = "https://crates.io/api/v1/crates/rytmyk-chord/versions",
  ```
- The cargo install command line:
  ```lua
    cmd.exec(
      "cargo install rytmyk-chord"
      .. " --version " .. shell_quote(version)
      .. " --root " .. shell_quote(ctx.install_path)
      .. " --locked"
    )
  ```
- Any remaining doc comment that says "claude-env" should say "rytmyk-chord" (e.g. "Fetch the latest non-yanked rytmyk-chord version from crates.io.")
- The User-Agent string `"mise-claude/2.0"` → `"rytmyk-chord/2.0"`.

Easiest way:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
sed -i '' \
  -e 's|crates/claude-env/versions|crates/rytmyk-chord/versions|g' \
  -e 's|cargo install claude-env|cargo install rytmyk-chord|g' \
  -e 's|claude-env version|rytmyk-chord version|g' \
  -e 's|"mise-claude/2.0"|"rytmyk-chord/2.0"|g' \
  hooks/backend_install.lua
```

Verify:

```bash
grep -n "claude" hooks/backend_install.lua
```

Expected: no output (exit 1).

- [ ] **Step 3: Update `hooks/backend_list_versions.lua`**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
sed -i '' \
  -e 's|crates/claude-env/versions|crates/rytmyk-chord/versions|g' \
  -e 's|"mise-claude/2.0"|"rytmyk-chord/2.0"|g' \
  hooks/backend_list_versions.lua
```

Verify:

```bash
grep -n "claude" hooks/backend_list_versions.lua
```

Expected: no output.

- [ ] **Step 4: Update `hooks/backend_exec_env.lua`**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
sed -i '' \
  -e 's|"/claude-env"|"/chord"|g' \
  -e 's|/claude-env\b|/chord|g' \
  -e 's|claude-env\.toml|chord.toml|g' \
  -e 's|/claude-env "|/chord "|g' \
  hooks/backend_exec_env.lua
```

Then read the file and verify the binary path interpolation now reads `chord` and the presence check is for `chord.toml`. Manually inspect:

```bash
cat hooks/backend_exec_env.lua
```

Expected lines (after edits):
```lua
  local bin = bin_dir .. "/chord"
  ...
  local f = io.open(project_root .. "/chord.toml", "r")
```

If a substitution missed a spot, do a follow-up `sed` or hand-edit and re-verify with `grep -n "claude" hooks/backend_exec_env.lua` (must be empty).

- [ ] **Step 5: Verify no leftover `claude` references in plugin code**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -n "claude" metadata.lua hooks/*.lua 2>&1
```

Expected: no output (exit 1).

- [ ] **Step 6: Commit**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git add metadata.lua hooks/
git commit -m "chore: rename mise plugin internals (claude → chord)

PLUGIN.name = chord. Hooks now bootstrap rytmyk-chord from crates.io,
expose chord binary on PATH, trigger 'chord install' on shell entry
when chord.toml is present."
```

---

## Task 3: Rename samples (`claude-env.toml` → `chord.toml`, `claude` → `chord` in `.mise.toml`)

**Files** (15 sample directories — careful with `git mv` per file):
- `sample/mcp/claude-env.toml` → `sample/mcp/chord.toml`
- `sample/skillssh/claude-env.toml` → `sample/skillssh/chord.toml`
- `sample/spec/bmad/claude-env.toml` → `sample/spec/bmad/chord.toml`
- `sample/spec/gsd/claude-env.toml` → `sample/spec/gsd/chord.toml`
- `sample/spec/openspec/claude-env.toml` → `sample/spec/openspec/chord.toml`
- `sample/plugin/anthropics/claude-code/claude-env.toml` → `chord.toml`
- `sample/plugin/anthropics/claude-plugins-official/claude-env.toml` → `chord.toml`
- `sample/plugin/anthropics/financial-services-plugins/claude-env.toml` → `chord.toml`
- `sample/plugin/anthropics/knowledge-work-plugins/claude-env.toml` → `chord.toml`
- `sample/plugin/caveman/claude-env.toml` → `chord.toml`
- `sample/plugin/chrome-dev-tools/claude-env.toml` → `chord.toml`
- `sample/plugin/context7/claude-env.toml` → `chord.toml`
- `sample/plugin/superpowers/claude-env.toml` → `chord.toml`
- `sample/plugin/visual-explainer/claude-env.toml` → `chord.toml`

Plus 15 `.mise.toml` files getting one-line edit each: `claude = "latest"` → `chord = "latest"`. Critical: do NOT touch `claude-code = "latest"` — that's the third-party Claude Code CLI.

- [ ] **Step 1: Discover the actual list of sample dirs to be safe**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
find sample -name 'claude-env.toml' -print
```

Expected: 14 paths (financial-services-plugins is skipped via `.test-skip` but still has the file).

```bash
find sample -name '.mise.toml' -print | wc -l
```

Expected: 15 (one per sample dir).

- [ ] **Step 2: Rename every `claude-env.toml` → `chord.toml`**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
find sample -name 'claude-env.toml' -print0 | while IFS= read -r -d '' f; do
  git mv "$f" "$(dirname "$f")/chord.toml"
done
```

Verify:

```bash
find sample -name 'claude-env.toml' -print
```

Expected: no output (no files left with old name).

```bash
find sample -name 'chord.toml' -print | wc -l
```

Expected: 14 (matches Step 1's count).

- [ ] **Step 3: Edit `.mise.toml` files — replace `claude = "latest"` with `chord = "latest"`**

The substitution must be exact — only the bare `claude` line, not `claude-code`. Use a regex anchored on word boundary:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
find sample -name '.mise.toml' -print0 | xargs -0 sed -i '' \
  -e 's|^claude = "latest"$|chord = "latest"|'
```

Verify the replacement worked and `claude-code` is untouched:

```bash
grep -rn "^claude " sample/ 2>/dev/null
```

Expected: no output (no bare `claude` lines remain).

```bash
grep -rn "^chord " sample/ 2>/dev/null | wc -l
```

Expected: 15 (one per sample).

```bash
grep -rn "claude-code = " sample/ 2>/dev/null | wc -l
```

Expected: 15 (untouched).

- [ ] **Step 4: Commit**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git add -A sample/
git commit -m "chore: rename samples (claude-env.toml → chord.toml, claude → chord in .mise.toml)

Per-sample config file renamed; mise plugin declaration line updated.
The claude-code = \"latest\" lines (third-party Claude Code CLI) are
left untouched."
```

---

## Task 4: Update CI workflows + Docker test infrastructure

**Files:**
- Modify: `.github/workflows/publish.yml`
- Modify: `.github/workflows/release.yml`
- Modify: `.github/workflows/integration.yml` (verify; likely no changes)
- Modify: `Dockerfile.test` (verify; likely no changes)
- Modify: `test/integration.sh`

- [ ] **Step 1: Update `.github/workflows/publish.yml`**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
sed -i '' 's|claude-env/Cargo.toml|chord/Cargo.toml|g' .github/workflows/publish.yml
```

Verify:

```bash
grep -n "claude-env\|chord" .github/workflows/publish.yml
```

Expected: shows `chord/Cargo.toml` references; no `claude-env` references.

- [ ] **Step 2: Update `.github/workflows/release.yml` and `.github/workflows/integration.yml`**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
sed -i '' 's|claude-env/Cargo.toml|chord/Cargo.toml|g' .github/workflows/release.yml .github/workflows/integration.yml
grep -rn "claude-env" .github/workflows/ 2>/dev/null
```

Expected: no output (no leftover `claude-env` refs in any workflow).

- [ ] **Step 3: Verify `Dockerfile.test` needs no changes**

```bash
grep -n "claude-env\|claude_env" Dockerfile.test 2>&1
```

Expected: no output. (The Dockerfile uses `WORKDIR /app` and `ENTRYPOINT ["bash", "test/integration.sh"]` with no manifest paths.)

If output appears, sed it with the same `claude-env` → `chord` rule and re-grep until clean.

- [ ] **Step 4: Update `test/integration.sh`**

The test runner builds `claude-env` from source and walks each sample. After rename, it must build `chord` and look for `chord.toml`.

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
sed -i '' \
  -e 's|claude-env|chord|g' \
  -e 's|claude_env|chord|g' \
  -e 's|CLAUDE_ENV|CHORD|g' \
  test/integration.sh
```

This three-rule sweep covers everything: paths (`/app/claude-env/...` → `/app/chord/...` including the binary name in `target/release/claude-env` because `claude-env` matches as a substring twice), env-var name and references (the `CLAUDE_ENV` rule catches both `CLAUDE_ENV=` assignment and `"$CLAUDE_ENV"` references), config filename (`claude-env.toml` → `chord.toml`), and all branding strings (`Building claude-env`, `claude-env install failed`, etc.).

Verify:

```bash
grep -n "claude-env\|claude_env\|CLAUDE_ENV" test/integration.sh 2>&1
```

Expected: no output.

```bash
grep -n "chord\|CHORD" test/integration.sh
```

Expected: shows updated paths, env var, and labels.

- [ ] **Step 5: Run the integration suite end-to-end**

This is the gate — the full Docker-based test harness must pass.

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
mise run test 2>&1 | tail -10
```

Expected (last lines):
```
  Passed: 13
  Failed: 0

All tests passed!
```

If it fails:
- Read the build output. If `cargo build` failed, a Rust source rename is incomplete — go back to Task 1.
- If a sample failed an assertion, the most likely cause is a leftover `claude-env.toml` reference in a sample's `test.sh` (unlikely but possible).
- Do not commit until 13/13.

- [ ] **Step 6: Commit**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git add .github/workflows/ Dockerfile.test test/integration.sh
git commit -m "ci: update workflow + Docker manifest paths to chord/

Workflows now publish from chord/Cargo.toml. Integration runner builds
the chord binary, copies chord.toml into per-sample temp dirs, and
invokes 'chord install' against each sample."
```

---

## Task 5: Rewrite docs (`README.md`, `CLAUDE.md`, `chord/README.md`, security-scan-instructions)

**Files:**
- Modify: `README.md` (root, full rewrite)
- Modify: `CLAUDE.md` (root, full rewrite)
- Modify: `chord/README.md` (full rewrite)
- Modify: `.github/security-scan-instructions.md` (URL refs only)

- [ ] **Step 1: Replace root `README.md` contents**

Overwrite `/Users/adrien/Dev/komune/experimentation/wasm/mise-claude/README.md` with EXACTLY:

```markdown
# chord

Declarative agent-tool environment manager. Declare your MCP servers, skills, plugins, and CLI tools in one file — `chord install` handles the rest.

`chord` is part of the [rytmyk](https://github.com/rytmyk) toolchain. The repo also ships a [mise](https://mise.jdx.dev) backend plugin that bootstraps the binary automatically.

## Install

Via mise (recommended):

```bash
mise plugin install chord https://github.com/rytmyk/chord
```

Then, in your project:

```toml
# .mise.toml
[tools]
chord = "latest"
```

```bash
mise install
```

Or directly via cargo:

```bash
cargo install rytmyk-chord
```

(The cargo package is `rytmyk-chord` — org-prefixed because the bare name `chord` is held by an unrelated dormant crate. The installed binary is just `chord`.)

## Quick start

Declare what you want in `chord.toml`:

```toml
[mcp]
context7 = "2.1.4"

[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"

[plugins]
"upstash/context7/context7-plugin@context7-marketplace" = "latest"

[cli]
gsd = "1.22.4"
```

Install:

```bash
chord install
```

Audit your environment:

```bash
chord inspect
```

When you `cd` into a directory containing `chord.toml`, the mise plugin runs `chord install --idempotent --quiet` automatically. Missing tools are installed silently; nothing happens if everything is up to date.

## What can you install?

### MCP servers

MCP servers extend what your agent can do — browse the web, access documentation, generate UI components, and more. Short aliases are available for popular servers:

| You write | What gets installed |
|-----------|--------------------|
| `mcp/context7` | `@upstash/context7-mcp` |
| `mcp/chrome-devtools` | `chrome-devtools-mcp` |
| `mcp/shadcn` | `shadcn` |

You can also use any npm package name directly.

### Workflow tools

Workflow tools add structured methodologies, slash commands, and agents to Claude Code. They set themselves up in your project when installed.

| You write | What it does |
|-----------|--------------|
| `spec/gsd` | GSD — structured project execution workflow |
| `spec/bmad` | BMAD Method — product development agents and commands |
| `spec/openspec` | OpenSpec — API specification tool |

### Skills

Skills from [skills.sh](https://skills.sh) teach Claude Code best practices for specific frameworks and topics — no server required.

```toml
# chord.toml
[skills]
"vercel-labs/next-skills/next-best-practices" = "latest"
"anthropics/skills/frontend-design" = "latest"
```

### Plugins

Native Claude Code plugins from GitHub-based marketplaces.

```toml
# chord.toml
[plugins]
"anthropics/claude-code/commit-commands@claude-code-plugins" = "latest"
"upstash/context7/context7-plugin@context7-marketplace" = "latest"
```

## Extra configuration

To pass additional settings to MCP servers, create a `.mcp-config.toml` file in your project:

```toml
["@upstash/context7-mcp"]
args = ["--api-key", "${CONTEXT7_API_KEY}"]
env = { LOG_LEVEL = "debug" }
```

- `args` — extra arguments passed to the server
- `env` — environment variables for the server
- `${VAR}` references are replaced with values from your environment

## How it works

The mise plugin (`hooks/`, `metadata.lua`) does only two things:

1. **List/install** — Queries crates.io for `rytmyk-chord` versions and runs `cargo install rytmyk-chord --locked` when mise resolves the tool.
2. **Shell entry** — Adds the binary to `PATH` and runs `chord install --idempotent --quiet` automatically when `chord.toml` exists in the project root.

All actual tool management — MCP server installs, plugin marketplace fetches, skills setup, lockfile maintenance — lives in the `chord` Rust binary (`chord/src/`).

## Local development

```bash
# Link the plugin locally
mise plugin link chord ./
mise install

# Declare tools in chord.toml, then install them
chord install

# Inspect current state
chord inspect
```

## Roadmap

`chord` will grow into an agent-agnostic package manager. Today it targets Claude Code; planned support includes Codex, OpenCode, and aider.

## Contributing

Contributions are not open at this time. This project is in early development and not yet accepting external pull requests.

## License

MIT
```

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -c "claude-env\|claude_env" README.md
```

Expected: `0`.

- [ ] **Step 2: Replace root `CLAUDE.md` contents**

Overwrite `/Users/adrien/Dev/komune/experimentation/wasm/mise-claude/CLAUDE.md` with EXACTLY:

```markdown
# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

A [mise](https://mise.jdx.dev) backend plugin (Lua) that bootstraps the `chord` Rust CLI via `cargo install rytmyk-chord`. All agent-tool management — MCP servers, skills, plugins, CLI tools — is delegated to `chord`. The plugin itself does nothing more than ensure the binary is installed and triggered on shell entry.

The cargo package is `rytmyk-chord` (org-prefixed because the bare `chord` crate is held by an unrelated dormant project). The binary, config file, and mise plugin all use the short name `chord` thanks to `[lib]` and `[[bin]]` overrides in `chord/Cargo.toml`.

## Local Development

```bash
mise plugin link chord ./
mise install               # bootstraps chord via cargo install rytmyk-chord
chord install              # installs tools declared in chord.toml
chord inspect              # audit current state
```

## Architecture

The plugin implements three of mise's Lua backend hooks. All hooks live in `hooks/`. There is no shared `lib/` — each hook is self-contained, and the small semver helpers are intentionally duplicated across `backend_install.lua` and `backend_list_versions.lua` (both files note this in a comment).

- `hooks/backend_list_versions.lua` — Queries `https://crates.io/api/v1/crates/rytmyk-chord/versions` and returns non-yanked versions sorted ascending.
- `hooks/backend_install.lua` — Resolves `latest` via the same crates.io endpoint, then runs `cargo install rytmyk-chord --version <v> --root <install_path> --locked`. Writes a `.installed` sentinel that mise uses to confirm the install succeeded.
- `hooks/backend_exec_env.lua` — Adds `<install_path>/bin` to `PATH`. If `chord.toml` exists in `pwd`, runs `chord install --idempotent --quiet` so missing tools are filled in transparently on shell entry. Failures are logged to stderr but do not break the shell.

`metadata.lua` declares the plugin name, version, author, and license for mise.

## Code Conventions

- Lua with LDoc-style annotations (`--- @param`, `--- @return`)
- Format with [StyLua](https://github.com/JohnnyMorganz/StyLua) (config in `stylua.toml`)
- All shell interpolation goes through `shell_quote()` (defined in each hook that needs it)

## Project Layout

- Root: the mise backend plugin (`hooks/`, `metadata.lua`, `mise.toml` for dev tasks, `Dockerfile.test` + `docker-compose.test.yml` for the test runner)
- `chord/`: the Rust CLI as a subcrate (its own `Cargo.toml`, `mise.toml`, `README.md`, `src/`, `tests/`)
- `sample/`: usage examples organized by tool category (`mcp/`, `skillssh/`, `spec/`, `plugin/`). Each sample has a `.mise.toml`, a `chord.toml`, and a `test.sh` exercising the install.
- `test/integration.sh`: end-to-end harness — builds chord, walks every sample, runs `chord install`, and asserts artifacts via each sample's `test.sh`.
- `docs/superpowers/`: design specs and implementation plans (historical record, kept).
```

Verify:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -c "claude-env\|claude_env" CLAUDE.md
```

Expected: `0`.

- [ ] **Step 3: Replace `chord/README.md` contents**

Overwrite `/Users/adrien/Dev/komune/experimentation/wasm/mise-claude/chord/README.md` with EXACTLY:

```markdown
# chord

Declarative agent-tool environment manager. Declare your MCP servers, skills, plugins, and CLI tools in one file — `chord install` handles the rest.

## Install

```bash
cargo install rytmyk-chord
```

The cargo package is `rytmyk-chord`; the installed binary is `chord`.

## Quick Start

Create `chord.toml` in your project root:

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
chord install
```

## Commands

| Command | Description |
|---------|-------------|
| `chord install` | Install from lockfile (or resolve + create lockfile) |
| `chord update` | Check for updates, show changelogs |
| `chord update <tool>` | Update a single tool |
| `chord diff <tool>` | Show changelog between versions |
| `chord list` | Show installed tools and status |
| `chord add <tool>` | Add a tool to config |
| `chord remove <tool>` | Remove tool and clean up |

## How It Works

1. Reads `chord.toml` for declared tools
2. Compares against `chord.lock` to determine what needs installing
3. Installs each tool sequentially (no concurrency issues)
4. Writes config files (`.mcp.json`, `.claude/settings.json`)
5. Updates `chord.lock` with resolved versions

Packages are cached globally at `~/.chord/packages/`.
```

Verify:

```bash
grep -c "claude-env\|claude_env" /Users/adrien/Dev/komune/experimentation/wasm/mise-claude/chord/README.md
```

Expected: `0`.

- [ ] **Step 4: Update `.github/security-scan-instructions.md`**

The scan-instructions doc references the cargo install command name. Update:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
sed -i '' \
  -e 's|cargo install claude-env|cargo install rytmyk-chord|g' \
  -e 's|claude-env|chord|g' \
  .github/security-scan-instructions.md
grep -n "claude-env" .github/security-scan-instructions.md 2>&1
```

Order matters: the more-specific `cargo install claude-env → cargo install rytmyk-chord` rule must come first, so the second sweep doesn't accidentally make it `cargo install chord`.

Expected: no output (no leftover `claude-env` references).

Manually inspect the file to make sure prose still reads coherently after substitution. If something reads awkwardly, hand-edit.

- [ ] **Step 5: Commit**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git add README.md CLAUDE.md chord/README.md .github/security-scan-instructions.md
git commit -m "docs: rewrite README, CLAUDE.md, chord/README.md for chord branding

Top-level README now describes chord as agent-tool environment
manager + mise plugin bootstrap. CLAUDE.md updated for the renamed
subcrate and binary. Security-scan-instructions updated to reference
the new cargo install command (rytmyk-chord)."
```

---

## Task 6: Update repository URL metadata

**Files:**
- Modify: `chord/Cargo.toml` (already set in Task 1, but verify the `repository` field reads `https://github.com/rytmyk/chord`)
- Modify: any remaining `komune-io/mise-claude` URL references

- [ ] **Step 1: Sweep for old repo URL references**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -rn "komune-io/mise-claude\|komune-io" \
  --include='*.md' --include='*.toml' --include='*.lua' --include='*.yml' --include='*.yaml' \
  --exclude-dir=docs --exclude-dir=.git --exclude-dir=target . 2>/dev/null
```

Expected: any remaining hits (likely none, as Tasks 1 + 5 covered Cargo.toml and READMEs).

- [ ] **Step 2: Apply substitutions to any remaining files**

For each file the grep returned, run:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
sed -i '' 's|komune-io/mise-claude|rytmyk/chord|g; s|komune-io|rytmyk|g' <FILEPATH>
```

Re-run the grep until it returns no output.

- [ ] **Step 3: Final repo URL verification**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -rn "github.com/rytmyk/chord" \
  --include='*.md' --include='*.toml' --include='*.lua' \
  --exclude-dir=docs --exclude-dir=.git --exclude-dir=target . 2>/dev/null | wc -l
```

Expected: at least 3 (Cargo.toml `repository`, root README, metadata.lua `homepage`).

- [ ] **Step 4: Commit**

If anything was changed in this task:

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git add -A
git commit -m "chore: update repository URL to rytmyk/chord in metadata"
```

If `git status` shows nothing to commit (Tasks 1 + 5 already covered everything), skip this commit and proceed to Task 7.

---

## Task 7: Final verification + push

**Files:** none modified.

- [ ] **Step 1: Full grep audit — no `claude-env` / `claude_env` / `mise-claude` survives anywhere outside historical docs**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
grep -rn "claude-env\|claude_env\|mise-claude" \
  --include='*.md' --include='*.toml' --include='*.lua' --include='*.rs' --include='*.sh' --include='*.yml' --include='*.yaml' --include='*.json' \
  --exclude-dir=docs --exclude-dir=.git --exclude-dir=target --exclude-dir=node_modules . 2>/dev/null
```

Expected: no output.

If output appears, read each hit:
- If it's in `docs/superpowers/specs/` or `docs/superpowers/plans/`: leave it, those are historical (and the `--exclude-dir=docs` should have skipped them — investigate why it didn't).
- Otherwise: fix the missed reference, commit as a "chore: catch leftover X reference" follow-up commit.

- [ ] **Step 2: Final integration test**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
mise run test 2>&1 | tail -10
```

Expected:
```
  Passed: 13
  Failed: 0

All tests passed!
```

- [ ] **Step 3: Final git status check**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git status
git log --oneline origin/feat/rename-to-chord..HEAD 2>/dev/null || git log --oneline -10
```

Expected: clean working tree on `feat/rename-to-chord`. Log shows the 5 or 6 rename commits since the spec commit.

- [ ] **Step 4: Push**

```bash
cd /Users/adrien/Dev/komune/experimentation/wasm/mise-claude
git push -u origin feat/rename-to-chord
```

Expected: branch published to origin.

- [ ] **Step 5: Open PR (manual / via gh)**

Once PR #1 (the cleanup PR) merges to main, this branch must be rebased onto the new main, then opened as PR #2:

```bash
git fetch origin
git rebase origin/main
git push --force-with-lease
gh pr create --title "Rename to chord (rytmyk/chord)" --body "$(cat <<'EOF'
## Summary
- Renames the project from claude-env/mise-claude to chord (cargo crate rytmyk-chord, binary chord, mise plugin chord, config file chord.toml).
- No behavior changes — pure rebrand. Six commits, one per layer (Rust, Lua plugin, samples, CI/Docker, docs, repo URL).
- Spec: docs/superpowers/specs/2026-05-09-rename-to-chord-design.md

## Out of scope (separate manual steps for the maintainer)
- Create rytmyk org on GitHub (if not exists) and transfer the repo, renaming to chord during transfer.
- Reattach CI secrets (CARGO_REGISTRY_TOKEN, CLAUDE_API_KEY) on the transferred repo.
- Run cargo publish on rytmyk-chord v0.1.0 from the new repo.

## Test plan
- [x] cargo test passes locally (13/13 chord crate tests)
- [x] mise run test passes (13/13 sample integration tests)
- [ ] CI pipeline green on the PR (integration + security review)
EOF
)"
```

---

## Done

After PR #2 lands and the user performs the GitHub repo transfer + `cargo publish rytmyk-chord`:

- `cargo install rytmyk-chord` installs `chord` binary
- `mise plugin install chord https://github.com/rytmyk/chord` works
- A project with `chord.toml` and `chord = "latest"` in `.mise.toml` auto-installs everything on `mise install` + shell entry
- The historical `docs/superpowers/specs/2026-04-*-claude-env-*-design.md` and related plans remain as record of the project's evolution
