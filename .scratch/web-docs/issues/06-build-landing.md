# Build landing page HTML/CSS

Label: `wayfinder:task`
Type: task (AFK)
Status: Closed — resolved
Blocked-by: 01 (scaffold), 02 (visual direction), 04 (landing content)
Parent map: [Web docs + landing page for chord](../MAP.md)

## Question

Build the real landing `index.html` + CSS from ticket 04's content, in the ticket 02 solarpunk direction.

- Implement the confirmed sections/copy and the `chord.toml` teaser.
- Install CTA + secondary link to docs.chord.rytmyk.ai.
- Responsive; no external CDN unless 02 decided otherwise.
- Verify `mise run landing:dev` serves it on localhost.

Depends on the shell (01), the look (02), and the content (04).

## Resolution

Real landing built at `site/landing/`, folding in the ticket-04 prototype:
- `index.html` — semantic markup, inline SVG sun symbol, real links (`docs.chord.rytmyk.ai`, `github.com/rytmyk-ai/chord`, `crates.io/crates/rytmyk-chord`, `rytmyk.ai`), `<meta description>`, favicon link.
- `style.css` — all styles extracted from the prototype (was inline); two-mode via `body.dusk` / `body.sand`.
- `favicon.svg` — the sound-ring sun mark (interim; optimized favicon + OG image are ticket 07).
- Theme toggle is a small inline `<script>`.

**Hero** = "High horizon" (copy in sky, thin dune, no plants, sun low-right). Responsive; no external CDN (system-font stack, self-hosted Inter deferred).

**Verified** — served via `mise run landing:dev` on http://localhost:8080: `/`, `/style.css`, `/favicon.svg` all 200 with correct content-types; full-page screenshot matches the approved prototype in both themes.

Prototype primary source retained at `.scratch/web-docs/prototypes/landing.html`.
