# TUI Phase 2: Add, Remove, Drift Install, Scope Switch

**Date:** 2026-05-14
**Status:** Approved
**Phase:** 2 of 2 (closes the April 2026 TUI roadmap)

## Problem

The current `chord inspect --tui` is read-only except for a single action (`e` toggles a plugin's `enabledPlugins` entry in the global `~/.claude/settings.json`). Users browsing their environment can see what is installed, declared, or drifted — but cannot act on it without dropping back to the shell to run `chord install`, `chord remove`, or edit `chord.toml` by hand.

The April 2026 TUI spec named three Phase 2 features explicitly: **install missing / fix drift**, **remove tool/plugin**, **scope switch (project ↔ global)**. This spec defines all three and the supporting refactor.

## Solution

Surface five new TUI actions, backed by a shared `src/operations/` module that both the CLI (`chord add`, `chord remove`, `chord install`) and the TUI invoke. Slow subprocess work (`npm`, `claude`, `npx`) is shown by dropping out of the alternate screen so the user sees the install stream live — the same trick `lazygit` uses.

### Scope (this spec)

- `a` — Add: free-form prompt `<section>:<name>@<version>` → writes `chord.toml` → installs.
- `d` — Remove: chord-managed items only → mirrors `chord remove` (strip chord.toml + lockfile + `.mcp.json` + delete package dir).
- `r` — Drift install: per-item re-install for entries declared in `chord.toml` but not on disk.
- `R` — Reconcile: runs the full `chord install` plan from inside the TUI.
- `e` — Scope picker: replaces the current ConfirmDisable popup with a modal that toggles project/global independently.
- Implements the currently-stubbed `chord add` CLI command (`main.rs:68` — `"not yet implemented: add {tool}"`).
- Extracts `run_install` / `run_remove` from `main.rs` into `src/operations/` so CLI and TUI share one code path.

### Out of scope

- Async install with in-TUI spinner. Drop-to-inline is the chosen UX. Threading + `mpsc` channels for live progress can come later.
- "Promote-from-tree" add flow (e.g. right-click on a cached-but-not-declared plugin). Free-form prompt is the v1 entry point.
- Add for arbitrary unmanaged items (standalone skills, manual MCPs). Only goes through the prompt with a known section.
- Remove of cached or standalone items. `d` only acts on chord-managed entries; cached-but-not-declared plugins keep the existing `e` toggle.
- Per-scope install location. Plugin cache stays at `~/.claude/plugins/cache/` regardless of which settings.json enables it.
- Concurrent file-edit safety. No flock on `chord.toml` or settings.json; last writer wins.
- Autocomplete / search inside the Add prompt.
- Undo for remove.
- Refresh strategy beyond full re-scan after each op.

## User Stories

1. **Add via TUI.** I open `chord inspect --tui`, press `a`, type `mcp:context7@latest`, hit Enter. The screen drops to inline output; I see `npm install` run; chord.toml gains the entry; the TUI re-enters and the new MCP server is in the tree.
2. **Fix drift.** A previous teammate added `cli:gh-dash@latest` to chord.toml but never ran `chord install`. I see the entry in the tree marked with a red `⚠`. I press `r`. Same drop-to-inline; on return the entry is fully installed and the warning is gone.
3. **Reconcile all.** I clone a fresh project. The tree shows several drift entries. I press `R`. Full `chord install` runs; the tree refreshes clean.
4. **Remove a tool I no longer want.** I navigate to `mcp:memory`, press `d`, confirm with `y`. chord.toml entry, lockfile row, `.mcp.json` server entry, and the package directory all disappear in one shot.
5. **Enable in project only.** I have `superpowers` enabled globally and want it project-scoped for an experiment. I press `e` on the plugin row. Modal shows `Project: ○ disabled / Global: ● enabled`. I press `p` (stages project = on), `g` (stages global = off), Enter. Both settings.json files update in lockstep.

## Architecture

### New module layout

```
src/operations/
  mod.rs        // OpContext, OperationError, OperationOutcome
  add.rs        // pub fn add(spec: &AddSpec, ctx: &OpContext) -> Result<...>
  remove.rs     // pub fn remove(name: &str, ctx: &OpContext) -> Result<...>
  install.rs    // pub fn install_all(ctx) -> ...
                // pub fn install_one(name: &str, ctx) -> ...
  scope.rs      // pub fn set_plugin_enabled(id, scope, enabled, ctx) -> ...
```

`OpContext` is a thin struct replacing the ad-hoc `&Path` arguments scattered through `main.rs` today:

```rust
pub struct OpContext<'a> {
    pub project_root: &'a Path,
    pub home_dir: &'a Path,
    pub packages_dir: &'a Path,
    pub verbose: bool,
}
```

`AddSpec` is the parsed `<section>:<name>@<version>` triple — the same parser used by the new `chord add` CLI and the TUI prompt:

```rust
pub struct AddSpec {
    pub section: Section,        // Mcp | Cli | Skills | Plugins
    pub name: String,
    pub version: String,         // defaults to "latest"
}

impl AddSpec {
    pub fn parse(input: &str) -> Result<Self, ParseError> { ... }
}
```

### `OperationError`

```rust
pub enum OperationError {
    ConfigRead(io::Error),
    ConfigParse(toml::de::Error),
    ConfigWrite(io::Error),
    LockfileWrite(io::Error),
    NotFound(String),        // remove: tool not in chord.toml
    Duplicate(String),       // add: tool already declared
    Parse(String),           // AddSpec parse failure
    Install(InstallError),   // wraps existing installer error
    Settings(io::Error),     // settings.json read/write
    McpConfig(io::Error),    // .mcp.json edit (remove path)
}
```

Implements `Display`. CLI prints to stderr and exits 2; TUI surfaces as a status-bar message.

### `main.rs` changes

- `run_install` (lines 208–322) becomes a one-liner calling `operations::install::install_all(&ctx)`.
- `run_remove` (lines 124–206) becomes a one-liner calling `operations::remove::remove(name, &ctx)`.
- `Command::Add { tool }` stub now calls `operations::add::add(&AddSpec::parse(tool)?, &ctx)` — the stubbed CLI command becomes real.
- `Command::Update` and `Command::Diff` stubs remain stubs (out of scope).

### TUI changes

**`tui/actions.rs`** is deleted. `toggle_plugin` moves to `operations::scope::set_plugin_enabled` and gains a `Scope` parameter (currently global-only).

**New `Mode` variants** in `tui/app.rs`:

```rust
pub enum Mode {
    Normal,
    Search,
    ViewMarkdown,
    AddPrompt,       // input field active
    ConfirmRemove,   // y/N for delete
    ScopePicker,     // plugin scope modal
}
```

`Mode::ConfirmDisable` is removed (subsumed by `ScopePicker`).

**New `App` fields**:

```rust
pub add_input: String,                   // accumulates AddPrompt text
pub pending_remove: Option<String>,      // tool name awaiting ConfirmRemove
pub scope_target: Option<ScopeTarget>,   // modal state for ScopePicker
```

```rust
pub struct ScopeTarget {
    pub plugin_id: String,
    pub current: ScopeState,
    pub staged: ScopeState,
}

pub struct ScopeState {
    pub project: bool,
    pub global: bool,
}
```

`pending_disable: Option<String>` is removed.

**New `TreeNode` fields** in `tui/tree.rs`:

```rust
pub drift: bool,        // from AuditEntry::drift
pub managed: bool,      // from AuditEntry::management == Managed
```

Threaded through `TreeNode::leaf` and `build_tree`. Used by the renderer (drift entries get `⚠` prefix and red foreground) and by the handler (`d` disabled on `!managed` nodes).

Drift entries are exempt from the "enabled only" filter. The check in `tui/app.rs::flatten_node` becomes:

```rust
if enabled_only
    && !node.enabled
    && node.kind != NodeKind::SectionHeader
    && !node.drift
{
    return;
}
```

### Drop-to-inline transition

Shared helper in `tui/mod.rs`:

```rust
pub fn run_inline<F, T>(terminal: &mut Terminal<...>, header: &str, f: F) -> io::Result<T>
where F: FnOnce() -> T
{
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    println!("▶ {header}");
    let result = f();
    println!("\n[Press any key to return]");
    let _ = event::read()?;
    execute!(terminal.backend_mut(), EnterAlternateScreen)?;
    enable_raw_mode()?;
    Ok(result)
}
```

Called by `a`, `d`, `r`, and `R`. Not called by `e` (scope toggles are fast file writes, same as today's behavior).

### Tree reload after any op

```rust
impl App {
    pub fn reload(&mut self, project_root: &Path, home_dir: &Path, config: &Config) {
        // Re-scan all categories, rebuild tree.
        // Preserve `selected` by node name+kind where possible (fall back to 0).
        // Preserve `show_enabled_only`, `focus`. Force `mode = Normal`.
        // Call update_preview().
    }
}
```

After add/remove, the config is re-read from disk (file changed). For scope and drift, config is unchanged so the existing reference is reused.

## Data flow per operation

### Add (`a`)

1. `Mode::AddPrompt` opens modal with empty input.
2. Printable chars + Backspace + Esc + Enter (standard input handling).
3. Enter → `AddSpec::parse(&input)`. Parse error → status-bar, modal stays open.
4. Drop to inline.
5. `operations::add::add(&spec, &ctx)`:
   - Read `chord.toml`.
   - Reject if `spec.name` already present in any section → `OperationError::Duplicate`.
   - Insert into the requested section's `BTreeMap`.
   - Write `chord.toml` (`toml::to_string_pretty` + `fs::write`).
   - Call `install::install_one(&spec.name, &ctx)`.
6. `install_one` builds a `Plan` from the just-written config + current lockfile, filters `plan.actions` to entries matching `spec.name`, runs the installer, updates the lockfile.
7. Re-enter alt-screen. `App::reload(...)`.

### Remove (`d`)

1. Selection must satisfy `node.managed`. If not → status-bar: "Not in chord.toml".
2. `Mode::ConfirmRemove`. `y`/`Y`/Enter → proceed; `n`/`N`/Esc → cancel.
3. Drop to inline.
4. `operations::remove::remove(name, &ctx)`:
   - Read chord.toml; locate section by membership; capture original TOML string for rollback.
   - Strip from in-memory config; write chord.toml.
   - If MCP: `mcp_config::remove_server(...)`. On failure → restore chord.toml from the captured string, return `McpConfig` error.
   - Strip lockfile entry; write lockfile. On failure → warn but continue (matches current behavior).
   - Delete package directory (`std::fs::remove_dir_all`). `NotFound` is OK.
5. Re-enter + reload.

### Drift install (`r`)

1. Selection must satisfy `node.drift`. Else → status-bar.
2. Drop to inline.
3. `operations::install::install_one(&node.name, &ctx)` — same code path as Add step 6, without the chord.toml write.
4. Re-enter + reload.

### Reconcile (`R`)

1. Drop to inline.
2. `operations::install::install_all(&ctx)`.
3. Re-enter + reload.

### Scope toggle (`e`)

1. Selection must be `NodeKind::Plugin`. Else → status-bar.
2. Read both settings.json files once, build `ScopeState { project, global }`, store as `App::scope_target.current`. Initialize `staged = current`.
3. `Mode::ScopePicker`. `p` flips `staged.project`; `g` flips `staged.global`. Live re-render reflects staged state.
4. Enter:
   - For each scope whose `staged != current`: `operations::scope::set_plugin_enabled(plugin_id, scope, staged_value, &ctx)`.
   - Each call read-modify-writes the respective settings.json.
5. Esc → discard.
6. No inline drop (fast). `App::reload(...)`.

## Error handling matrix

| Operation | Failure point | Behavior |
|---|---|---|
| Add | Parse error | Status-bar (TUI) / stderr+exit-2 (CLI). Modal stays open in TUI. |
| Add | Duplicate | Same UX. chord.toml not touched. |
| Add | chord.toml write | Bail before install. Original unchanged. |
| Add | Install fails after chord.toml write | Entry stays in chord.toml (now drift). Status: "Added but install failed — drift". User retries with `r`. Rationale: removing the entry hides intent. |
| Remove | Not in chord.toml | Status-bar; no state change. |
| Remove | chord.toml write | Bail. Lockfile + filesystem untouched. |
| Remove | `.mcp.json` write | **Restore chord.toml from in-memory backup**, return `McpConfig` error. |
| Remove | Package dir delete | Warn, continue (matches current behavior at `main.rs:192`). |
| Remove | Lockfile write | Warn, continue. |
| Drift install | Install fails | Status-bar; drift stays drift. No rollback needed. |
| Scope | Read failure on modal open | Status-bar; modal does not open. |
| Scope | One scope write succeeds, other fails | First stands. Status: "Project applied; global failed: …". No rollback (writes are idempotent; user re-presses `e`). |
| Reconcile | Per-action errors | Existing `Reporter` behavior — process continues across actions (matches current CLI). |

### TUI-specific concerns

- **Subprocess SIGINT**: while in inline output mode, Ctrl-C reaches the child. On child exit (any cause), we always re-enter alt-screen and refresh. If re-entry itself fails, panic with the original error (already the pattern at `tui/mod.rs:54-57`).
- **Tree-reload failure**: if scanner panics or `build_tree` returns empty unexpectedly, swallow and keep the old tree. Status-bar: "Refresh failed; tree may be stale".

## UX details

### Key map (Normal mode, Tree focus)

| Key | Action | Applies to | Confirms? |
|---|---|---|---|
| `a` | Open Add prompt | Anywhere | Implicit (Enter submits) |
| `d` | Remove | `node.managed == true` | Yes (`ConfirmRemove`) |
| `r` | Drift install | `node.drift == true` | No |
| `R` | Reconcile all | Anywhere | No |
| `e` | Scope picker | `node.kind == Plugin` | Yes (modal Apply) |

Unchanged: `q`/`Esc` quit, `i` enabled-only filter, `/` search, `v` fullscreen view, `Tab` focus, navigation keys.

### Add prompt

```
┌─ Add tool ─────────────────────────────────────────┐
│ <section>:<name>@<version>                         │
│ > mcp:context7@latest_                             │
│                                                    │
│ section ∈ {mcp, cli, skills, plugins}              │
│ [Enter] add  [Esc] cancel                          │
└────────────────────────────────────────────────────┘
```

Parser rules:

1. Split on the **first** `:` → `(section, rest)`. Section must be one of `mcp`, `cli`, `skills`, `plugins`. Else reject.
2. Split `rest` on the **last** `@` → `(name, version)`. If no `@` present, `name = rest`, `version = "latest"`.
3. Reject empty `name`. Reject empty `version` if `@` was present (trailing `@`).

Plugin names contain `@marketplace` (e.g. `owner/repo/plugin@marketplace`), so a plugin add looks like:

```
plugins:anthropics/claude-plugins-official/code-review@claude-plugins-official@latest
```

The "last `@`" rule means the trailing `@latest` is the version; the inner `@claude-plugins-official` stays inside `name`. To install with implicit `latest`, drop the trailing `@version`:

```
plugins:anthropics/claude-plugins-official/code-review@claude-plugins-official
```

Errors:
- Unknown section → reject with explicit message
- Empty name → reject
- Empty version after `@` → reject
- Missing `:` → reject (the section is required)

### Remove confirm

```
┌─ Confirm remove ───────────────────────────────────┐
│                                                    │
│         Remove tool from chord.toml?               │
│                                                    │
│                  context7                          │
│                                                    │
│   Also deletes ~/.chord/packages/context7          │
│   and removes from .mcp.json.                      │
│                                                    │
│         [Y]es / Enter         [N]o / Esc           │
└────────────────────────────────────────────────────┘
```

### Scope picker (replaces ConfirmDisable)

```
┌─ Plugin scope: superpowers@official ───────────────┐
│                                                    │
│   Project:  ○ disabled                             │
│   Global:   ● enabled                              │
│                                                    │
│   [p] toggle project    [g] toggle global          │
│   [Enter] apply         [Esc] cancel               │
└────────────────────────────────────────────────────┘
```

Modal re-renders live as `p`/`g` flip the staged state.

### Drift visual

Drift entries render with `⚠ ` prefix and red foreground (`Color::Red`). They are exempt from the "enabled only" filter — drift is precisely the state where the user needs to act, so it's always visible.

The detail pane gains a metadata line for drift:

```
Status: ⚠ drift (declared, not installed)
```

## Testing strategy

### Unit tests — `tests/unit/operations/`

**`add_test.rs`**:

- `AddSpec::parse("mcp:context7@latest")` → `{ Mcp, "context7", "latest" }`
- `AddSpec::parse("cli:foo")` → version defaults to `"latest"`
- `AddSpec::parse("plugins:owner/repo/plugin@marketplace@latest")` → `{ Plugins, "owner/repo/plugin@marketplace", "latest" }` (last-`@` rule)
- `AddSpec::parse("plugins:owner/repo/plugin@marketplace")` → version defaults to `"latest"`, name keeps the inner `@marketplace`
- `AddSpec::parse("skills:vercel-labs/next-skills/next-best-practices")` → version defaults to `"latest"`
- `AddSpec::parse("bogus:foo")` → `Parse` error
- `AddSpec::parse("mcp:")` / `":foo@1"` / `""` / `"foo"` → `Parse` error
- `AddSpec::parse("mcp:foo@")` → `Parse` error (trailing `@`)
- `add()` with new tool → chord.toml gains entry; lockfile updated
- `add()` with duplicate name in any section → `Duplicate` error; chord.toml unchanged
- chord.toml roundtrips cleanly (BTreeMap key order is stable)

**`remove_test.rs`**:

- Remove an MCP tool → chord.toml entry gone, `.mcp.json` server gone, lockfile entry gone, package dir gone
- Remove a non-existent tool → `NotFound`, no side effects
- Remove with `.mcp.json` write failure (read-only fixture) → chord.toml restored from in-memory backup
- Remove with missing package dir → succeeds (current behavior preserved)

**`install_test.rs`**:

- `install_one` filters plan to a single name, leaves others alone
- `install_one` for an entry not in chord.toml → `NotFound`
- Existing `tests/unit/resolver_test.rs` and `tests/integration/install_test.rs` continue to pass — the move is mechanical, behavior preserved

**`scope_test.rs`**:

- `set_plugin_enabled(id, Project, true)` writes `<project>/.claude/settings.json`
- `set_plugin_enabled(id, Global, false)` strips key from `~/.claude/settings.json`
- Toggling one scope doesn't touch the other
- Creates parent dir if missing (mirrors existing `actions::toggle_plugin` at `tui/actions.rs:33`)

### TUI handler tests — extend `tui/handler.rs::tests`

To make operations mockable, wrap them in a trait:

```rust
pub trait OpRunner {
    fn add(&self, spec: &AddSpec) -> Result<(), OperationError>;
    fn remove(&self, name: &str) -> Result<(), OperationError>;
    fn install_one(&self, name: &str) -> Result<(), OperationError>;
    fn install_all(&self) -> Result<(), OperationError>;
    fn set_scope(&self, plugin_id: &str, scope: Scope, enabled: bool)
        -> Result<(), OperationError>;
}
```

Production impl wraps `operations::*`. Test impl records calls.

Cases:

- Add prompt: `a` switches to `AddPrompt`; chars accumulate; Backspace pops; Esc cancels; Enter with valid spec calls runner; Enter with invalid spec keeps modal open + sets status
- Remove: `d` on managed → `ConfirmRemove`; `d` on unmanaged → status-bar, no mode change; y/Enter calls runner; n/Esc cancels
- Drift install: `r` on drift → runner called; `r` on non-drift → status-bar, no call
- Scope picker: `e` on plugin → `ScopePicker` with staged = current; `p`/`g` flip staged; Enter calls runner only for changed scopes; Esc discards

### Integration tests

- `tests/integration/cli_add_test.rs` (new) — `chord add mcp:context7@latest` end-to-end, verifying chord.toml mutation and exit code. Now that the CLI command is real, this is required coverage.
- Existing integration tests continue to pass.

### Not tested

- Actual TUI rendering — consistent with current test approach
- Drop-to-inline transition itself (raw mode toggle is integration-territory); the operation behind it is unit-tested directly
- Concurrent file edits — out of scope

## Risks

- **Wider blast radius than typical PR.** Refactoring `main.rs::run_install` + `run_remove` into `operations/` touches the install hot path. Mitigation: the move is mechanical (cut, paste, adjust signatures); existing unit and integration tests guard behavior.
- **Drop-to-inline + raw mode toggling can leak on panic.** If the operation panics between `LeaveAlternateScreen` and `EnterAlternateScreen`, the terminal is left in a broken state. Mitigation: same as `tui/mod.rs` does today — best-effort cleanup; the user can run `reset` if it happens. Adding a panic guard (drop guard) is a defensible future enhancement, marked out of scope here.
- **Partial-failure rollback complexity in remove.** The MCP-config-fail rollback path is the trickiest correctness case. Mitigation: explicit unit test with a read-only `.mcp.json` fixture.
- **TUI mode count grows.** Six modes total. Mitigation: each mode is small and has its own handler function, mirroring the existing pattern. The growth is linear, not branching.

## Open questions resolved during brainstorming

- Q1 — Scope-switch semantics: chose **A+B** (scope-aware toggle plus implicit move via staged state in one modal).
- Q2 — Remove target: **A** (chord-managed only).
- Q3 — Install semantics: **add-and-install** via free-form prompt (npm-install pattern), drift handled as a separate per-item `r` action.
- Q4 — Scope modal UX: **single modal** with staged toggles + Apply, not dual keys.
- Q5 — Slow ops UX: **drop to inline output** (`lazygit` pattern).
