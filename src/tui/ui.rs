use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap};

use crate::tui::app::{App, Mode};
use crate::tui::tree::NodeKind;

pub fn render(frame: &mut Frame, app: &App) {
    match app.mode {
        Mode::ViewMarkdown => render_markdown_overlay(frame, app),
        Mode::ConfirmDisable => {
            render_main(frame, app);
            render_confirm_disable_popup(frame, app);
        }
        Mode::AddPrompt => {
            render_main(frame, app);
            render_add_prompt(frame, app);
        }
        _ => render_main(frame, app),
    }
}

fn render_main(frame: &mut Frame, app: &App) {
    let area = frame.area();

    // Split vertically: main content + 1-line status bar
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(1)])
        .split(area);

    let main_area = vertical[0];
    let status_area = vertical[1];

    // Split main horizontally: 45% tree + 55% detail
    let horizontal = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
        .split(main_area);

    let tree_area = horizontal[0];
    let detail_area = horizontal[1];

    render_tree(frame, app, tree_area);
    render_detail(frame, app, detail_area);
    render_status(frame, app, status_area);
}

fn render_tree(frame: &mut Frame, app: &App, area: Rect) {
    let title = if app.mode == Mode::Search && !app.search_query.is_empty() {
        format!(" chord [/{}] ", app.search_query)
    } else if app.show_enabled_only {
        " chord [enabled only] ".to_string()
    } else {
        " chord ".to_string()
    };

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

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = app
        .flat
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let node = match app.resolve_node(&entry.node_index) {
                Some(n) => n,
                None => return ListItem::new(""),
            };

            let indent = "  ".repeat(entry.depth);

            // Arrow indicator
            let arrow = if entry.is_expandable {
                if entry.expanded {
                    "▼ "
                } else {
                    "▶ "
                }
            } else {
                "  "
            };

            // Plugin symbol
            let symbol = if node.kind == NodeKind::Plugin {
                if node.enabled {
                    "● "
                } else {
                    "○ "
                }
            } else {
                ""
            };

            let drift_marker = if node.drift { "⚠ " } else { "" };

            let label = format!("{}{}{}{}{}", indent, arrow, symbol, drift_marker, node.name);

            let base_style = if node.drift {
                Style::default().fg(Color::Red)
            } else if node.kind == NodeKind::SectionHeader {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            } else if node.kind == NodeKind::Plugin {
                if node.enabled {
                    Style::default().fg(Color::Green)
                } else {
                    Style::default().fg(Color::DarkGray)
                }
            } else if node.enabled {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            // Symbol style for plugin dot
            let style = if i == app.selected {
                base_style.add_modifier(Modifier::REVERSED)
            } else {
                base_style
            };

            // Build spans with symbol coloring for plugins
            let spans = if node.kind == NodeKind::Plugin && !symbol.is_empty() {
                let symbol_color = if node.enabled {
                    Color::Green
                } else {
                    Color::DarkGray
                };
                let symbol_style = if i == app.selected {
                    Style::default()
                        .fg(symbol_color)
                        .add_modifier(Modifier::REVERSED)
                } else {
                    Style::default().fg(symbol_color)
                };
                let text_style = style;
                let prefix = format!("{}{}", indent, arrow);
                Line::from(vec![
                    Span::styled(prefix, text_style),
                    Span::styled(symbol.to_string(), symbol_style),
                    Span::styled(node.name.clone(), text_style),
                ])
            } else {
                Line::from(vec![Span::styled(label, style)])
            };

            ListItem::new(spans)
        })
        .collect();

    let list = List::new(items);
    frame.render_widget(list, inner);
}

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

    let mut metadata = build_metadata_lines(node);
    metadata.push(Line::from(""));
    metadata.push(keybind_hint_line(app));
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

fn build_metadata_lines(node: &crate::tui::tree::TreeNode) -> Vec<Line<'static>> {
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

    if node.drift {
        lines.push(Line::from(vec![
            Span::styled("        ", Style::default()),
            Span::styled(
                "⚠ drift (declared, not installed)",
                Style::default().fg(Color::Red),
            ),
        ]));
    }

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
            "[Tab/Esc] back  [j/k] scroll  [PgUp/PgDn] page  [v] fullscreen  [q] quit".to_string()
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

fn render_status(frame: &mut Frame, app: &App, area: Rect) {
    let content = match app.mode {
        Mode::Search => {
            format!("/{}", app.search_query)
        }
        _ => {
            if let Some((msg, _)) = &app.status_message {
                msg.clone()
            } else {
                {
                    let filter = if app.show_enabled_only {
                        " [enabled only]"
                    } else {
                        ""
                    };
                    format!(
                        "chord inspect{} — [q] quit [/] search [i] toggle filter",
                        filter
                    )
                }
            }
        }
    };

    let style = match app.mode {
        Mode::Search => Style::default().fg(Color::Yellow),
        _ => {
            if app.status_message.is_some() {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            }
        }
    };

    let para = Paragraph::new(content).style(style);
    frame.render_widget(para, area);
}

fn render_confirm_disable_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 28, frame.area());

    // Wipe the cells underneath so the main view doesn't bleed through.
    frame.render_widget(Clear, area);

    let plugin_id = app.pending_disable.as_deref().unwrap_or("(unknown)");

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Confirm disable ")
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from(""),
        Line::from("Disable plugin?").alignment(Alignment::Center),
        Line::from(""),
        Line::from(Span::styled(
            plugin_id.to_string(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .alignment(Alignment::Center),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "[Y]es / Enter",
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::raw("     "),
            Span::styled("[N]o / Esc", Style::default().fg(Color::Green)),
        ])
        .alignment(Alignment::Center),
    ];

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

fn render_add_prompt(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 26, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Add tool ")
        .title_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .border_style(Style::default().fg(Color::Yellow));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let lines = vec![
        Line::from("<section>:<name>@<version>").alignment(Alignment::Center),
        Line::from(""),
        Line::from(vec![
            Span::raw("> "),
            Span::styled(
                app.add_input.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled("_", Style::default().fg(Color::Cyan)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "section \u{2208} {mcp, cli, skills, plugins}",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled("[Enter]", Style::default().fg(Color::Green)),
            Span::raw(" add    "),
            Span::styled("[Esc]", Style::default().fg(Color::Red)),
            Span::raw(" cancel"),
        ])
        .alignment(Alignment::Center),
    ];

    let para = Paragraph::new(lines).wrap(Wrap { trim: false });
    frame.render_widget(para, inner);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let vertical = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(vertical[1])[1]
}

fn render_markdown_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Markdown View — [q/Esc] close  [j/k] scroll  [PgUp/PgDn] page ")
        .title_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    // Header: metadata when a selection exists; falls back to body-only otherwise.
    let metadata = app.selected_node().map(build_metadata_lines);

    let content = app.markdown_content.as_deref().unwrap_or("");
    let rendered = crate::tui::markdown::render(content);

    let body_para = Paragraph::new(rendered)
        .wrap(Wrap { trim: false })
        .scroll((app.markdown_scroll, 0));

    match metadata {
        Some(meta) if !meta.is_empty() => {
            let meta_height = meta.len() as u16 + 1;
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([Constraint::Length(meta_height), Constraint::Min(0)])
                .split(inner);

            let meta_para = Paragraph::new(meta).wrap(Wrap { trim: false });
            frame.render_widget(meta_para, chunks[0]);

            let sep = Block::default()
                .borders(Borders::TOP)
                .border_style(Style::default().fg(Color::DarkGray));
            let sep_inner = sep.inner(chunks[1]);
            frame.render_widget(sep, chunks[1]);
            frame.render_widget(body_para, sep_inner);
        }
        _ => frame.render_widget(body_para, inner),
    }
}
