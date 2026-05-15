use std::io;
use std::path::Path;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::operations::add::AddSpec;
use crate::tui::app::{App, Mode};
use crate::tui::OpRunner;

/// Test-only entry point that constructs a [`DefaultOpRunner`] inline.
///
/// Production code uses `handle_key_with_runner` directly (called from
/// `tui::run_loop` with a single runner shared across the loop). This
/// shim exists only for unit tests that exercise focus / navigation /
/// search / markdown paths — none of which touch the install or remove
/// subprocess paths, so the placeholder `project_root` and `packages_dir`
/// values below are inert in practice.
#[cfg(test)]
pub fn handle_key(app: &mut App, key: KeyEvent, home_dir: &Path) -> io::Result<()> {
    use crate::tui::DefaultOpRunner;
    let mut runner = DefaultOpRunner::new(Path::new("."), home_dir, Path::new("."), false);
    handle_key_with_runner(app, key, &mut runner)
}

pub fn handle_key_with_runner<R: OpRunner>(
    app: &mut App,
    key: KeyEvent,
    runner: &mut R,
) -> io::Result<()> {
    match app.mode {
        Mode::Normal => handle_normal(app, key, runner),
        Mode::Search => handle_search(app, key),
        Mode::ViewMarkdown => handle_markdown(app, key),
        Mode::ScopePicker => handle_scope_picker(app, key, runner),
        Mode::AddPrompt => handle_add_prompt(app, key, runner),
        Mode::ConfirmRemove => handle_confirm_remove(app, key, runner),
    }
}

fn handle_normal<R: OpRunner>(app: &mut App, key: KeyEvent, _runner: &mut R) -> io::Result<()> {
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
            use crate::tui::app::ScopeTarget;
            if let Some(node) = app.selected_node() {
                if node.kind != crate::tui::tree::NodeKind::Plugin {
                    app.set_status("Not a plugin".to_string());
                    return Ok(());
                }
                let plugin_id = match node.plugin_id.clone() {
                    Some(id) => id,
                    None => {
                        app.set_status("Plugin id missing".to_string());
                        return Ok(());
                    }
                };
                let current = read_scope_state(&plugin_id, app.home_dir.as_deref());
                app.scope_target = Some(ScopeTarget {
                    plugin_id,
                    current: current.clone(),
                    staged: current,
                });
                app.mode = Mode::ScopePicker;
            }
        }

        KeyCode::Char('a') => {
            app.mode = Mode::AddPrompt;
            app.add_input.clear();
        }

        KeyCode::Char('d') => {
            if let Some(node) = app.selected_node() {
                if node.managed {
                    app.pending_remove = Some(node.name.clone());
                    app.mode = Mode::ConfirmRemove;
                } else {
                    app.set_status("Not in chord.toml".to_string());
                }
            }
        }

        KeyCode::Char('r') => {
            if let Some(node) = app.selected_node() {
                if node.drift {
                    app.pending_inline_op =
                        Some(crate::tui::app::InlineOp::InstallOne(node.name.clone()));
                } else {
                    app.set_status("Not a drift entry".to_string());
                }
            }
        }

        KeyCode::Char('R') => {
            app.pending_inline_op = Some(crate::tui::app::InlineOp::InstallAll);
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

fn handle_scope_picker<R: OpRunner>(
    app: &mut App,
    key: KeyEvent,
    runner: &mut R,
) -> io::Result<()> {
    use crate::inspect::Scope;

    match key.code {
        KeyCode::Char('p') => {
            if let Some(t) = app.scope_target.as_mut() {
                t.staged.project = !t.staged.project;
            }
        }
        KeyCode::Char('g') => {
            if let Some(t) = app.scope_target.as_mut() {
                t.staged.global = !t.staged.global;
            }
        }
        KeyCode::Enter => {
            if let Some(t) = app.scope_target.take() {
                let mut errors: Vec<String> = Vec::new();
                if t.staged.project != t.current.project {
                    if let Err(e) = runner.set_scope(&t.plugin_id, Scope::Project, t.staged.project)
                    {
                        errors.push(format!("project: {e}"));
                    }
                }
                if t.staged.global != t.current.global {
                    if let Err(e) = runner.set_scope(&t.plugin_id, Scope::Global, t.staged.global) {
                        errors.push(format!("global: {e}"));
                    }
                }
                if errors.is_empty() {
                    app.set_status(format!("Scope updated for {}", t.plugin_id));
                } else {
                    app.set_status(format!("Scope errors: {}", errors.join("; ")));
                }
                app.dirty = true;
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Esc => {
            app.scope_target = None;
            app.mode = Mode::Normal;
            app.set_status("Scope edit cancelled".to_string());
        }
        _ => {}
    }
    Ok(())
}

fn read_scope_state(plugin_id: &str, home_dir: Option<&Path>) -> crate::tui::app::ScopeState {
    use crate::tui::app::ScopeState;

    fn is_enabled_in(settings_path: &Path, plugin_id: &str) -> bool {
        let content = match std::fs::read_to_string(settings_path) {
            Ok(s) => s,
            Err(_) => return false,
        };
        let json: serde_json::Value = match serde_json::from_str(&content) {
            Ok(v) => v,
            Err(_) => return false,
        };
        let cache_name = plugin_id.split('@').next().unwrap_or(plugin_id);
        json.get("enabledPlugins")
            .and_then(|v| v.as_object())
            .map(|obj| {
                obj.keys()
                    .any(|k| k.split('@').next().unwrap_or(k) == cache_name)
            })
            .unwrap_or(false)
    }

    let project_settings = Path::new(".").join(".claude").join("settings.json");
    let project = is_enabled_in(&project_settings, plugin_id);

    let global = match home_dir {
        Some(h) => is_enabled_in(&h.join(".claude").join("settings.json"), plugin_id),
        None => false,
    };

    ScopeState { project, global }
}

fn handle_confirm_remove<R: OpRunner>(
    app: &mut App,
    key: KeyEvent,
    runner: &mut R,
) -> io::Result<()> {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') | KeyCode::Enter => {
            if let Some(name) = app.pending_remove.take() {
                match runner.remove(&name) {
                    Ok(()) => {
                        app.set_status(format!("Removed {name}"));
                        app.dirty = true;
                    }
                    Err(e) => app.set_status(format!("Remove failed: {e}")),
                }
            }
            app.mode = Mode::Normal;
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.pending_remove = None;
            app.mode = Mode::Normal;
            app.set_status("Remove cancelled".to_string());
        }
        _ => {}
    }
    Ok(())
}

fn handle_add_prompt<R: OpRunner>(app: &mut App, key: KeyEvent, runner: &mut R) -> io::Result<()> {
    match key.code {
        KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.add_input.clear();
        }
        KeyCode::Backspace => {
            app.add_input.pop();
        }
        KeyCode::Enter => match AddSpec::parse(&app.add_input) {
            Ok(spec) => match runner.add(&spec) {
                Ok(()) => {
                    app.set_status(format!("Added {}", spec.name));
                    app.pending_inline_op =
                        Some(crate::tui::app::InlineOp::InstallOne(spec.name.clone()));
                    app.dirty = true;
                    app.mode = Mode::Normal;
                    app.add_input.clear();
                }
                Err(e) => {
                    app.set_status(format!("Add failed: {e}"));
                }
            },
            Err(e) => {
                app.set_status(format!("Invalid spec: {e}"));
            }
        },
        KeyCode::Char(c) => {
            app.add_input.push(c);
        }
        _ => {}
    }
    Ok(())
}

fn handle_markdown(app: &mut App, key: KeyEvent) -> io::Result<()> {
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => {
            app.mode = Mode::Normal;
            app.update_preview();
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
            drift: false,
            managed: false,
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
    fn scope_picker_enter_with_no_changes_marks_dirty_and_returns_to_normal() {
        // Smoke test: opening and immediately confirming scope picker sets dirty
        // and returns to Normal without calling the runner.
        use crate::inspect::Scope;
        use crate::operations::OperationError;
        use crate::tui::app::{ScopeState, ScopeTarget};
        use crate::tui::OpRunner;

        struct NoopRunner;
        impl OpRunner for NoopRunner {
            fn add(
                &mut self,
                _spec: &crate::operations::add::AddSpec,
            ) -> Result<(), OperationError> {
                Ok(())
            }
            fn remove(&mut self, _name: &str) -> Result<(), OperationError> {
                Ok(())
            }
            fn install_one(&mut self, _name: &str) -> Result<(), OperationError> {
                Ok(())
            }
            fn install_all(&mut self) -> Result<(), OperationError> {
                Ok(())
            }
            fn set_scope(
                &mut self,
                _plugin_id: &str,
                _scope: Scope,
                _enabled: bool,
            ) -> Result<(), OperationError> {
                Ok(())
            }
        }

        let mut app = App::new(vec![leaf("a", None)], None);
        let state = ScopeState {
            project: false,
            global: true,
        };
        app.scope_target = Some(ScopeTarget {
            plugin_id: "a".to_string(),
            current: state.clone(),
            staged: state,
        });
        app.mode = Mode::ScopePicker;

        let mut runner = NoopRunner;
        // Press Enter with staged == current → no runner calls, dirty set, mode Normal.
        handle_key_with_runner(&mut app, key(KeyCode::Enter), &mut runner).unwrap();
        assert_eq!(app.mode, Mode::Normal);
        assert!(app.dirty);
    }

    #[test]
    fn closing_markdown_overlay_restores_inline_preview() {
        let (_f, p) = tmp_path("# inline content");
        let mut app = App::new(vec![leaf("a", Some(p))], None);
        let home = PathBuf::from("/");

        // Confirm inline preview loaded by App::new.
        assert!(app
            .markdown_content
            .as_deref()
            .unwrap_or("")
            .contains("inline content"));

        // Open fullscreen overlay via 'v'.
        handle_key(&mut app, key(KeyCode::Char('v')), &home).unwrap();
        assert_eq!(app.mode, crate::tui::app::Mode::ViewMarkdown);

        // Close overlay via Esc.
        handle_key(&mut app, key(KeyCode::Esc), &home).unwrap();
        assert_eq!(app.mode, crate::tui::app::Mode::Normal);

        // Inline preview must still be loaded (not wiped to None).
        assert!(app
            .markdown_content
            .as_deref()
            .unwrap_or("")
            .contains("inline content"));
    }
}
