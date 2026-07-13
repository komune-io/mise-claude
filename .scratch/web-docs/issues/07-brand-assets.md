# Brand assets — favicon, OG image, wordmark lockups

Label: `wayfinder:task`
Type: task (AFK)
Status: Closed — resolved
Blocked-by: none (02 visual direction resolved)
Parent map: [Web docs + landing page for chord](../MAP.md)

## Question

Derive the fixed brand assets from the locked visual direction (ticket 02), so the landing (06) and docs (05) can reference them:

- **Favicon** — the sound-ring sun from `assets/logo.svg`, exported/optimized at favicon sizes (works on light + dark tabs).
- **Social / OG image** — a `1200×630` card in the Dusk direction: sun mark + `chord` wordmark + tagline over the illustrated scene.
- **Wordmark lockups** — sun + `chord` (primary) and sun + `rytmyk` (family), horizontal, as reusable SVG.

Source of truth is ticket 02's resolution (palette, type, mark). Produced assets get linked here and consumed by 05/06.

## Resolution

Brand assets produced under `site/brand/` + wired into both surfaces.

**Reusable brand SVGs** (`site/brand/`)
- `mark.svg` — canonical sound-ring sun.
- `lockup-chord.svg` — sun + `chord.` horizontal lockup (wordmark = `currentColor`, defaults to dusk ink; verified rendering).
- `lockup-rytmyk.svg` — sun + `rytmyk` family lockup.

**Raster assets** (rendered from HTML sources via headless Chrome, kept for regen)
- `site/landing/og-image.png` — 1200×630 social card (Dusk scene + wordmark + tagline). Source: `site/brand/og-image.src.html`.
- `site/landing/apple-touch-icon.png` — 180×180 opaque tile. Source: `site/brand/touch-icon.src.html`.
- Favicons: `favicon.svg` already ships on landing + docs (interim from earlier tickets; kept).

**Wired**
- Landing `index.html` — apple-touch-icon + full Open Graph/Twitter meta (absolute `chord.rytmyk.ai` URLs; hosting is the separate step).
- Docs `theme/head.hbs` — OG/Twitter meta injected into every page (absolute prod URLs). `docs:build` green.

Note: PNG social/touch assets use absolute prod URLs, so they resolve once hosted (step 2), not on localhost; the SVG favicon works locally now.
