# Build docs content into mdBook + solarpunk theme

Label: `wayfinder:task`
Type: task (AFK)
Status: Closed — resolved
Blocked-by: 01 (scaffold), 02 (visual direction), 03 (docs IA)
Parent map: [Web docs + landing page for chord](../MAP.md)

## Question

Fill the mdBook scaffold with the essentials pages from ticket 03's outline, and skin it to the ticket 02 solarpunk direction (custom CSS / theme).

- Write each page (Getting Started, `chord.toml` reference, CLI reference) per the locked IA.
- Apply the solarpunk palette/type to the mdBook theme.
- Verify `mise run docs:dev` serves the finished docs on localhost.

Depends on all three prerequisites: the shell (01), the look (02), and the content plan (03).

## Resolution

All 5 pages written with **real content** (sourced from `README.md`, `src/core/config.rs`, `src/shell/cli.rs` — not invented), and mdBook skinned to the solarpunk direction. `docs:build` green; both themes verified in Chrome.

**Content** (`site/docs/src/`)
- `introduction.md` — what chord is · the declare→install→inspect model · quickstart · roadmap · "binary install lives in the README".
- `chord-toml.md` — the four tables (`[mcp] [skills] [plugins] [cli]`), version strings, MCP aliases table, `.mcp-config.toml` extra config, skills.sh / plugin-marketplace slug forms.
- `commands.md` — all 9 verbs with real synopsis + flags + examples (`clean --all` destructive warning included).
- `tui.md` — `chord inspect --tui` (screenshots TODO).
- `mise-plugin.md` — usage + the three Lua hooks.

**Theme** (`site/docs/`)
- `theme/solarpunk.css` — overrides mdBook vars: dark `navy` → Dusk (`#191327`, sun-orange links, sun-underline H1), light → Warm sand (`#f3e7dd`, terracotta). Neutral-grotesque font. Per-theme inline-code pills; terminal code blocks kept dark in both modes (deliberate).
- `theme/favicon.svg` — the sun mark.
- `book.toml` — `additional-css`, `default-theme = navy`, `preferred-dark-theme = navy`.

**mdBook 0.5 quirks handled** — dropped `git-repository-icon` (0.5's FA parser rejected it; the default GitHub icon works); assets are content-hashed (`theme/solarpunk-<hash>.css`).

**Verified** — `mise run docs:build` writes all pages; served on :8091, full-page screenshots of Commands + chord.toml reference confirm Dusk **and** Warm-sand render correctly.

Note: the repo **README** still uses the old `komune-io/mise-claude` URL and "mise recommended" framing — updating it is repo housekeeping outside this map.
