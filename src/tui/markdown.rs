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
            Event::Text(t) => self.push_span(t.into_string()),
            Event::Code(t) => {
                let style = self.current_style().fg(Color::Yellow);
                self.current.push(Span::styled(t.into_string(), style));
            }
            Event::SoftBreak | Event::HardBreak => self.flush_line(),
            _ => {}
        }
    }

    /// Finalize the document, dropping the trailing blank line sentinel.
    /// Every block-ending event (Paragraph, etc.) pushes a `Line::from("")` spacer,
    /// so the document always ends with one trailing blank — `spans.is_empty()` reliably matches it.
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

    fn span_texts<'a>(line: &'a Line<'a>) -> Vec<&'a str> {
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
            assert!(
                span.style.add_modifier.contains(Modifier::BOLD),
                "src={src}"
            );
        }
    }
}
