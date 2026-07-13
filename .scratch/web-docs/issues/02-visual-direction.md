# Lock solarpunk visual direction (palette, type, wordmark)

Label: `wayfinder:prototype`
Type: prototype (HITL)
Status: Closed — resolved
Blocked-by: none
Parent map: [Web docs + landing page for chord](../MAP.md)

## Question

Pin down the fresh, minimal, solarpunk visual identity both surfaces inherit. Produce a small concrete artifact to react to (a swatch/type/wordmark prototype), then lock:

- Color palette (accent + neutrals; light/dark stance).
- Typography (families, no external CDN unless decided otherwise).
- The `chord` wordmark / logo treatment.

Resolution links the prototype and records the locked tokens — these become the shared reference for tickets 04 (landing content), 05 (docs theme), 06 (landing build).

## Resolution

**Direction: musical-solarpunk, "Dusk"-led, two-mode.** Arrived at via a UI prototype (three rounds — colour-only and layout-only takes were rejected; a hand-authored illustrated SVG scene won).

**Logo / mark** — the rytmyk mark is a **sound-ring sun**: four radiating rings + a soft glow + a flat setting-sun core. Standalone asset committed at `.scratch/web-docs/assets/logo.svg`. `chord` is one tool under rytmyk → wordmark lockup = **sun + `chord`** (the same sun serves `rytmyk`).

**Modes** (brand is two-mode):
- Dark (primary) — "Dusk": sky gradient `#2a2f52 → #0e1620`.
- Light — "Warm sand": `#f3e7dd`.

**Palette**
- Setting-sun `#ff9d5c` (mark + primary accent, both modes)
- Glowing lime `#b6ff5c` (dark-mode accent) · terracotta `#c2603a` (light-mode accent)
- Hills / greens — dark: `#233f34 / #18342a / #0c211a`; light: `#cdd9b0 / #8bb072 / #4a7a4f`
- Ink — dark `#eafff0`, light `#2a2320`; sub-text dark `#9fbfa6`, light `#6a5c50`

**Typography** — single **neutral grotesque** for wordmark + body. Ship self-hosted **Inter**; system fallback `"Helvetica Neue", Arial, system-ui`. Code in `ui-monospace, Menlo`. (Serif was tried and rejected — grotesque holds up against the illustration.)

**Hero illustration** — hand-authored musical-solarpunk scene: sound-ring sun, three layered hills, note-plants (stalks blooming into noteheads), a central "chord tree" whose canopy is a stacked triad (the harmony metaphor), eighth-note birds. This scene is the landing hero (feeds ticket 06).

**Assets (linked, not pasted):**
- `.scratch/web-docs/assets/logo.svg` — the mark (deliverable)
- `.scratch/web-docs/prototypes/hero-context.html` — the illustrated hero, Dusk + warm-sand, in Helvetica Neue (the winning prototype)
- `.scratch/web-docs/prototypes/visual-direction.html` — the mood/scene exploration (primary source)
- `.scratch/web-docs/assets/logo-preview.html` — bg + font chooser (primary source)
