# Landing content & narrative (what/why → install CTA → docs link)

Label: `wayfinder:prototype`
Type: prototype (HITL)
Status: Closed — resolved
Blocked-by: 02 (visual direction) — resolved
Parent map: [Web docs + landing page for chord](../MAP.md)

## Question

Decide the landing page's sections and copy. It does two jobs (confirmed): explain the concept **and** drive install.

- Narrative: the problem (scattered agent tooling) → the declarative solution.
- A `chord.toml` teaser + copy-paste install (mise + cargo).
- Primary CTA = install; secondary = link to docs.chord.rytmyk.ai.
- Section order / wireframe (a rough prototype to react to).

Blocked by 02 so the copy prototype uses the locked palette/type/wordmark.

Resolution links the content prototype / wireframe and records the final section list + copy that ticket 06 builds.

## Resolution

Landing settled via an iterated UI prototype (dune scene reskin → three lean hero layouts → chosen + verified in Chrome, both themes). Full working prototype: `.scratch/web-docs/prototypes/landing.html`.

**Hero layout — "High horizon"** (chosen over two-column and minimal-poster): copy sits entirely in the sky; a **thin two-band dune** runs along the very bottom; the sound-ring sun sets low-right. No vegetation (plants were rejected — looked childish and the earlier version had text bleeding onto the sand). This layout *structurally* keeps all copy off the dune.

**Two modes** (per ticket 02): Dusk (default, violet night) / Warm-sand (light, cream daytime desert), nav toggle. Spacing on one scale (`8/16/24/40/64/104`).

**Section order & content (does both jobs — explain + drive install):**
1. **Hero** — eyebrow "rytmyk toolchain" · wordmark `chord.` · tagline "Every agent tool, *in harmony.*" · one-line lede · CTAs **Install chord →** (primary) + **Read the docs**.
2. **Why** (the problem) — "Agent tooling drifts. chord keeps it in tune." → declarative pitch, + 4 feature cards: *One file, everything · Reproducible · mise-native · Audit & clean*.
3. **chord.toml** — the four sections (MCP servers / skills / plugins / CLI tools) explained beside a real snippet.
4. **Install** — mise (recommended) + cargo, side by side.
5. **Footer** — docs.chord.rytmyk.ai · GitHub · crates.io · rytmyk.ai · MIT.

**Primary CTA** = install; **secondary** = docs (docs.chord.rytmyk.ai). Copy uses the `rytmyk-ai/chord` repo + new URLs.

**Asset:** `.scratch/web-docs/prototypes/landing.html` (the winning prototype — ticket 06 folds it into the real `site/landing/`). Superseded hero explorations: `hero-variants.html`, `hero-context.html`.
