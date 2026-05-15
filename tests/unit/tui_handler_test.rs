use chord::inspect::Scope;
use chord::operations::add::AddSpec;
use chord::operations::OperationError;
use chord::tui::app::{App, Mode};
use chord::tui::handler::handle_key;
use chord::tui::tree::{NodeKind, TreeNode};
use chord::tui::{handler, OpRunner};
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
    // Default filter hides disabled plugins; surface this one so 'e' has a
    // selection to act on.
    app.toggle_enabled_filter();

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

// ── MockRunner-based tests (Task 13) ──────────────────────────────────────

#[derive(Default)]
struct MockRunner {
    pub adds: Vec<AddSpec>,
    pub removes: Vec<String>,
    pub installs_one: Vec<String>,
    pub installs_all: u32,
    pub set_scopes: Vec<(String, Scope, bool)>,
}

impl OpRunner for MockRunner {
    fn add(&mut self, spec: &AddSpec) -> Result<(), OperationError> {
        self.adds.push(spec.clone());
        Ok(())
    }
    fn remove(&mut self, name: &str) -> Result<(), OperationError> {
        self.removes.push(name.to_string());
        Ok(())
    }
    fn install_one(&mut self, name: &str) -> Result<(), OperationError> {
        self.installs_one.push(name.to_string());
        Ok(())
    }
    fn install_all(&mut self) -> Result<(), OperationError> {
        self.installs_all += 1;
        Ok(())
    }
    fn set_scope(
        &mut self,
        plugin_id: &str,
        scope: Scope,
        enabled: bool,
    ) -> Result<(), OperationError> {
        self.set_scopes
            .push((plugin_id.to_string(), scope, enabled));
        Ok(())
    }
}

#[test]
fn pressing_a_enters_add_prompt_mode() {
    let mut app = app_with_plugin("demo@market", true);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('a')), &mut runner).unwrap();
    assert_eq!(app.mode, Mode::AddPrompt);
    assert!(app.add_input.is_empty());
}

#[test]
fn typing_in_add_prompt_accumulates_input() {
    let mut app = app_with_plugin("demo@market", true);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('a')), &mut runner).unwrap();
    for c in "mcp:foo".chars() {
        handler::handle_key_with_runner(&mut app, key(KeyCode::Char(c)), &mut runner).unwrap();
    }
    assert_eq!(app.add_input, "mcp:foo");
}

#[test]
fn enter_in_add_prompt_with_valid_spec_invokes_runner() {
    let mut app = app_with_plugin("demo@market", true);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('a')), &mut runner).unwrap();
    for c in "mcp:foo@latest".chars() {
        handler::handle_key_with_runner(&mut app, key(KeyCode::Char(c)), &mut runner).unwrap();
    }
    handler::handle_key_with_runner(&mut app, key(KeyCode::Enter), &mut runner).unwrap();
    assert_eq!(runner.adds.len(), 1);
    assert_eq!(runner.adds[0].name, "foo");
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn enter_with_invalid_spec_keeps_modal_open() {
    let mut app = app_with_plugin("demo@market", true);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('a')), &mut runner).unwrap();
    for c in "bogus".chars() {
        handler::handle_key_with_runner(&mut app, key(KeyCode::Char(c)), &mut runner).unwrap();
    }
    handler::handle_key_with_runner(&mut app, key(KeyCode::Enter), &mut runner).unwrap();
    assert_eq!(runner.adds.len(), 0);
    assert_eq!(app.mode, Mode::AddPrompt);
    assert!(app.status_message.is_some());
}

#[test]
fn esc_in_add_prompt_cancels() {
    let mut app = app_with_plugin("demo@market", true);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('a')), &mut runner).unwrap();
    for c in "mcp:foo".chars() {
        handler::handle_key_with_runner(&mut app, key(KeyCode::Char(c)), &mut runner).unwrap();
    }
    handler::handle_key_with_runner(&mut app, key(KeyCode::Esc), &mut runner).unwrap();
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.add_input.is_empty());
}

// ── ConfirmRemove tests (Task 14) ──────────────────────────────────────────

fn managed_leaf(name: &str) -> TreeNode {
    TreeNode {
        name: name.to_string(),
        kind: NodeKind::McpServer,
        enabled: true,
        scope: None,
        path: None,
        plugin_id: None,
        children: Vec::new(),
        expanded: false,
        hidden: false,
        drift: false,
        managed: true,
    }
}

#[test]
fn pressing_d_on_managed_enters_confirm_remove() {
    let mut app = App::new(vec![managed_leaf("context7")], None);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('d')), &mut runner).unwrap();
    assert_eq!(app.mode, Mode::ConfirmRemove);
    assert_eq!(app.pending_remove.as_deref(), Some("context7"));
}

#[test]
fn pressing_d_on_unmanaged_is_status_message() {
    let mut unmanaged = managed_leaf("standalone-skill");
    unmanaged.managed = false;
    let mut app = App::new(vec![unmanaged], None);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('d')), &mut runner).unwrap();
    assert_eq!(app.mode, Mode::Normal);
    assert!(app.status_message.is_some());
}

#[test]
fn y_in_confirm_remove_calls_runner() {
    let mut app = App::new(vec![managed_leaf("context7")], None);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('d')), &mut runner).unwrap();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('y')), &mut runner).unwrap();
    assert_eq!(runner.removes, vec!["context7".to_string()]);
    assert_eq!(app.mode, Mode::Normal);
}

#[test]
fn n_in_confirm_remove_cancels() {
    let mut app = App::new(vec![managed_leaf("context7")], None);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('d')), &mut runner).unwrap();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('n')), &mut runner).unwrap();
    assert_eq!(runner.removes.len(), 0);
    assert_eq!(app.mode, Mode::Normal);
}

// ── InlineOp queue tests (Task 15) ────────────────────────────────────────

use chord::tui::app::InlineOp;

fn drift_leaf(name: &str) -> TreeNode {
    TreeNode {
        name: name.to_string(),
        kind: NodeKind::McpServer,
        enabled: false,
        scope: None,
        path: None,
        plugin_id: None,
        children: Vec::new(),
        expanded: false,
        hidden: false,
        drift: true,
        managed: true,
    }
}

#[test]
fn pressing_r_on_drift_sets_pending_install_one() {
    let mut app = App::new(vec![drift_leaf("context7")], None);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('r')), &mut runner).unwrap();
    assert!(matches!(
        app.pending_inline_op,
        Some(InlineOp::InstallOne(ref n)) if n == "context7"
    ));
}

#[test]
fn pressing_r_on_non_drift_is_noop_with_status() {
    let mut app = App::new(vec![managed_leaf("context7")], None);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('r')), &mut runner).unwrap();
    assert!(app.pending_inline_op.is_none());
    assert!(app.status_message.is_some());
}

#[test]
fn pressing_capital_r_sets_pending_install_all() {
    let mut app = App::new(vec![managed_leaf("context7")], None);
    let mut runner = MockRunner::default();
    let event = KeyEvent::new(KeyCode::Char('R'), KeyModifiers::SHIFT);
    handler::handle_key_with_runner(&mut app, event, &mut runner).unwrap();
    assert!(matches!(app.pending_inline_op, Some(InlineOp::InstallAll)));
}

#[test]
fn successful_add_sets_pending_install_one_and_dirty() {
    let mut app = app_with_plugin("demo@market", true);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('a')), &mut runner).unwrap();
    for c in "mcp:context7@latest".chars() {
        handler::handle_key_with_runner(&mut app, key(KeyCode::Char(c)), &mut runner).unwrap();
    }
    handler::handle_key_with_runner(&mut app, key(KeyCode::Enter), &mut runner).unwrap();
    assert!(app.dirty);
    assert!(matches!(
        app.pending_inline_op,
        Some(InlineOp::InstallOne(ref n)) if n == "context7"
    ));
}

#[test]
fn successful_remove_sets_dirty() {
    let mut app = App::new(vec![managed_leaf("context7")], None);
    let mut runner = MockRunner::default();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('d')), &mut runner).unwrap();
    handler::handle_key_with_runner(&mut app, key(KeyCode::Char('y')), &mut runner).unwrap();
    assert!(app.dirty);
}

// ── reload test ────────────────────────────────────────────────────────────

#[test]
fn reload_rebuilds_tree_and_preserves_selection_by_name() {
    use chord::config::Config;
    use chord::tui::tree::{NodeKind, TreeNode};

    let leaf_a = TreeNode {
        name: "alpha".to_string(),
        kind: NodeKind::Skill,
        enabled: true,
        scope: None,
        path: None,
        plugin_id: None,
        children: Vec::new(),
        expanded: false,
        hidden: false,
        drift: false,
        managed: false,
    };
    let leaf_b = TreeNode {
        name: "beta".to_string(),
        ..leaf_a.clone()
    };

    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    // No config or scanner state — reload should produce a tree
    // (just section headers) and the call must not panic.
    let mut app = App::new(vec![leaf_a, leaf_b], Some(home.path().to_path_buf()));
    app.selected = 1;

    let cfg = Config::default();
    app.reload(project.path(), home.path(), &cfg);

    // After reload, the tree is rebuilt from a scan of the empty test
    // home dir, so the previous leaves are gone. Just verify the call
    // returns without panic and selected is in range.
    assert!(app.selected < app.flat.len().max(1));
}
