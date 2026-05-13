use chord::tui::app::{App, Mode};
use chord::tui::handler::handle_key;
use chord::tui::tree::TreeNode;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::fs;
use tempfile::TempDir;

/// Build a minimal App with one plugin node, set to enabled by default.
fn app_with_plugin(plugin_id: &str, enabled: bool) -> App {
    let mut plugin = TreeNode::plugin(plugin_id, enabled, None, None);
    plugin.expanded = false;
    App::new(vec![plugin], None)
}

/// Write a `~/.claude/settings.json` containing the given enabled plugin
/// inside the supplied tempdir and return the home path.
fn home_with_enabled_plugin(home: &TempDir, plugin_id: &str) {
    let claude = home.path().join(".claude");
    fs::create_dir_all(&claude).unwrap();
    fs::write(
        claude.join("settings.json"),
        format!(r#"{{"enabledPlugins":{{"{}":true}}}}"#, plugin_id),
    )
    .unwrap();
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

#[test]
fn pressing_e_on_enabled_plugin_enters_confirm_disable_mode() {
    let home = TempDir::new().unwrap();
    home_with_enabled_plugin(&home, "demo@market");
    let mut app = app_with_plugin("demo@market", true);

    handle_key(&mut app, key(KeyCode::Char('e')), home.path()).unwrap();

    assert_eq!(app.mode, Mode::ConfirmDisable);
    assert_eq!(app.pending_disable.as_deref(), Some("demo@market"));

    // settings.json must NOT have been touched yet.
    let content = fs::read_to_string(home.path().join(".claude/settings.json")).unwrap();
    assert!(content.contains("demo@market"));
}

#[test]
fn pressing_e_on_disabled_plugin_enables_immediately_without_prompt() {
    let home = TempDir::new().unwrap();
    fs::create_dir_all(home.path().join(".claude")).unwrap();
    fs::write(
        home.path().join(".claude/settings.json"),
        r#"{"enabledPlugins":{}}"#,
    )
    .unwrap();
    let mut app = app_with_plugin("demo@market", false);

    handle_key(&mut app, key(KeyCode::Char('e')), home.path()).unwrap();

    assert_eq!(app.mode, Mode::Normal);
    assert!(app.pending_disable.is_none());

    let content = fs::read_to_string(home.path().join(".claude/settings.json")).unwrap();
    assert!(content.contains("demo@market"));
}

#[test]
fn confirm_disable_y_writes_settings_and_returns_to_normal() {
    let home = TempDir::new().unwrap();
    home_with_enabled_plugin(&home, "demo@market");
    let mut app = app_with_plugin("demo@market", true);

    handle_key(&mut app, key(KeyCode::Char('e')), home.path()).unwrap();
    assert_eq!(app.mode, Mode::ConfirmDisable);

    handle_key(&mut app, key(KeyCode::Char('y')), home.path()).unwrap();

    assert_eq!(app.mode, Mode::Normal);
    assert!(app.pending_disable.is_none());

    let content = fs::read_to_string(home.path().join(".claude/settings.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed["enabledPlugins"].get("demo@market").is_none());
}

#[test]
fn confirm_disable_enter_also_confirms() {
    let home = TempDir::new().unwrap();
    home_with_enabled_plugin(&home, "demo@market");
    let mut app = app_with_plugin("demo@market", true);

    handle_key(&mut app, key(KeyCode::Char('e')), home.path()).unwrap();
    handle_key(&mut app, key(KeyCode::Enter), home.path()).unwrap();

    assert_eq!(app.mode, Mode::Normal);
    let content = fs::read_to_string(home.path().join(".claude/settings.json")).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(parsed["enabledPlugins"].get("demo@market").is_none());
}

#[test]
fn confirm_disable_n_cancels_without_writing_settings() {
    let home = TempDir::new().unwrap();
    home_with_enabled_plugin(&home, "demo@market");
    let mut app = app_with_plugin("demo@market", true);

    handle_key(&mut app, key(KeyCode::Char('e')), home.path()).unwrap();
    handle_key(&mut app, key(KeyCode::Char('n')), home.path()).unwrap();

    assert_eq!(app.mode, Mode::Normal);
    assert!(app.pending_disable.is_none());

    let content = fs::read_to_string(home.path().join(".claude/settings.json")).unwrap();
    assert!(content.contains("demo@market"));
}

#[test]
fn confirm_disable_esc_cancels_without_writing_settings() {
    let home = TempDir::new().unwrap();
    home_with_enabled_plugin(&home, "demo@market");
    let mut app = app_with_plugin("demo@market", true);

    handle_key(&mut app, key(KeyCode::Char('e')), home.path()).unwrap();
    handle_key(&mut app, key(KeyCode::Esc), home.path()).unwrap();

    assert_eq!(app.mode, Mode::Normal);
    assert!(app.pending_disable.is_none());

    let content = fs::read_to_string(home.path().join(".claude/settings.json")).unwrap();
    assert!(content.contains("demo@market"));
}
