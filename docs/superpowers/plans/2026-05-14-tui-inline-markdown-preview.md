# TUI Inline Markdown Preview Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show a styled markdown preview of the selected skill/command/agent file inline in the TUI Detail pane, beneath the metadata block. Reuse the same styled renderer in the existing fullscreen `v` overlay so both surfaces look consistent.

**Architecture:** A new `src/tui/markdown.rs` module wraps `pulldown-cmark` and converts source markdown into `Vec<ratatui::text::Line<'static>>`. App gains a `Focus` enum (`Tree`/`Preview`) and a `home_dir` field; a new `update_preview()` method is called from every selection-changing mutator, lazy-reading the underlying file (`std::fs::read_to_string`) and resetting scroll. UI splits the Detail pane vertically when content is available, auto-sizing the metadata block; border color is the focus indicator.

**Tech Stack:** Rust 2021, `ratatui 0.29`, `crossterm 0.28`, `pulldown-cmark 0.12` (new), `tempfile 3` for tests (already in dev-deps).

**Spec:** `docs/superpowers/specs/2026-05-14-tui-inline-markdown-preview-design.md`

---

## File Structure

**Create:**
- `src/tui/markdown.rs` — pure renderer, parses markdown → styled `Vec<Line>`. Unit tests co-located.

**Modify:**
- `Cargo.toml` — add `pulldown-cmark = { version = "0.12", default-features = false }`.
- `src/tui/mod.rs` — register `pub mod markdown;`, update `App::new` call to pass `home_dir`.
- `src/tui/app.rs` — add `Focus` enum, `focus` and `home_dir` fields, `update_preview()` method, call it from mutators, add `expand_tilde` helper.
- `src/tui/handler.rs` — route keys by focus, handle `Tab`/`Esc`/`PgUp`/`PgDn`, call `update_preview()` after `execute_toggle`, use shared `expand_tilde`.
- `src/tui/ui.rs` — split Detail when preview present, dynamic keybind hint, focus-colored borders, styled overlay.

Each file keeps a single responsibility: `markdown.rs` is the renderer, `app.rs` is state, `handler.rs` is input routing, `ui.rs` is drawing.

---

## Task 1: Add `pulldown-cmark` dependency

**Files:**
- Modify: `Cargo.toml`

- [ ] **Step 1: Add the dependency**

Edit `Cargo.toml`. In the `[dependencies]` section, after the `crossterm` line, add:

```toml
pulldown-cmark = { version = "0.12", default-features = false }
```

- [ ] **Step 2: Verify it resolves**

Run: `cargo check`
Expected: Resolves and compiles existing code with no errors. `Cargo.lock` updates.

- [ ] **Step 3: Commit**

```bash
git add Cargo.toml Cargo.lock
git commit -m "deps: add pulldown-cmark for TUI markdown rendering"
```

---

## Task 2: Renderer scaffold — paragraphs with inline styling

**Files:**
- Create: `src/tui/markdown.rs`
- Modify: `src/tui/mod.rs:1-5` (register module)

- [ ] **Step 1: Register the module**

Edit `src/tui/mod.rs`. Replace lines 1-5 with:

```rust
pub mod actions;
pub mod app;
pub mod handler;
pub mod markdown;
pub mod tree;
pub mod ui;
```

- [ ] **Step 2: Write failing tests**

Create `src/tui/markdown.rs` with:

```rust
//! Convert markdown source to styled ratatui Lines for the TUI preview pane.

use pulldown_cmark::{Event, Parser, Tag, TagEnd};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

/// Render markdown source into a sequence of styled lines.
///
/// Returns an empty Vec for empty input. Tables, blockquotes, images, and HTML
/// pass-through are silently dropped in this phase.
pub fn render(source: &str) -> Vec<Line<'static>> {
    let parser = Parser::new(source);
    let mut renderer = Renderer::default();
    for event in parser {
        renderer.handle(event);
    }
    renderer.finish()
}

#[derive(Default)]
struct Renderer {
    lines: Vec<Line<'static>>,
    current: Vec<Span<'static>>,
    style_stack: Vec<Style>,
}

impl Renderer {
    fn current_style(&self) -> Style {
        self.style_stack.last().copied().unwrap_or_default()
    }

    fn push_span(&mut self, text: String) {
        let style = self.current_style();
        self.current.push(Span::styled(text, style));
    }

    fn flush_line(&mut self) {
        let spans = std::mem::take(&mut self.current);
        self.lines.push(Line::from(spans));
    }

    fn handle(&mut self, event: Event<'_>) {
        match event {
            Event::Start(Tag::Paragraph) => {}
            Event::End(TagEnd::Paragraph) => {
                self.flush_line();
                self.lines.push(Line::from(""));
            }
            Event::Start(Tag::Strong) => {
                let style = self.current_style().add_modifier(Modifier::BOLD);
                self.style_stack.push(style);
            }
            Event::End(TagEnd::Strong) => {
                self.style_stack.pop();
            }
            Event::Start(Tag::Emphasis) => {
                let style = self.current_style().add_modifier(Modifier::ITALIC);
                self.style_stack.push(style);
            }
            Event::End(TagEnd::Emphasis) => {
                self.style_stack.pop();
            }
            Event::Text(t) => self.push_span(t.into_string()),
            Event::Code(t) => {
                let style = self.current_style().fg(Color::Yellow);
                self.current
                    .push(Span::styled(t.into_string(), style));
            }
            Event::SoftBreak | Event::HardBreak => self.flush_line(),
            _ => {}
        }
    }

    fn finish(mut self) -> Vec<Line<'static>> {
        if !self.current.is_empty() {
            self.flush_line();
        }
        // Drop trailing blank line for a tidier preview.
        if matches!(self.lines.last(), Some(line) if line.spans.is_empty()) {
            self.lines.pop();
        }
        self.lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn span_texts(line: &Line) -> Vec<&str> {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn render_empty_returns_empty_vec() {
        assert!(render("").is_empty());
    }

    #[test]
    fn render_plain_paragraph_preserves_text() {
        let lines = render("Hello world");
        assert_eq!(lines.len(), 1);
        assert_eq!(span_texts(&lines[0]), vec!["Hello world"]);
    }

    #[test]
    fn render_bold_span_has_bold_modifier() {
        let lines = render("foo **bar** baz");
        let bold = lines[0]
            .spans
            .iter()
            .find(|s| s.content == "bar")
            .expect("bold span");
        assert!(bold.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn render_italic_span_has_italic_modifier() {
        let lines = render("foo *bar* baz");
        let italic = lines[0]
            .spans
            .iter()
            .find(|s| s.content == "bar")
            .expect("italic span");
        assert!(italic.style.add_modifier.contains(Modifier::ITALIC));
    }

    #[test]
    fn render_inline_code_is_yellow() {
        let lines = render("call `foo()` now");
        let code = lines[0]
            .spans
            .iter()
            .find(|s| s.content == "foo()")
            .expect("code span");
        assert_eq!(code.style.fg, Some(Color::Yellow));
    }
}
```

- [ ] **Step 3: Run tests and confirm they fail**

Run: `cargo test --lib markdown::tests`
Expected: Either compiles and all tests pass (if `pulldown-cmark` and renderer logic line up), OR fails on compile/style assertions — whichever, this step confirms the *initial state* before iterating.

If any test fails, fix the renderer until all 5 pass.

- [ ] **Step 4: Run the full test suite to confirm no regressions**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/tui/mod.rs src/tui/markdown.rs
git commit -m "feat(tui): markdown renderer scaffold with inline styling"
```

---

## Task 3: Renderer — heading support

**Files:**
- Modify: `src/tui/markdown.rs`

- [ ] **Step 1: Write failing tests**

Append to the `tests` module in `src/tui/markdown.rs`, just before the closing `}`:

```rust
    #[test]
    fn render_h1_is_cyan_bold() {
        let lines = render("# Title");
        assert_eq!(lines.len(), 1);
        let span = &lines[0].spans[0];
        assert_eq!(span.content, "Title");
        assert_eq!(span.style.fg, Some(Color::Cyan));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn render_h2_is_magenta_bold() {
        let lines = render("## Section");
        let span = &lines[0].spans[0];
        assert_eq!(span.style.fg, Some(Color::Magenta));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn render_h3_is_yellow_bold() {
        let lines = render("### Sub");
        let span = &lines[0].spans[0];
        assert_eq!(span.style.fg, Some(Color::Yellow));
        assert!(span.style.add_modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn render_h4_h5_h6_are_white_bold() {
        for src in ["#### a", "##### b", "###### c"] {
            let lines = render(src);
            let span = &lines[0].spans[0];
            assert_eq!(span.style.fg, Some(Color::White), "src={src}");
            assert!(span.style.add_modifier.contains(Modifier::BOLD), "src={src}");
        }
    }
```

- [ ] **Step 2: Run tests, expect failures**

Run: `cargo test --lib markdown::tests`
Expected: New heading tests fail — headings currently render with no style.

- [ ] **Step 3: Implement heading handling**

In `src/tui/markdown.rs`, modify the `handle` method's match arm — add these arms before the `_ => {}` fallback:

```rust
            Event::Start(Tag::Heading { level, .. }) => {
                let color = match level {
                    pulldown_cmark::HeadingLevel::H1 => Color::Cyan,
                    pulldown_cmark::HeadingLevel::H2 => Color::Magenta,
                    pulldown_cmark::HeadingLevel::H3 => Color::Yellow,
                    _ => Color::White,
                };
                let style = Style::default().fg(color).add_modifier(Modifier::BOLD);
                self.style_stack.push(style);
            }
            Event::End(TagEnd::Heading(_)) => {
                self.style_stack.pop();
                self.flush_line();
                self.lines.push(Line::from(""));
            }
```

- [ ] **Step 4: Run tests, expect pass**

Run: `cargo test --lib markdown::tests`
Expected: All heading tests pass plus existing inline tests still pass.

- [ ] **Step 5: Commit**

```bash
git add src/tui/markdown.rs
git commit -m "feat(tui/markdown): heading styling H1-H6"
```

---

## Task 4: Renderer — lists, code blocks, links

**Files:**
- Modify: `src/tui/markdown.rs`

- [ ] **Step 1: Write failing tests**

Append to the `tests` module in `src/tui/markdown.rs`:

```rust
    #[test]
    fn render_unordered_list_item_starts_with_bullet() {
        let lines = render("- item one\n- item two\n");
        assert_eq!(lines.len(), 2);
        assert!(span_texts(&lines[0])[0].starts_with("• "));
        assert!(span_texts(&lines[1])[0].starts_with("• "));
    }

    #[test]
    fn render_ordered_list_item_starts_with_number() {
        let lines = render("1. one\n2. two\n");
        assert_eq!(lines.len(), 2);
        assert!(span_texts(&lines[0])[0].starts_with("1. "));
        assert!(span_texts(&lines[1])[0].starts_with("2. "));
    }

    #[test]
    fn render_fenced_code_block_emits_one_line_per_source_line() {
        let lines = render("```\nlet x = 1;\nlet y = 2;\n```\n");
        // Two source lines → two rendered lines.
        let code_lines: Vec<&Line> = lines
            .iter()
            .filter(|l| {
                l.spans
                    .iter()
                    .any(|s| s.content.contains("let "))
            })
            .collect();
        assert_eq!(code_lines.len(), 2);
        for line in &code_lines {
            let span = &line.spans[0];
            assert_eq!(span.style.fg, Some(Color::Gray));
        }
    }

    #[test]
    fn render_link_emits_underlined_cyan_with_url() {
        let lines = render("see [docs](https://example.com)");
        let text = span_texts(&lines[0]).join("");
        assert!(text.contains("docs"));
        assert!(text.contains("https://example.com"));
        let link_span = lines[0]
            .spans
            .iter()
            .find(|s| s.content.contains("docs"))
            .expect("link span");
        assert_eq!(link_span.style.fg, Some(Color::Cyan));
        assert!(link_span
            .style
            .add_modifier
            .contains(Modifier::UNDERLINED));
    }

    #[test]
    fn render_does_not_panic_on_table_or_html() {
        // Tables and raw HTML are dropped; renderer must not panic.
        let _ = render("| a | b |\n|---|---|\n| 1 | 2 |\n");
        let _ = render("<div>raw</div>");
    }
```

- [ ] **Step 2: Run tests, expect failures**

Run: `cargo test --lib markdown::tests`
Expected: New tests fail — list/code-block/link events are currently ignored.

- [ ] **Step 3: Replace the `Renderer` struct definition**

In `src/tui/markdown.rs`, replace the existing `Renderer` struct (the `#[derive(Default)] struct Renderer { ... }` block) with:

```rust
#[derive(Default)]
struct Renderer {
    lines: Vec<Line<'static>>,
    current: Vec<Span<'static>>,
    style_stack: Vec<Style>,
    list_stack: Vec<Option<u64>>, // Some(n) for ordered (next index), None for unordered
    in_code_block: bool,
    pending_link_url: Option<String>,
}
```

- [ ] **Step 4: Replace the `Event::Text` arm**

In `src/tui/markdown.rs`, in the `handle` method, replace the existing line `Event::Text(t) => self.push_span(t.into_string()),` with:

```rust
            Event::Text(t) => {
                if self.in_code_block {
                    let text = t.into_string();
                    for (i, line) in text.split('\n').enumerate() {
                        if i > 0 {
                            self.flush_line();
                        }
                        self.current.push(Span::styled(
                            line.to_string(),
                            Style::default().fg(Color::Gray),
                        ));
                    }
                } else {
                    self.push_span(t.into_string());
                }
            }
```

- [ ] **Step 5: Add list / code block / link arms**

In `src/tui/markdown.rs`, in the `handle` method, insert these arms immediately before the final `_ => {}` fallback:

```rust
            Event::Start(Tag::List(first)) => {
                self.list_stack.push(first);
            }
            Event::End(TagEnd::List(_)) => {
                self.list_stack.pop();
            }
            Event::Start(Tag::Item) => {
                let prefix = match self.list_stack.last_mut() {
                    Some(Some(n)) => {
                        let p = format!("{}. ", n);
                        *n += 1;
                        p
                    }
                    _ => "• ".to_string(),
                };
                self.current.push(Span::raw(prefix));
            }
            Event::End(TagEnd::Item) => {
                self.flush_line();
            }
            Event::Start(Tag::CodeBlock(_)) => {
                self.in_code_block = true;
            }
            Event::End(TagEnd::CodeBlock) => {
                self.in_code_block = false;
                self.flush_line();
            }
            Event::Start(Tag::Link { dest_url, .. }) => {
                let style = Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::UNDERLINED);
                self.style_stack.push(style);
                self.pending_link_url = Some(dest_url.into_string());
            }
            Event::End(TagEnd::Link) => {
                self.style_stack.pop();
                if let Some(url) = self.pending_link_url.take() {
                    self.current.push(Span::styled(
                        format!(" ({})", url),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::UNDERLINED),
                    ));
                }
            }
```

- [ ] **Step 6: Run tests, expect pass**

Run: `cargo test --lib markdown::tests`
Expected: All renderer tests pass (13 in total: empty, paragraph, bold, italic, code, H1, H2, H3, H4-H6, ul, ol, code block, link, no-panic).

- [ ] **Step 7: Run clippy and fmt**

Run: `cargo fmt --check && cargo clippy --lib -- -D warnings`
Expected: No warnings.

- [ ] **Step 8: Commit**

```bash
git add src/tui/markdown.rs
git commit -m "feat(tui/markdown): list, code-block, and link rendering"
```

---

## Task 5: Add `Focus` enum and `home_dir` to App

**Files:**
- Modify: `src/tui/app.rs:1-53`
- Modify: `src/tui/mod.rs:43`

- [ ] **Step 1: Add `Focus` enum and fields**

In `src/tui/app.rs`, replace the `Mode` enum block (lines 3-9) with:

```rust
use std::path::PathBuf;

#[derive(Debug, PartialEq)]
pub enum Mode {
    Normal,
    Search,
    ViewMarkdown,
    ConfirmDisable,
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Focus {
    Tree,
    Preview,
}
```

In the same file, modify the `App` struct (lines 19-33) — add two new fields:

```rust
pub struct App {
    pub tree: Vec<TreeNode>,
    pub flat: Vec<FlatEntry>,
    pub selected: usize,
    pub mode: Mode,
    pub search_query: String,
    pub detail_scroll: u16,
    pub markdown_content: Option<String>,
    pub markdown_scroll: u16,
    pub status_message: Option<(String, std::time::Instant)>,
    pub should_quit: bool,
    pub show_enabled_only: bool,
    pub pending_disable: Option<String>,
    pub focus: Focus,
    pub home_dir: Option<PathBuf>,
}
```

Modify `App::new` (lines 36-53):

```rust
    pub fn new(tree: Vec<TreeNode>, home_dir: Option<PathBuf>) -> Self {
        let mut app = Self {
            tree,
            flat: Vec::new(),
            selected: 0,
            mode: Mode::Normal,
            search_query: String::new(),
            detail_scroll: 0,
            markdown_content: None,
            markdown_scroll: 0,
            status_message: None,
            should_quit: false,
            show_enabled_only: false,
            pending_disable: None,
            focus: Focus::Tree,
            home_dir,
        };
        app.rebuild_flat();
        app
    }
```

- [ ] **Step 2: Update the call site**

Edit `src/tui/mod.rs:43`. Replace:

```rust
    let mut app = App::new(tree_nodes);
```

with:

```rust
    let mut app = App::new(tree_nodes, Some(home_dir.to_path_buf()));
```

- [ ] **Step 3: Verify the build**

Run: `cargo build`
Expected: Builds with no errors.

- [ ] **Step 4: Run tests**

Run: `cargo test`
Expected: All existing tests still pass.

- [ ] **Step 5: Commit**

```bash
git add src/tui/app.rs src/tui/mod.rs
git commit -m "feat(tui/app): add Focus enum and home_dir field"
```

---

## Task 6: Implement `update_preview()` on App

**Files:**
- Modify: `src/tui/app.rs`

- [ ] **Step 1: Write failing tests**

Append to `src/tui/app.rs` a new `#[cfg(test)] mod tests` block at the end of the file:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::tree::{NodeKind, TreeNode};
    use std::io::Write;

    fn leaf_with_path(name: &str, path: Option<String>) -> TreeNode {
        TreeNode {
            name: name.to_string(),
            kind: NodeKind::Skill,
            enabled: true,
            scope: None,
            path,
            plugin_id: None,
            children: Vec::new(),
            expanded: false,
            hidden: false,
        }
    }

    fn make_app_with(nodes: Vec<TreeNode>) -> App {
        App::new(nodes, None)
    }

    #[test]
    fn update_preview_clears_for_node_without_path() {
        let mut app = make_app_with(vec![leaf_with_path("orphan", None)]);
        app.markdown_content = Some("stale".to_string());
        app.focus = Focus::Preview;
        app.update_preview();
        assert!(app.markdown_content.is_none());
        assert_eq!(app.focus, Focus::Tree);
    }

    #[test]
    fn update_preview_clears_for_plugin_pseudo_path() {
        let mut app = make_app_with(vec![leaf_with_path(
            "plug",
            Some("plugin foo".to_string()),
        )]);
        app.markdown_content = Some("stale".to_string());
        app.focus = Focus::Preview;
        app.update_preview();
        assert!(app.markdown_content.is_none());
        assert_eq!(app.focus, Focus::Tree);
    }

    #[test]
    fn update_preview_loads_existing_file() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "# Hello").unwrap();
        let path = file.path().to_string_lossy().to_string();

        let mut app = make_app_with(vec![leaf_with_path("skill", Some(path))]);
        app.update_preview();
        let content = app.markdown_content.as_deref().unwrap_or("");
        assert!(content.contains("# Hello"));
        assert_eq!(app.markdown_scroll, 0);
    }

    #[test]
    fn update_preview_clears_and_resets_focus_on_read_error() {
        let mut app = make_app_with(vec![leaf_with_path(
            "missing",
            Some("/nonexistent/path/file.md".to_string()),
        )]);
        app.markdown_content = Some("stale".to_string());
        app.focus = Focus::Preview;
        app.markdown_scroll = 5;
        app.update_preview();
        assert!(app.markdown_content.is_none());
        assert_eq!(app.focus, Focus::Tree);
        assert_eq!(app.markdown_scroll, 0);
        assert!(app.status_message.is_some());
    }

    #[test]
    fn expand_tilde_uses_home_dir() {
        let home = std::path::PathBuf::from("/home/user");
        assert_eq!(expand_tilde("~/file.md", Some(&home)), "/home/user/file.md");
        assert_eq!(expand_tilde("/abs/path", Some(&home)), "/abs/path");
        assert_eq!(expand_tilde("~/file.md", None), "~/file.md");
    }
}
```

- [ ] **Step 2: Run tests, expect failures**

Run: `cargo test --lib tui::app::tests`
Expected: Compile failure — `update_preview` and `expand_tilde` are not defined yet.

- [ ] **Step 3: Implement `expand_tilde` and `update_preview`**

In `src/tui/app.rs`, just above the `fn flatten_node(` definition (around line 174), insert:

```rust
/// Expand a leading "~/" segment using the supplied home directory.
/// Leaves the path unchanged if it does not start with "~/" or if no home is set.
pub fn expand_tilde(path: &str, home: Option<&std::path::Path>) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(h) = home {
            return h.join(rest).to_string_lossy().to_string();
        }
    }
    path.to_string()
}
```

Then add an `impl App` block continuation — append a new method inside the existing `impl App { ... }` block, just before its closing brace:

```rust
    /// Reload the markdown preview based on the current selection.
    ///
    /// Hides the preview when the selected node has no readable file. Resets
    /// scroll and force-returns focus to the tree whenever preview becomes
    /// unavailable, so the user is never stranded in preview focus.
    pub fn update_preview(&mut self) {
        self.markdown_content = None;
        self.markdown_scroll = 0;

        let Some(node) = self.selected_node() else {
            self.focus = Focus::Tree;
            return;
        };

        let Some(path) = node.path.clone() else {
            self.focus = Focus::Tree;
            return;
        };

        if path.starts_with("plugin ") {
            self.focus = Focus::Tree;
            return;
        }

        let expanded = expand_tilde(&path, self.home_dir.as_deref());
        match std::fs::read_to_string(&expanded) {
            Ok(content) => {
                self.markdown_content = Some(content);
            }
            Err(e) => {
                self.focus = Focus::Tree;
                self.set_status(format!("Preview unavailable: {}", e));
            }
        }
    }
```

- [ ] **Step 4: Run tests, expect pass**

Run: `cargo test --lib tui::app::tests`
Expected: All 5 new tests pass.

- [ ] **Step 5: Run the full suite**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 6: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat(tui/app): update_preview lazy-loads markdown on selection"
```

---

## Task 7: Wire `update_preview()` into App mutators

**Files:**
- Modify: `src/tui/app.rs`

- [ ] **Step 1: Write failing tests**

In the existing `mod tests` block at the bottom of `src/tui/app.rs`, append:

```rust
    #[test]
    fn move_down_calls_update_preview() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "second").unwrap();
        let path = file.path().to_string_lossy().to_string();

        let mut app = make_app_with(vec![
            leaf_with_path("first", None),
            leaf_with_path("second", Some(path.clone())),
        ]);
        assert!(app.markdown_content.is_none());
        app.move_down();
        assert!(app
            .markdown_content
            .as_deref()
            .unwrap_or("")
            .contains("second"));
    }

    #[test]
    fn move_up_calls_update_preview() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "first").unwrap();
        let path = file.path().to_string_lossy().to_string();

        let mut app = make_app_with(vec![
            leaf_with_path("first", Some(path)),
            leaf_with_path("second", None),
        ]);
        app.move_down();
        assert!(app.markdown_content.is_none());
        app.move_up();
        assert!(app
            .markdown_content
            .as_deref()
            .unwrap_or("")
            .contains("first"));
    }

    #[test]
    fn new_calls_update_preview_for_initial_selection() {
        let mut file = tempfile::NamedTempFile::new().unwrap();
        writeln!(file, "initial").unwrap();
        let path = file.path().to_string_lossy().to_string();

        let app = make_app_with(vec![leaf_with_path("initial", Some(path))]);
        assert!(app
            .markdown_content
            .as_deref()
            .unwrap_or("")
            .contains("initial"));
    }
```

- [ ] **Step 2: Run tests, expect failures**

Run: `cargo test --lib tui::app::tests`
Expected: New tests fail — `move_up`/`move_down`/`new` do not yet call `update_preview`.

- [ ] **Step 3: Wire `update_preview` into mutators**

In `src/tui/app.rs`:

Replace `App::new` body so it calls `update_preview` after `rebuild_flat`:

```rust
    pub fn new(tree: Vec<TreeNode>, home_dir: Option<PathBuf>) -> Self {
        let mut app = Self {
            tree,
            flat: Vec::new(),
            selected: 0,
            mode: Mode::Normal,
            search_query: String::new(),
            detail_scroll: 0,
            markdown_content: None,
            markdown_scroll: 0,
            status_message: None,
            should_quit: false,
            show_enabled_only: false,
            pending_disable: None,
            focus: Focus::Tree,
            home_dir,
        };
        app.rebuild_flat();
        app.update_preview();
        app
    }
```

Replace `move_up` and `move_down` (originally lines 91-103):

```rust
    pub fn move_up(&mut self) {
        if self.selected > 0 {
            self.selected -= 1;
        }
        self.detail_scroll = 0;
        self.update_preview();
    }

    pub fn move_down(&mut self) {
        if !self.flat.is_empty() && self.selected < self.flat.len() - 1 {
            self.selected += 1;
        }
        self.detail_scroll = 0;
        self.update_preview();
    }
```

Replace `toggle_expand`, `expand`, `collapse` (originally lines 105-139) — append `self.update_preview();` after `self.rebuild_flat();` in each:

```rust
    pub fn toggle_expand(&mut self) {
        if let Some(entry) = self.flat.get(self.selected) {
            if !entry.is_expandable {
                return;
            }
            let path = entry.node_index.clone();
            if let Some(node) = self.resolve_node_mut(&path) {
                node.expanded = !node.expanded;
            }
            self.rebuild_flat();
            self.update_preview();
        }
    }

    pub fn expand(&mut self) {
        if let Some(entry) = self.flat.get(self.selected) {
            if !entry.is_expandable {
                return;
            }
            let path = entry.node_index.clone();
            if let Some(node) = self.resolve_node_mut(&path) {
                node.expanded = true;
            }
            self.rebuild_flat();
            self.update_preview();
        }
    }

    pub fn collapse(&mut self) {
        if let Some(entry) = self.flat.get(self.selected) {
            let path = entry.node_index.clone();
            if let Some(node) = self.resolve_node_mut(&path) {
                node.expanded = false;
            }
            self.rebuild_flat();
            self.update_preview();
        }
    }
```

Replace `toggle_enabled_filter` (originally lines 141-151):

```rust
    pub fn toggle_enabled_filter(&mut self) {
        self.show_enabled_only = !self.show_enabled_only;
        self.rebuild_flat();
        self.selected = 0;
        let label = if self.show_enabled_only {
            "enabled only"
        } else {
            "all"
        };
        self.set_status(format!("Filter: {}", label));
        self.update_preview();
    }
```

Replace `apply_search_filter` (originally lines 157-171):

```rust
    pub fn apply_search_filter(&mut self) {
        if self.search_query.is_empty() {
            for node in &mut self.tree {
                unhide_all(node);
            }
        } else {
            let query = self.search_query.to_lowercase();
            for node in &mut self.tree {
                filter_node(node, &query);
            }
        }
        self.rebuild_flat();
        self.selected = 0;
        self.update_preview();
    }
```

- [ ] **Step 4: Run tests, expect pass**

Run: `cargo test`
Expected: All tests pass (existing + 3 new).

- [ ] **Step 5: Commit**

```bash
git add src/tui/app.rs
git commit -m "feat(tui/app): call update_preview from selection-changing mutators"
```

---

## Task 8: Handler — focus-routed keys and update_preview after toggle

**Files:**
- Modify: `src/tui/handler.rs`

- [ ] **Step 1: Write failing tests**

Append a new `#[cfg(test)]` block to the end of `src/tui/handler.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tui::app::Focus;
    use crate::tui::tree::{NodeKind, TreeNode};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::io::Write;
    use std::path::PathBuf;

    fn leaf(name: &str, path: Option<String>) -> TreeNode {
        TreeNode {
            name: name.to_string(),
            kind: NodeKind::Skill,
            enabled: true,
            scope: None,
            path,
            plugin_id: None,
            children: Vec::new(),
            expanded: false,
            hidden: false,
        }
    }

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    fn tmp_path(contents: &str) -> (tempfile::NamedTempFile, String) {
        let mut f = tempfile::NamedTempFile::new().unwrap();
        write!(f, "{}", contents).unwrap();
        let p = f.path().to_string_lossy().to_string();
        (f, p)
    }

    #[test]
    fn tab_with_preview_switches_focus_to_preview() {
        let (_f, p) = tmp_path("# x");
        let mut app = App::new(vec![leaf("a", Some(p))], None);
        let home = PathBuf::from("/");
        assert_eq!(app.focus, Focus::Tree);
        handle_key(&mut app, key(KeyCode::Tab), &home).unwrap();
        assert_eq!(app.focus, Focus::Preview);
    }

    #[test]
    fn tab_without_preview_is_noop() {
        let mut app = App::new(vec![leaf("a", None)], None);
        let home = PathBuf::from("/");
        handle_key(&mut app, key(KeyCode::Tab), &home).unwrap();
        assert_eq!(app.focus, Focus::Tree);
    }

    #[test]
    fn j_in_preview_focus_scrolls_markdown() {
        let (_f, p) = tmp_path("# x");
        let mut app = App::new(vec![leaf("a", Some(p))], None);
        let home = PathBuf::from("/");
        handle_key(&mut app, key(KeyCode::Tab), &home).unwrap();
        let before = app.markdown_scroll;
        handle_key(&mut app, key(KeyCode::Char('j')), &home).unwrap();
        assert_eq!(app.markdown_scroll, before + 1);
    }

    #[test]
    fn k_in_preview_saturates_at_zero() {
        let (_f, p) = tmp_path("# x");
        let mut app = App::new(vec![leaf("a", Some(p))], None);
        let home = PathBuf::from("/");
        handle_key(&mut app, key(KeyCode::Tab), &home).unwrap();
        handle_key(&mut app, key(KeyCode::Char('k')), &home).unwrap();
        assert_eq!(app.markdown_scroll, 0);
    }

    #[test]
    fn pagedown_in_preview_pages() {
        let (_f, p) = tmp_path("# x");
        let mut app = App::new(vec![leaf("a", Some(p))], None);
        let home = PathBuf::from("/");
        handle_key(&mut app, key(KeyCode::Tab), &home).unwrap();
        handle_key(&mut app, key(KeyCode::PageDown), &home).unwrap();
        assert_eq!(app.markdown_scroll, 20);
    }

    #[test]
    fn esc_in_preview_returns_focus_to_tree() {
        let (_f, p) = tmp_path("# x");
        let mut app = App::new(vec![leaf("a", Some(p))], None);
        let home = PathBuf::from("/");
        handle_key(&mut app, key(KeyCode::Tab), &home).unwrap();
        handle_key(&mut app, key(KeyCode::Esc), &home).unwrap();
        assert_eq!(app.focus, Focus::Tree);
        assert!(!app.should_quit);
    }

    #[test]
    fn esc_in_tree_quits() {
        let mut app = App::new(vec![leaf("a", None)], None);
        let home = PathBuf::from("/");
        handle_key(&mut app, key(KeyCode::Esc), &home).unwrap();
        assert!(app.should_quit);
    }
}
```

- [ ] **Step 2: Run tests, expect failures**

Run: `cargo test --lib tui::handler::tests`
Expected: New tests fail — `Tab` and focus routing are not handled.

- [ ] **Step 3: Replace `handle_normal`**

In `src/tui/handler.rs`, replace the existing `handle_normal` function (lines 39-126) with:

```rust
fn handle_normal(app: &mut App, key: KeyEvent, home_dir: &Path) -> io::Result<()> {
    use crate::tui::app::Focus;

    // Focus-routed keys.
    match (app.focus, key.code) {
        (Focus::Preview, KeyCode::Char('j') | KeyCode::Down) => {
            app.markdown_scroll = app.markdown_scroll.saturating_add(1);
            return Ok(());
        }
        (Focus::Preview, KeyCode::Char('k') | KeyCode::Up) => {
            app.markdown_scroll = app.markdown_scroll.saturating_sub(1);
            return Ok(());
        }
        (Focus::Preview, KeyCode::PageDown) => {
            app.markdown_scroll = app.markdown_scroll.saturating_add(20);
            return Ok(());
        }
        (Focus::Preview, KeyCode::PageUp) => {
            app.markdown_scroll = app.markdown_scroll.saturating_sub(20);
            return Ok(());
        }
        (Focus::Preview, KeyCode::Char('h') | KeyCode::Left) => return Ok(()),
        (Focus::Preview, KeyCode::Char('l') | KeyCode::Right) => return Ok(()),
        (Focus::Preview, KeyCode::Enter) => return Ok(()),
        (Focus::Preview, KeyCode::Esc) => {
            app.focus = Focus::Tree;
            return Ok(());
        }
        (Focus::Tree, KeyCode::Tab) => {
            if app.markdown_content.is_some() {
                app.focus = Focus::Preview;
            } else {
                app.set_status("No preview available".to_string());
            }
            return Ok(());
        }
        (Focus::Preview, KeyCode::Tab) => {
            app.focus = Focus::Tree;
            return Ok(());
        }
        _ => {}
    }

    // Tree-focus default routing (and focus-agnostic keys).
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Left | KeyCode::Char('h') => app.collapse(),
        KeyCode::Right | KeyCode::Char('l') => app.expand(),
        KeyCode::Enter => app.toggle_expand(),

        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.search_query.clear();
            app.apply_search_filter();
        }

        KeyCode::Char('e') => {
            if let Some(node) = app.selected_node() {
                if let Some(plugin_id) = node.plugin_id.clone() {
                    if node.enabled {
                        app.pending_disable = Some(plugin_id);
                        app.mode = Mode::ConfirmDisable;
                    } else {
                        execute_toggle(app, home_dir, &plugin_id, false);
                    }
                } else {
                    app.set_status("No plugin selected".to_string());
                }
            }
        }

        KeyCode::Char('i') => {
            app.toggle_enabled_filter();
        }

        KeyCode::Char('v') => {
            if let Some(node) = app.selected_node() {
                let path_opt = node.path.clone();
                match path_opt {
                    None => {
                        app.set_status("No path for selected item".to_string());
                    }
                    Some(ref p) if p.starts_with("plugin ") => {
                        app.set_status(format!("Cannot read: '{}' is not a file path", p));
                    }
                    Some(ref p) => {
                        let expanded =
                            crate::tui::app::expand_tilde(p, app.home_dir.as_deref());
                        match std::fs::read_to_string(&expanded) {
                            Ok(content) => {
                                app.markdown_content = Some(content);
                                app.markdown_scroll = 0;
                                app.mode = Mode::ViewMarkdown;
                            }
                            Err(e) => {
                                app.set_status(format!("Cannot read '{}': {}", expanded, e));
                            }
                        }
                    }
                }
            }
        }

        _ => {}
    }
    Ok(())
}
```

This change also (a) removes the local `dirs::home_dir()` lookup for `~/` expansion in the `v` handler (now uses `app.home_dir` via the shared `expand_tilde`) and (b) deletes the now-unused `use std::path::Path;` if it's no longer referenced — verify by running `cargo check` and re-adding only if needed.

- [ ] **Step 4: Run tests, expect pass**

Run: `cargo test`
Expected: All tests pass — handler tests (7 new) and existing app/markdown tests.

- [ ] **Step 5: Lint**

Run: `cargo clippy -- -D warnings`
Expected: No warnings.

- [ ] **Step 6: Commit**

```bash
git add src/tui/handler.rs
git commit -m "feat(tui/handler): focus-routed keys with Tab toggle"
```

---

## Task 9: Call `update_preview` after `execute_toggle`

**Files:**
- Modify: `src/tui/handler.rs:18-37`

- [ ] **Step 1: Write the failing test**

Append to the `tests` module in `src/tui/handler.rs`:

```rust
    #[test]
    fn execute_toggle_refreshes_preview() {
        // After a toggle, the tree is rebuilt — the selected node may now point
        // to a different file (e.g., a previously-hidden plugin becomes visible).
        // This is a smoke test that update_preview is called after execute_toggle,
        // mirroring the same contract that move_*/expand/* satisfy.
        let (_f, p) = tmp_path("# initial");
        let mut node = leaf("a", Some(p.clone()));
        node.plugin_id = Some("a".to_string());
        let mut app = App::new(vec![node], None);
        assert!(app
            .markdown_content
            .as_deref()
            .unwrap_or("")
            .contains("initial"));

        // Overwrite the file mid-session.
        std::fs::write(&p, "# changed").unwrap();

        // Use the public path: simulate a plugin enable (currently_enabled=false).
        let home = PathBuf::from("/");
        execute_toggle(&mut app, &home, "a", false);

        assert!(app
            .markdown_content
            .as_deref()
            .unwrap_or("")
            .contains("changed"));
    }
```

Note: `execute_toggle` will attempt to write `home_dir/.claude/settings.json`. To keep the test hermetic, use a real tempdir for home:

Replace the inline `let home = PathBuf::from("/");` in this test with:

```rust
        let home_tmp = tempfile::tempdir().unwrap();
        let home = home_tmp.path().to_path_buf();
```

- [ ] **Step 2: Run test, expect failure**

Run: `cargo test --lib tui::handler::tests::execute_toggle_refreshes_preview`
Expected: Failure — `markdown_content` still contains "initial" because no reload happens.

- [ ] **Step 3: Add `update_preview` call in `execute_toggle`**

In `src/tui/handler.rs`, replace the existing `execute_toggle` function:

```rust
fn execute_toggle(app: &mut App, home_dir: &Path, plugin_id: &str, currently_enabled: bool) {
    match actions::toggle_plugin(home_dir, plugin_id, currently_enabled) {
        Ok(()) => {
            if let Some(node_mut) = app.selected_node_mut() {
                node_mut.enabled = !currently_enabled;
            }
            app.rebuild_flat();
            app.update_preview();
            let action = if currently_enabled {
                "disabled"
            } else {
                "enabled"
            };
            app.set_status(format!("Plugin '{}' {}", plugin_id, action));
        }
        Err(e) => {
            app.set_status(format!("Error toggling plugin: {}", e));
        }
    }
}
```

- [ ] **Step 4: Run tests, expect pass**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 5: Commit**

```bash
git add src/tui/handler.rs
git commit -m "fix(tui/handler): refresh preview after plugin toggle"
```

---

## Task 10: Split Detail rendering

**Files:**
- Modify: `src/tui/ui.rs:161-271`

- [ ] **Step 1: Replace `render_detail`**

In `src/tui/ui.rs`, replace the entire `render_detail` function (lines 161-271) with:

```rust
fn render_detail(frame: &mut Frame, app: &App, area: Rect) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Detail ")
        .title_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let Some(node) = app.selected_node() else {
        let para = Paragraph::new("No selection");
        frame.render_widget(para, inner);
        return;
    };

    let metadata = build_metadata_lines(app, node);
    let preview_present = app.markdown_content.is_some();

    if !preview_present {
        let para = Paragraph::new(metadata)
            .wrap(Wrap { trim: false })
            .scroll((app.detail_scroll, 0));
        frame.render_widget(para, inner);
        return;
    }

    let meta_height = metadata.len() as u16 + 1;
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(meta_height), Constraint::Min(0)])
        .split(inner);

    let meta_para = Paragraph::new(metadata).wrap(Wrap { trim: false });
    frame.render_widget(meta_para, chunks[0]);

    let preview_border_color = if app.focus == crate::tui::app::Focus::Preview {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let preview_block = Block::default()
        .borders(Borders::TOP)
        .title(" Preview ")
        .title_style(Style::default().fg(preview_border_color))
        .border_style(Style::default().fg(preview_border_color));

    let preview_inner = preview_block.inner(chunks[1]);
    frame.render_widget(preview_block, chunks[1]);

    let content = app.markdown_content.as_deref().unwrap_or("");
    let rendered = crate::tui::markdown::render(content);
    let preview_para = Paragraph::new(rendered)
        .wrap(Wrap { trim: false })
        .scroll((app.markdown_scroll, 0));
    frame.render_widget(preview_para, preview_inner);
}

fn build_metadata_lines(app: &App, node: &crate::tui::tree::TreeNode) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    lines.push(Line::from(vec![
        Span::styled("Name:   ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            node.name.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    let kind_str = match node.kind {
        crate::tui::tree::NodeKind::SectionHeader => "Section",
        crate::tui::tree::NodeKind::Plugin => "Plugin",
        crate::tui::tree::NodeKind::Skill => "Skill",
        crate::tui::tree::NodeKind::Command => "Command",
        crate::tui::tree::NodeKind::Agent => "Agent",
        crate::tui::tree::NodeKind::McpServer => "MCP Server",
        crate::tui::tree::NodeKind::Hook => "Hook",
    };
    lines.push(Line::from(vec![
        Span::styled("Type:   ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(kind_str),
    ]));

    let scope_str = match &node.scope {
        Some(crate::inspect::Scope::Project) => "Project",
        Some(crate::inspect::Scope::Global) => "Global",
        None => "—",
    };
    lines.push(Line::from(vec![
        Span::styled("Scope:  ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(scope_str),
    ]));

    let (status_str, status_color) = if node.enabled {
        ("Enabled", Color::Green)
    } else {
        ("Disabled", Color::DarkGray)
    };
    lines.push(Line::from(vec![
        Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(status_str, Style::default().fg(status_color)),
    ]));

    if let Some(path) = &node.path {
        lines.push(Line::from(vec![
            Span::styled("Path:   ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(path.clone()),
        ]));
    }

    if let Some(plugin_id) = &node.plugin_id {
        lines.push(Line::from(vec![
            Span::styled("Plugin: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(plugin_id.clone()),
        ]));
    }

    if !node.children.is_empty() {
        lines.push(Line::from(vec![
            Span::styled("Items:  ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(node.children.len().to_string()),
        ]));
    }

    lines.push(Line::from(""));
    lines.push(keybind_hint_line(app));

    lines
}

fn keybind_hint_line(app: &App) -> Line<'static> {
    let filter_label = if app.show_enabled_only {
        "all"
    } else {
        "enabled"
    };
    let text = match (app.focus, app.markdown_content.is_some()) {
        (crate::tui::app::Focus::Preview, _) => {
            "[Tab/Esc] back  [j/k] scroll  [PgUp/PgDn] page  [v] fullscreen  [q] quit"
                .to_string()
        }
        (crate::tui::app::Focus::Tree, true) => format!(
            "[Tab] preview  [e] toggle  [v] full  [i] {}  [/] search  [q] quit",
            filter_label
        ),
        (crate::tui::app::Focus::Tree, false) => format!(
            "[e] toggle  [v] view  [i] {}  [/] search  [q] quit",
            filter_label
        ),
    };
    Line::from(Span::styled(text, Style::default().fg(Color::DarkGray)))
}
```

- [ ] **Step 2: Verify build**

Run: `cargo build`
Expected: Builds cleanly.

- [ ] **Step 3: Run all tests**

Run: `cargo test`
Expected: All tests pass.

- [ ] **Step 4: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat(tui/ui): split Detail pane with inline markdown preview"
```

---

## Task 11: Focus indicator on tree border + styled fullscreen overlay

**Files:**
- Modify: `src/tui/ui.rs` — `render_tree`, `render_markdown_overlay`

- [ ] **Step 1: Update `render_tree` border color**

In `src/tui/ui.rs`, locate `render_tree` (around lines 48-159). Replace the block construction (lines 57-65) with:

```rust
    let border_color = if app.focus == crate::tui::app::Focus::Tree {
        Color::Cyan
    } else {
        Color::DarkGray
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(
            Style::default()
                .fg(border_color)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(border_color));
```

- [ ] **Step 2: Update `render_markdown_overlay` to use the styled renderer**

In `src/tui/ui.rs`, replace the body of `render_markdown_overlay` (lines 379-395) with:

```rust
fn render_markdown_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Markdown View — [q/Esc] close ")
        .title_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = app.markdown_content.as_deref().unwrap_or("");
    let rendered = crate::tui::markdown::render(content);
    let para = Paragraph::new(rendered)
        .wrap(Wrap { trim: false })
        .scroll((app.markdown_scroll, 0));
    frame.render_widget(para, inner);
}
```

- [ ] **Step 3: Verify build**

Run: `cargo build`
Expected: Builds cleanly.

- [ ] **Step 4: Run tests + lint**

Run: `cargo test && cargo clippy -- -D warnings`
Expected: All tests pass, no warnings.

- [ ] **Step 5: Commit**

```bash
git add src/tui/ui.rs
git commit -m "feat(tui/ui): focus border indicator + styled fullscreen overlay"
```

---

## Task 12: Final verification

**Files:** none

- [ ] **Step 1: Format check**

Run: `mise run fmt -- --check` (or `cargo fmt --check`)
Expected: No diff.

- [ ] **Step 2: Lint**

Run: `mise run lint` (or `cargo clippy --all-targets -- -D warnings`)
Expected: No warnings.

- [ ] **Step 3: Full test suite**

Run: `mise run test` (or `cargo test`)
Expected: All tests pass.

- [ ] **Step 4: Manual smoke test**

Run: `cargo run -- inspect`

Verify in order:
1. Navigate to a plugin → Skills → pick a skill. The right pane shows metadata followed by a styled markdown preview separated by a horizontal rule and `Preview` title.
2. Press `Tab`. The preview-block separator turns cyan; the tree block dims. The keybind hint line changes to scroll keys.
3. Press `j` and `k` while in preview focus — markdown scrolls, tree selection unchanged.
4. Press `PageDown` / `PageUp` — preview pages by ~20 lines.
5. Press `Esc`. Focus returns to tree (does NOT quit).
6. Navigate to a plugin row (whose path is `plugin <id>`). The preview pane disappears, metadata expands. Keybind hint reverts to default.
7. Navigate to a section header. Same as 6.
8. Press `v` on a skill. Fullscreen overlay shows styled markdown (colored headings, yellow code spans, etc.). Press `Esc` or `q` to close.
9. Toggle a plugin with `e`. Confirm popup behavior unchanged. After confirming, preview updates if the selection's file changed.
10. Press `q` from tree focus — exits cleanly.

If any step fails, fix the issue, add a regression test if applicable, and re-run from Step 1.

- [ ] **Step 5: Commit any fixes**

If Step 4 surfaced any issues that required code changes, commit them as `fix(tui): <issue>` before moving on.

---

## Self-Review Notes

Cross-checked against spec sections:

- **Architecture / new module** → Tasks 2-4 (renderer scaffold + heading + lists/code/links).
- **`Focus` enum, `home_dir`, `update_preview`** → Tasks 5-7.
- **Handler key routing + Tab/Esc** → Task 8.
- **Preview refresh after toggle** → Task 9.
- **UI split, auto-size metadata, dynamic hint, focus border** → Tasks 10-11.
- **Styled fullscreen overlay** → Task 11.
- **Manual smoke test** → Task 12.

All identifiers used in later tasks (`update_preview`, `expand_tilde`, `Focus::{Tree, Preview}`, `markdown::render`) are defined in earlier tasks. No placeholders. Code blocks are concrete and complete.
