# Docs information architecture (page tree + per-page outline)

Label: `wayfinder:grilling`
Type: grilling (HITL)
Status: Closed — resolved
Blocked-by: none
Parent map: [Web docs + landing page for chord](../MAP.md)

## Question

Decide the essentials docs sitemap and what each page covers. Target ~4–6 pages:

- Getting Started (install via mise + cargo, first `chord install`).
- `chord.toml` reference (every declarable: MCP servers, skills, plugins, CLI tools).
- CLI command reference (install, inspect, clean, …).

Grill to confirm the exact page list, order, and a per-page outline. Reuse existing `README.md` / `docs/` content where it fits. Deeper pages stay Out of scope (docs v2).

Resolution records the final `SUMMARY.md` structure and per-page outlines that ticket 05 writes against.

## Resolution

**Reframe (decided in the grill):** the docs site is a **usage manual**, not an onboarding guide. Installing the binary (`cargo install rytmyk-chord`) is developer setup → stays in the repo **README**, not the docs. mise is now an **edge convenience**, not the core path.

**Final IA** — `SUMMARY.md` written (`site/docs/src/`), 5 pages, stubs created with per-page outlines, `docs:build` green:

1. **Introduction** (`introduction.md`) — what chord is · the `chord.toml → chord install` model · tiny quickstart (declare → install → inspect) · how the docs are organized. Notes that binary install lives in the README.
2. **chord.toml reference** (`chord-toml.md`) — the four tables `[mcp] [skills] [plugins] [cli]`, each a flat `name = "version"` map; unknown keys rejected (`deny_unknown_fields`); version-string resolution; annotated example. *(Facts pulled from `src/core/config.rs`.)*
3. **Commands** (`commands.md`) — **single page**, one section per verb: `install · inspect · list · add · remove · update · diff · clean · migrate` (9 verbs, from `src/shell/cli.rs`).
4. **TUI** (`tui.md`) — `chord inspect --tui`: what it shows, navigation, relation to plain `inspect`, screenshots.
5. **mise plugin** (`mise-plugin.md`) — **usage + how it works**: add plugin / `chord = "latest"` / auto-bootstrap, plus a short note on the three Lua backend hooks (list-versions / install / exec-env).

**Bug caught & fixed:** `chord.toml` section is `[cli]`, not `[tools]` — corrected in the landing (`site/landing/index.html`) and prototype.

**Landing follow-up folded in:** flipped the landing Install section to **cargo primary** ("recommended"), mise demoted to "Via mise plugin (optional)". Applied to `site/landing/index.html` + prototype.

**Still open (not this ticket):** the landing's "mise-native" *feature card* still frames mise as a headline feature — worth revisiting given mise-at-edge, but left as a judgment call.
