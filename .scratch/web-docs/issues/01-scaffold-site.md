# Scaffold `site/` — mdBook init + landing skeleton + mise dev tasks

Label: `wayfinder:task`
Type: task (AFK)
Status: Closed — resolved
Blocked-by: none
Parent map: [Web docs + landing page for chord](../MAP.md)

## Question

Stand up the empty shell both surfaces build into, running on localhost:

- Create `site/docs/` as an initialized mdBook (`book.toml`, `src/SUMMARY.md`, placeholder page) — no real content yet.
- Create `site/landing/` with a placeholder `index.html` + `style.css`.
- Add mise tasks so `mise run docs:dev` serves the mdBook and `mise run landing:dev` serves the landing on localhost (record the ports chosen).
- Confirm both serve locally.

Resolution records: the chosen dev ports, task names, and any mdBook version/config facts tickets 05/06 depend on.

## Resolution

Scaffold stood up in-repo under `site/`, both surfaces serve on localhost.

**Layout**
- `site/docs/` — mdBook: `book.toml`, `src/SUMMARY.md`, `src/introduction.md` (placeholder). Real content = ticket 05, IA = ticket 03.
- `site/landing/` — `index.html` + `style.css` (placeholder with the sun mark). Real page = ticket 06 (folds in `.scratch/web-docs/prototypes/landing.html`).

**Toolchain** — `mdbook = "latest"` added to `[tools]` in `mise.toml`; `mise install` provisions it as a **prebuilt binary** (resolved **mdbook v0.5.4**, no cargo compile).

**mise dev tasks** (facts 05/06 depend on):
- `mise run docs:dev` → `mdbook serve site/docs --open` on **http://localhost:3000**
- `mise run docs:build` → static output to `site/docs/book/`
- `mise run landing:dev` → `python3 -m http.server 8080 --directory site/landing` on **http://localhost:8080** (no extra dep)

**git** — `.gitignore` `docs/` rule was unanchored and would have swallowed `site/docs/`; anchored it to `/docs/` (root design-docs only) and added `site/docs/book/` (generated output). Verified: `site/docs/src` tracked, `site/docs/book` ignored.

**Verified** — `docs:build` writes `site/docs/book/index.html`; `landing:dev` returns HTTP 200 with the right `<title>`.
