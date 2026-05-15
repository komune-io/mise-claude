# chord

`chord` is a declarative manager for an agent's tool environment: a project declares the MCP servers, CLI tools, skills, and Claude Code plugins it depends on in `chord.toml`, and `chord` makes those declarations real on disk.

## Language

### Install verbs (distinct operations, not synonyms)

**Bootstrap**:
Place the `chord` binary on the user's `PATH`. Performed once by the mise plugin's `cargo install rytmyk-chord`.
_Avoid_: setup, init, "install chord itself".

**Install (all)**:
Read `chord.toml`, resolve a Plan against the Lockfile, and bring every declared entry into the state it requests. The headline `chord install` CLI verb. Idempotent.
_Avoid_: sync, reconcile (reserved for the TUI batch action), "run the installer".

**Install (one)**:
Same as Install (all) but constrained to a single entry by name. Used for drift fixes (`r` key in the TUI) and as the second half of `chord add`.
_Avoid_: install-one (cargo-style hyphenation), per-tool install.

**Fetch**:
The subprocess work that actually materializes one entry — `npm install`, `claude plugin install`, or `npx skills add`. Performed inside `Installer::install` trait impls.
_Avoid_: install (overloaded), download, sync.

**Add**:
Bring a brand-new entry under `chord` management: write the line into `chord.toml`, then Install (one). The user-facing verb behind `chord add` and the TUI's `a` key.
_Avoid_: register, declare-and-install, install-new.

### Umbrella nouns

**Tool**:
An installable that `chord` manages — one of four kinds: MCP server, CLI binary, Skill, or Plugin. The umbrella term for resolver, lockfile, and `Installer` trait code. Distinct from a "mise tool" (a binary mise installs on PATH) and from an "MCP tool" (a function an LLM can call).
_Avoid_: package (npm-specific), dependency (implies transitivity), item.

**Entry**:
A row in `chord.toml`. Usually declares one Tool; the section header (`[mcp]` / `[cli]` / `[skills]` / `[plugins]`) tells you which kind. The one exception is the Wildcard Skill entry — a 2-segment `[skills]` row that declares a whole Skill repo and expands to many Tools at install time.
_Avoid_: declaration, line, key.

### Skill nouns

**Skill entry**:
A 3-segment row in `[skills]` of `chord.toml` declaring one specific Skill — `"<owner>/<repo>/<skill-name>" = "<version>"`. Installs that one Skill via `npx skills add <owner>/<repo> --skill <skill-name>`.
_Avoid_: skill declaration, single skill, named skill.

**Wildcard Skill entry**:
A 2-segment row in `[skills]` of `chord.toml` — `"<owner>/<repo>" = "<version>"`. Means "install every Skill the repo exposes" and runs `npx skills add <owner>/<repo> --skill '*'`. One row, many installed Skills. Distinct in shape and cardinality from a Skill entry.
_Avoid_: bulk skill entry, glob skill entry, skill repo entry.

**Skill repo**:
A GitHub repository that exposes one or more Skills, installable via `npx skills add <owner>/<repo>`. Both Skill entries and Wildcard Skill entries point at a Skill repo. Each installed Skill carries the identity of its Skill repo so the reconciler can validate Wildcard Skill entries against actual disk state.
_Avoid_: skill source (too generic), skill marketplace (Skills don't have a marketplace.json), skills package.

### Plugin nouns

**Plugin**:
A Claude Code plugin — the thing materialized into `~/.claude/plugins/cache/<marketplace>/<plugin>/` by `PluginInstaller`. Bare "plugin" always means this in chord's domain.
_Avoid_: Claude plugin (verbose), claude-code plugin, claude-plugin.

**Mise backend plugin**:
The Lua hooks at the repo root (`metadata.lua`, `hooks/*.lua`) that mise loads. Always qualified — never write "plugin" bare to mean this.
_Avoid_: mise plugin (ambiguous with `mise plugin link` CLI verb), chord plugin.

**Plugin entry**:
A row in the `[plugins]` table of `chord.toml`. Declares one Plugin to install. Format `"<owner>/<repo>/<plugin>@<marketplace>" = "<version>"`.
_Avoid_: plugin declaration, plugin spec.

**Marketplace**:
An upstream repo's `.claude-plugin/marketplace.json` that lists one or more Plugins. Required by `chord install` for `[plugins]` entries — without it, `claude plugin marketplace add` has nothing to register.
_Avoid_: plugin source, plugin index, registry (already a chord-internal type).

### Operation vocabulary (verbs vs nouns)

**Inspect** (verb):
The act of looking at the current Claude Code environment. The `chord inspect` CLI verb (and `--tui` for the interactive view). Reads files; never writes.
_Avoid_: audit (as a verb — see below).

**Audit** (noun):
The structured output of an Inspect run — an `AuditReport` of `AuditEntry` rows, one per discovered or declared item. Each row carries Tool kind, Scope, Managed/Manual, Drift, Enabled/Disabled, Shadowed.
_Avoid_: inspection (as a noun), report (too generic).

> *Convention*: "Run `chord inspect`; it produces an Audit." Two words, two roles — verb for the action, noun for the artifact.

### State vocabulary

**Managed**:
A Tool that is declared in `chord.toml`. Chord owns its lifecycle. Code: `Management::Managed`.

**Manual**:
A Tool present on disk but not declared in `chord.toml`. Chord does not own it.
_Avoid_: unmanaged, orphan, foreign.

**Drift**:
A Managed Tool whose actual state diverges from its declaration. Today the only divergence chord detects is "missing on disk"; the concept is broader (e.g. wrong version installed would also be drift if chord checked for it).
_Avoid_: missing (too narrow), broken.

**Enabled / Disabled**:
Plugin-only. Comes from the presence of a key in `enabledPlugins` inside `settings.json` (Project or Global). Orthogonal to Managed/Manual — a Plugin can be Managed-and-Disabled, or Manual-and-Enabled. Not a chord concept; chord just reflects what Claude Code's settings file says.

**Shadowed**:
A Plugin appears in both Project and Global `settings.json`. The Project entry wins at runtime; the Global one is *shadowed*. Code today: `AuditEntry.overridden_by: Some("project")` — flagged for rename to `shadowed_by` (see below).

### Scope

**Scope** (Project / Global):
Where a Plugin (or skill / MCP server / hook) physically lives. Project scope = `<project>/.claude/`. Global scope = `~/.claude/`. The TUI scope picker toggles a Plugin's enabled-state independently in each scope. `chord.toml` is always Project-scoped — there is no Global `chord.toml` today.

### Resolver nouns

**Resolver**:
The pure function `resolver::resolve(config, lockfile, is_installed) -> Plan`. Decides what each Entry needs: Install, Upgrade, or Skip. No side effects.

**Plan**:
The Resolver's output — a list of PlannedActions, one per Entry. Consumed by `operations::install::install_all` / `install_one`.

**PlannedAction**:
One row of a Plan. Carries `name`, `package` (resolved via Alias), `version`, `tool_type`, and the assigned Action.

**Action**:
The verb the Resolver assigned: `Install` (not installed yet), `Upgrade` (locked at a different version than declared), or `Skip` (already at the right version).
_Avoid_: operation (reserved for `operations::*`), step.

### Lockfile nouns

**Lockfile** (`chord.lock`):
Records what was actually installed. One row per Entry, grouped by Tool kind. Written after every successful Install (all) or Install (one). Authoritative; the Resolver reads it to detect Upgrade. Chord owns this file end-to-end.
_Avoid_: lock, manifest.

**Locked Tool**:
A row in the Lockfile — `{ package, version, integrity, resolved_at }`. Captures the state of one materialized Entry.

**Skills lockfile** (`skills-lock.json`):
A project-scoped JSON file written by the `npx skills` CLI, recording each installed Skill's upstream Skill repo, ref, and content hash. Chord *reads* this file (during the scan phase of Audit) to attach the Skill repo identity to each discovered Skill; chord never writes it. Distinct from the chord-owned Lockfile.
_Avoid_: skills lock, skill manifest, lockfile (without qualifier).

### Hook nouns

**Hook**:
A Claude Code hook — a shell command registered in `settings.json` under a lifecycle event (`PreToolUse`, `PostToolUse`, `Stop`, etc.). Discovered by the Audit; rendered under a "Hooks" section in the TUI tree. Configured by users.
_Avoid_: lifecycle hook (verbose), shell hook.

**Mise backend hook**:
A Lua function in `hooks/*.lua` that chord's Mise backend plugin implements to satisfy mise's plugin protocol (`BackendInstall`, `BackendExecEnv`, `BackendListVersions`). Always qualified — never write "hook" bare to mean this.
_Avoid_: lua hook, plugin hook, mise hook.

### Registry nouns

**Alias**:
A friendly short name mapping to a canonical npm package name in `Registry::aliases`. Example: `context7` → `@upstash/context7-mcp`. Lets `chord.toml` declarations stay readable.

**Override** (install-time):
A per-package customization in `Registry::overrides` (`bin_name`, `post_install`, `extra_deps`). Tweaks how `Installer::install` materializes a specific package. Distinct from the scope shadowing above — that's why the audit field is being renamed.
_Avoid_: scope override (use Shadowed for that).

## Relationships

- An **Entry** declares one **Tool** — except a **Wildcard Skill entry**, which declares a **Skill repo** that expands to many Tools.
- An **Install** (all) reads every Entry in `chord.toml`, builds a Plan, and Fetches each non-skipped Tool.
- An **Add** writes an Entry, then Installs (one) that Tool.
- A **Managed** Tool that is missing on disk is in **Drift**. The `r` key in the TUI runs Install (one) to clear the Drift.
- A **Plugin** can be **Enabled** in Project scope, Global scope, or both. When both, the Global entry is **Shadowed** by the Project entry.
- A **Marketplace** lists one or more **Plugins**; `chord` requires it upstream to materialize a `[plugins]` Entry.
- A **Skill repo** exposes one or more **Skills**, each installable individually via a **Skill entry** or in bulk via a **Wildcard Skill entry**.
- The **Skills lockfile** records each installed Skill's source **Skill repo**; chord reads it during Audit to validate Wildcard Skill entries against actual disk state.

## Example dialogue

> **User:** "I added `context7` to `chord.toml` but `chord install` says it's already installed. Why?"
> **Maintainer:** "Because the Lockfile has it. The Resolver returned `Skip` for that Entry. If you want to force re-Fetch, delete the Locked Tool row from `chord.lock` and run `chord install` again."
> **User:** "And in the TUI, what's the ⚠ I'm seeing on `superpowers`?"
> **Maintainer:** "That's a Drift — the Entry is Managed but missing on disk. Press `r` to Install (one) just that Plugin, or `R` to Install (all)."
> **User:** "Hit `r`. It says installed. But it still shows up disabled in the tree."
> **Maintainer:** "Drift is gone — it's on disk now. But it's still Disabled at both Scopes because no `enabledPlugins` entry exists. Press `e` and toggle Project on."
> **User:** "If I had it enabled Globally too, what happens?"
> **Maintainer:** "The Global one is Shadowed by Project. Both files still hold the entry; only Project wins at runtime."

## Flagged ambiguities

- "install" was used for at least five distinct operations (Bootstrap, Install all, Install one, Fetch, Add) — resolved by the verbs above.
- "plugin" was used for four distinct things (Claude Code plugin, mise backend plugin, chord.toml entry, marketplace) — resolved: bare "Plugin" = Claude Code plugin; "Mise backend plugin" is always qualified.
- "tool" was used for both chord's umbrella concept and mise's binary-on-PATH concept — resolved: bare "Tool" = chord's umbrella; "mise tool" always qualified.
- "override" was used for both `Registry::ToolOverride` (install-time customization) and `AuditEntry.overridden_by` (scope precedence) — resolved: "Override" means only the Registry concept. The scope-precedence field will be renamed `overridden_by` → `shadowed_by`, and prose uses "Shadowed".
- A chord project can hold *two* lockfiles side by side: `chord.lock` (chord owns) and `skills-lock.json` (the `npx skills` CLI owns; chord reads). Resolved: bare "Lockfile" = `chord.lock`; the other is always "Skills lockfile". The `.gitignore` recommends excluding the latter since `chord.lock` is the authoritative source for chord-managed state.
- `[skills]` rows have two distinct shapes (3-segment "Skill entry" and 2-segment "Wildcard Skill entry") with different cardinalities (1 Tool vs N Tools per row). Resolved: glossary treats them as distinct terms, not variants of one term.
