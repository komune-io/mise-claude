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
        Mode::ConfirmDisable => handle_confirm_disable(app, key, home_dir),
    }
}

/// Apply a plugin enable/disable, sync the tree, and write a status message.
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
                        let expanded = crate::tui::app::expand_tilde(p, app.home_dir.as_deref());
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

fn handle_confirm_disable(app: &mut App, key: KeyEvent, home_dir: &Path) -> io::Result<()> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            if let Some(plugin_id) = app.pending_disable.take() {
                execute_toggle(app, home_dir, &plugin_id, true);
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.pending_disable = None;
            app.mode = Mode::Normal;
            app.set_status("Disable cancelled".to_string());
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

        // Use a real tempdir for HOME so execute_toggle can write settings.json.
        let home_tmp = tempfile::tempdir().unwrap();
        let home = home_tmp.path().to_path_buf();
        execute_toggle(&mut app, &home, "a", false);

        assert!(app
            .markdown_content
            .as_deref()
            .unwrap_or("")
            .contains("changed"));
    }
}
