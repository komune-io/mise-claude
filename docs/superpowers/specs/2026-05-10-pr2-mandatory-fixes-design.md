# PR #2 Mandatory Fix-Pack — Design

**Date:** 2026-05-10
**Branch:** `feat/rename-to-chord` (PR #2)
**Scope:** Blocker and high-risk findings from caveman code review of PR #2. Out-of-scope items deferred as separate issues.

## Goal

Make PR #2 safe to merge by addressing eight findings that either break behavior, leak the old project name, or weaken the release pipeline. Quality / nit findings are explicitly deferred.

## Non-goals

- Repository rename (`mise-claude` → `chord` on GitHub).
- Adding feature flags or migration shims (project is pre-1.0; hard cuts are fine).
- Refactors not driven by the eight findings.
- Running the full e2e Docker suite locally — CI covers it.

## Fixes

### 1. Resolver `latest` thrash — `src/resolver.rs:65-70`

**Symptom:** When `chord.toml` declares `tool = "latest"`, the lockfile stores the concrete resolved version (e.g. `"1.2.3"`). On the next resolve, `entry.version != "latest"` → `Action::Upgrade`. Combined with `backend_exec_env.lua`'s shell-entry trigger, npm reinstalls on every new shell.

**Fix:** Treat `latest` and `*` as wildcard markers. The action ladder becomes:

```
if locked.is_none()                 → Install
else if requested ∈ {latest, *}     → if is_installed: Skip; else Install
else if locked.version != requested → Upgrade
else if !is_installed               → Install
else                                 → Skip
```

A future `chord install --refresh` flag can force re-resolution; not in this PR.

**Tests:** add cases to `tests/unit/resolver_test.rs`:
- `"latest"` requested + locked concrete + installed → `Skip`
- `"latest"` requested + locked concrete + not installed → `Install`
- `"latest"` requested + no lockfile entry → `Install`
- `"*"` behaves the same as `"latest"`
- Concrete version mismatch still produces `Upgrade` (regression guard)

### 2. Env var rename — `src/main.rs:320-327`

**Symptom:** `packages_dir()` still reads `CLAUDE_ENV_HOME`. Stale name post-rename.

**Fix:** Rename env var to `CHORD_HOME`. No fallback to the old name (pre-1.0, no external users).

**Verification:** `rg "CLAUDE_ENV_HOME"` in `src/`, `tests/`, `sample/`, `hooks/`, `e2e/`, `README.md`, `CLAUDE.md` after edit → must return zero results.

### 3. `Cargo.toml` repository URL — `Cargo.toml:7`

**Symptom:** `repository = "https://github.com/rytmyk/chord"` points to a non-existent GitHub path. crates.io metadata would link nowhere.

**Fix:** Set to `https://github.com/komune-io/mise-claude` (the current upstream truth). A future repo rename can update this in lockstep.

### 4. User-Agent string drift — `hooks/backend_install.lua:35`, `hooks/backend_list_versions.lua:31`

**Symptom:** Both hooks send `User-Agent: "rytmyk-chord/2.0"`, but `Cargo.toml` is at `0.1.0`. Version embedded in UA will rot.

**Fix:** Replace with a static, version-free identifier: `"rytmyk-chord (mise-plugin)"`. Applied to both hooks for consistency with the existing "keep in sync" duplication policy.

### 5. `publish.yml` hardening — `.github/workflows/publish.yml`

**Symptom:** Workflow runs `cargo test` (no `--locked`) before `cargo publish`. No dry-run gate, no clippy gate. A drifted `Cargo.lock` or a manifest error only surfaces during the real publish.

**Fix:** Replace the single `cargo test` step with three steps:
1. `cargo test --locked`
2. `cargo clippy --all-targets --locked -- -D warnings`
3. `cargo publish --dry-run --locked`

`cargo publish` (the existing real publish step) stays last and unchanged.

### 6. `release.yml` tag ordering — `.github/workflows/release.yml:29-32`

**Symptom:** Workflow does `git tag` → `git push origin v$VERSION` → `gh release create`. If the release step fails (token, network, duplicate), the tag is already pushed and orphans on the remote.

**Fix:** Drop the manual `git tag` + `git push origin` lines. Use `gh release create v$VERSION --target $GITHUB_SHA --generate-notes`. `gh` creates the tag at the target SHA as part of the release; if release creation fails, no tag is pushed.

**Caveat:** the workflow already runs from `workflow_dispatch` on the chosen ref, so `$GITHUB_SHA` is the dispatched commit. Document this in the workflow comment.

### 7. e2e Dockerfile claude-code pin — `e2e/Dockerfile:6`

**Symptom:** `npm install -g @anthropic-ai/claude-code` is unpinned. Upstream breakage silently turns CI red.

**Fix:** Pin to the current latest stable version. Resolution step: run `npm view @anthropic-ai/claude-code version` at fix time and use that exact string. Renovate (`renovate.json` already covers npm) can keep it current.

### 8. `detect_binary` determinism — `src/installer/mcp.rs:46-66`

**Symptom:** `read_dir` ordering is filesystem-dependent. If an npm package ships more than one binary in `node_modules/.bin`, the picked one differs across runs / hosts.

**Fix:** Collect entries into a `Vec`, sort lexically by file name (ignoring leading `.` files as before), pick the first. Same selection on every host.

**Tests:** existing tests cover single-binary case. Add one fixture-driven test with two non-dot entries (`a-bin`, `b-bin`) asserting `a-bin` wins.

## Build sequence

The order keeps the test suite green between steps:

1. Resolver fix + new unit tests
2. `CLAUDE_ENV_HOME` → `CHORD_HOME` (grep verify)
3. `Cargo.toml` repository URL
4. UA strings in both hooks
5. `detect_binary` sort + new test
6. `publish.yml` rewrite
7. `release.yml` rewrite
8. `e2e/Dockerfile` pin

After each batch: `mise run test`, then `mise run lint && mise run fmt` at the end.

## Verification

- `mise run test` — green
- `mise run lint` — green (clippy `-D warnings`)
- `mise run fmt` — no diff
- `rg "CLAUDE_ENV_HOME|rytmyk-chord/2.0|rytmyk/chord"` — zero hits
- Manual: `chord install` twice in a row on a `chord.toml` with `mcp.context7 = "latest"` → second run reports `skip`, not `installed`. (No npm reinstall.)

Skipping `mise run e2e` locally; CI covers it.

## Risk

- **Resolver edge:** user replaces a concrete pin with `"latest"`. Locked version is older than current upstream `latest`. Under the new logic this returns `Skip`, not `Upgrade`. Acceptable — that is the explicit `latest` contract until `--refresh` exists.
- **Release atomicity:** `gh release create --target $SHA` succeeds, then a later step fails. Release exists tagged; no rollback. Mitigation: keep the workflow short after the create call (it already is).
- **claude-code pin staleness:** the pinned version drifts from upstream. Renovate is configured to bump; if it stalls, CI keeps passing on the old version which is the safe failure mode.

## Out of scope (file issues post-merge)

- `post_install` shell quoting in `src/installer/cli_tool.rs`
- `chord.toml` path discovery (walk up vs. cwd-only)
- `e2e/run.sh` errexit + `sed` test.sh rewriting
- `backend_exec_env.lua` fallback paths and silent-failure UX
- `lockfile.rs` IO-vs-parse error type
- `migrate` fail-fast reordering
- `resolver` `Registry::default()` rebuild on every call
- Archiving `docs/superpowers/` history out of main

## Test plan summary

| Area | Change | Coverage |
|---|---|---|
| Resolver `latest` | New unit cases | `tests/unit/resolver_test.rs` |
| `detect_binary` | New unit case | `tests/integration/install_test.rs` or new |
| Env rename | Grep verify | No test (config plumbing) |
| Cargo.toml URL | Visual diff | None |
| UA strings | Visual diff | None |
| publish/release workflows | Visual diff + dry-run on CI | CI run on PR |
| Dockerfile pin | CI run | CI run |
