use std::io;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::tui::actions;
use crate::tui::app::{App, Mode};

pub fn handle_key(app: &mut App, key: KeyEvent, home_dir: &Path) -> io::Result<()> {
    match app.mode {
        Mode::Normal => handle_normal(app, key, home_dir),
        Mode::Search => handle_search(app, key),
        Mode::ViewMarkdown => handle_markdown(app, key),
    }
}

fn handle_normal(app: &mut App, key: KeyEvent, home_dir: &Path) -> io::Result<()> {
    match key.code {
        // Quit
        KeyCode::Char('q') | KeyCode::Esc => {
            app.should_quit = true;
        }
        KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            app.should_quit = true;
        }

        // Navigation
        KeyCode::Up | KeyCode::Char('k') => app.move_up(),
        KeyCode::Down | KeyCode::Char('j') => app.move_down(),
        KeyCode::Left | KeyCode::Char('h') => app.collapse(),
        KeyCode::Right | KeyCode::Char('l') => app.expand(),
        KeyCode::Enter => app.toggle_expand(),

        // Search mode
        KeyCode::Char('/') => {
            app.mode = Mode::Search;
            app.search_query.clear();
            app.apply_search_filter();
        }

        // Toggle plugin
        KeyCode::Char('e') => {
            if let Some(node) = app.selected_node() {
                if let Some(plugin_id) = node.plugin_id.clone() {
                    let currently_enabled = node.enabled;
                    match actions::toggle_plugin(home_dir, &plugin_id, currently_enabled) {
                        Ok(()) => {
                            // Update tree state
                            if let Some(node_mut) = app.selected_node_mut() {
                                node_mut.enabled = !currently_enabled;
                            }
                            app.rebuild_flat();
                            let action = if currently_enabled { "disabled" } else { "enabled" };
                            app.set_status(format!("Plugin '{}' {}", plugin_id, action));
                        }
                        Err(e) => {
                            app.set_status(format!("Error toggling plugin: {}", e));
                        }
                    }
                } else {
                    app.set_status("No plugin selected".to_string());
                }
            }
        }

        // Toggle enabled/all filter
        KeyCode::Char('i') => {
            app.toggle_enabled_filter();
        }

        // View markdown
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
                        // Expand ~/
                        let expanded = if p.starts_with("~/") {
                            let rest = &p[2..];
                            match dirs::home_dir() {
                                Some(h) => h.join(rest).to_string_lossy().to_string(),
                                None => p.clone(),
                            }
                        } else {
                            p.clone()
                        };

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

fn handle_search(app: &mut App, key: KeyEvent) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.search_query.clear();
            app.apply_search_filter();
        }
        KeyCode::Enter => {
            app.mode = Mode::Normal;
            // Keep the current filter active
        }
        KeyCode::Backspace => {
            app.search_query.pop();
            app.apply_search_filter();
        }
        KeyCode::Char(c) => {
            app.search_query.push(c);
            app.apply_search_filter();
        }
        _ => {}
    }
    Ok(())
}

fn handle_markdown(app: &mut App, key: KeyEvent) -> io::Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.markdown_content = None;
            app.markdown_scroll = 0;
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.markdown_scroll = app.markdown_scroll.saturating_sub(1);
        }
        KeyCode::Down | KeyCode::Char('j') => {
            app.markdown_scroll = app.markdown_scroll.saturating_add(1);
        }
        KeyCode::PageUp => {
            app.markdown_scroll = app.markdown_scroll.saturating_sub(20);
        }
        KeyCode::PageDown => {
            app.markdown_scroll = app.markdown_scroll.saturating_add(20);
        }
        _ => {}
    }
    Ok(())
}
