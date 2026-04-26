use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::tui::app::{App, Mode};
use crate::tui::tree::NodeKind;

pub fn render(frame: &mut Frame, app: &App) {
    match app.mode {
        Mode::ViewMarkdown => {
            render_markdown_overlay(frame, app);
        }
        _ => {
            render_main(frame, app);
        }
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
        format!(" claude-env [/{}] ", app.search_query)
    } else if app.show_enabled_only {
        " claude-env [enabled only] ".to_string()
    } else {
        " claude-env ".to_string()
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .title_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let items: Vec<ListItem> = app
        .flat
        .iter()
        .enumerate()
        .map(|(i, entry)| {
            let node = match app.resolve_node_pub(&entry.node_index) {
                Some(n) => n,
                None => return ListItem::new(""),
            };

            let indent = "  ".repeat(entry.depth);

            // Arrow indicator
            let arrow = if entry.is_expandable {
                if entry.expanded { "▼ " } else { "▶ " }
            } else {
                "  "
            };

            // Plugin symbol
            let symbol = if node.kind == NodeKind::Plugin {
                if node.enabled { "● " } else { "○ " }
            } else {
                ""
            };

            let label = format!("{}{}{}{}", indent, arrow, symbol, node.name);

            let base_style = if node.kind == NodeKind::SectionHeader {
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
                let symbol_color = if node.enabled { Color::Green } else { Color::DarkGray };
                let symbol_style = if i == app.selected {
                    Style::default().fg(symbol_color).add_modifier(Modifier::REVERSED)
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

    let mut lines: Vec<Line> = Vec::new();

    // Name
    lines.push(Line::from(vec![
        Span::styled("Name:   ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(
            node.name.clone(),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
    ]));

    // Type
    let kind_str = match node.kind {
        NodeKind::SectionHeader => "Section",
        NodeKind::Plugin => "Plugin",
        NodeKind::Skill => "Skill",
        NodeKind::Command => "Command",
        NodeKind::Agent => "Agent",
        NodeKind::McpServer => "MCP Server",
        NodeKind::Hook => "Hook",
    };
    lines.push(Line::from(vec![
        Span::styled("Type:   ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(kind_str),
    ]));

    // Scope
    let scope_str = match &node.scope {
        Some(crate::inspect::Scope::Project) => "Project",
        Some(crate::inspect::Scope::Global) => "Global",
        None => "—",
    };
    lines.push(Line::from(vec![
        Span::styled("Scope:  ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(scope_str),
    ]));

    // Status
    let (status_str, status_color) = if node.enabled {
        ("Enabled", Color::Green)
    } else {
        ("Disabled", Color::DarkGray)
    };
    lines.push(Line::from(vec![
        Span::styled("Status: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::styled(status_str, Style::default().fg(status_color)),
    ]));

    // Path
    if let Some(path) = &node.path {
        lines.push(Line::from(vec![
            Span::styled("Path:   ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(path.clone()),
        ]));
    }

    // Plugin ID
    if let Some(plugin_id) = &node.plugin_id {
        lines.push(Line::from(vec![
            Span::styled(
                "Plugin: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(plugin_id.clone()),
        ]));
    }

    // Child count for plugin/section nodes
    if !node.children.is_empty() {
        lines.push(Line::from(vec![
            Span::styled(
                "Items:  ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(node.children.len().to_string()),
        ]));
    }

    // Spacer
    lines.push(Line::from(""));

    // Keybinding hints
    let filter_label = if app.show_enabled_only { "all" } else { "enabled" };
    lines.push(Line::from(Span::styled(
        format!("[e] toggle  [v] view  [i] {}  [/] search  [q] quit", filter_label),
        Style::default().fg(Color::DarkGray),
    )));

    let para = Paragraph::new(lines)
        .wrap(Wrap { trim: false })
        .scroll((app.detail_scroll, 0));
    frame.render_widget(para, inner);
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
                    let filter = if app.show_enabled_only { " [enabled only]" } else { "" };
                    format!("claude-env inspect{} — [q] quit [/] search [i] toggle filter", filter)
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

fn render_markdown_overlay(frame: &mut Frame, app: &App) {
    let area = frame.area();

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Markdown View — [q/Esc] close ")
        .title_style(Style::default().fg(Color::Cyan));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let content = app.markdown_content.as_deref().unwrap_or("");
    let para = Paragraph::new(content)
        .wrap(Wrap { trim: false })
        .scroll((app.markdown_scroll, 0));
    frame.render_widget(para, inner);
}
