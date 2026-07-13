# Web docs + landing page for chord

Label: `wayfinder:map`
Tracker: local markdown — tickets live in `.scratch/web-docs/issues/`

## Destination

A landing page (**chord.rytmyk.ai**) and an essentials docs site (**docs.chord.rytmyk.ai**) for `chord`, built with a fresh solarpunk visual identity — mdBook for the docs, a hand-rolled `index.html` for the landing — all running on **localhost**. Reaching the end means both surfaces build and serve locally via mise tasks. Hosting/deployment is a separate second step and is out of scope.

## Notes

- **Visual direction:** fresh, minimal, **solarpunk**. Locked in detail by ticket 02 — every visual ticket honors it.
- **Execution in the map (Livré):** this map *builds*, it doesn't only plan. Task tickets that produce the actual site are in scope (overrides wayfinder's plan-only default).
- **Stack:** mdBook (Rust-native, no Node) for docs; hand-rolled static HTML/CSS for the landing. Both served on localhost via mise dev tasks.
- **Location:** in this repo under `site/` (`site/landing/`, `site/docs/`). Versioned with the CLI.
- **Repo rename in flight:** komune-io/mise-claude → **rytmyk-ai/chord**. Use the new name/URLs in copy and links.
- **Tracker ops:** no native blocking in local markdown → tickets declare `Blocked-by:` in their body. A ticket is claimed via its `Status:` line. Frontier = open + unblocked + unclaimed.

## Decisions so far

<!-- one line per closed ticket: enough to judge relevance, then open the ticket for detail -->

- [Lock solarpunk visual direction](issues/02-visual-direction.md) — musical-solarpunk "Dusk"-led, two-mode (dark `#0e1620` / warm-sand `#f3e7dd`); mark = sound-ring sun (`assets/logo.svg`); accents sun `#ff9d5c` + lime `#b6ff5c`; neutral grotesque type (Inter / Helvetica Neue); hand-drawn illustrated hero scene.
- [Landing content & narrative](issues/04-landing-content.md) — "High horizon" hero (copy in sky, thin dune, no plants, sun sets low-right), two-mode; sections Hero→Why(4 cards)→chord.toml→Install→Footer; CTA install + docs link. Prototype: `prototypes/landing.html`.
- [Scaffold site/](issues/01-scaffold-site.md) — `site/docs` (mdBook 0.5.4 via mise tool) + `site/landing` (static) in-repo; tasks `docs:dev` (:3000), `docs:build`, `landing:dev` (:8080); `/docs/` gitignore anchored so `site/docs` is tracked, `site/docs/book/` ignored. Both verified serving.
- [Build landing page](issues/06-build-landing.md) — real landing at `site/landing/` (`index.html` + `style.css` + `favicon.svg`), folded from the prototype; two-mode, real links; verified serving on :8080, matches approved design.
- [Docs information architecture](issues/03-docs-ia.md) — docs = **usage manual** (binary install → README, mise = edge). 5 pages: Introduction · chord.toml reference (`[mcp] [skills] [plugins] [cli]`) · Commands (single page, 9 verbs) · TUI · mise plugin. `SUMMARY.md` + stubs written, `docs:build` green. Fixed `[tools]`→`[cli]` bug + flipped landing to cargo-primary.
- [Build docs content + theme](issues/05-build-docs.md) — 5 pages written with real content (from README + `config.rs`/`cli.rs`); mdBook skinned solarpunk (`theme/solarpunk.css`: navy=Dusk, light=Warm-sand; sun favicon). Both themes verified in Chrome. Docs half complete.
- [Brand assets](issues/07-brand-assets.md) — `site/brand/` mark + chord/rytmyk lockups (SVG); 1200×630 OG card + 180×180 apple-touch (rasterized via Chrome); OG/Twitter meta wired into landing + docs (`theme/head.hbs`).

## Destination reached ✅

All 7 tickets closed. Landing (`site/landing/`) + essentials docs (`site/docs/`) build and serve on localhost (`mise run landing:dev` :8080 · `mise run docs:dev` :3000) in the musical-solarpunk direction, two-mode. Hosting/deploy remains the separate second step (out of scope). The map is complete.

## Not yet specified

<!-- cross-link fog resolved: dev ports are 3000 (docs) / 8080 (landing); prod links target docs.chord.rytmyk.ai, handled in the 05/06 builds -->
- (nothing outstanding — the way to the destination is now clear; remaining tickets are all specifiable)

## Out of scope

- Hosting / deployment of either surface (the separate second step).
- Docs v2 — deeper pages ruled past this destination: mise plugin internals, the 14-samples walkthrough, architecture / core-shell seam, contributing guide.
