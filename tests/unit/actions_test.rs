use chord::tui::actions;
use std::fs;
use tempfile::TempDir;

#[test]
fn toggle_plugin_disables() {
    let home = TempDir::new().unwrap();
    let claude_dir = home.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"enabledPlugins":{"superpowers@claude-plugins-official":true,"caveman@caveman":true}}"#,
    )
    .unwrap();

    actions::toggle_plugin(home.path(), "superpowers@claude-plugins-official", true).unwrap();

    let content = fs::read_to_string(claude_dir.join("settings.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["enabledPlugins"]
        .get("superpowers@claude-plugins-official")
        .is_none());
    assert!(json["enabledPlugins"]["caveman@caveman"].as_bool().unwrap());
}

#[test]
fn toggle_plugin_enables() {
    let home = TempDir::new().unwrap();
    let claude_dir = home.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();
    fs::write(
        claude_dir.join("settings.json"),
        r#"{"enabledPlugins":{"caveman@caveman":true}}"#,
    )
    .unwrap();

    actions::toggle_plugin(home.path(), "superpowers@claude-plugins-official", false).unwrap();

    let content = fs::read_to_string(claude_dir.join("settings.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(
        json["enabledPlugins"]["superpowers@claude-plugins-official"]
            .as_bool()
            .unwrap()
    );
    assert!(json["enabledPlugins"]["caveman@caveman"].as_bool().unwrap());
}

#[test]
fn toggle_plugin_creates_settings_if_missing() {
    let home = TempDir::new().unwrap();
    let claude_dir = home.path().join(".claude");
    fs::create_dir_all(&claude_dir).unwrap();

    actions::toggle_plugin(home.path(), "new-plugin@marketplace", false).unwrap();

    let content = fs::read_to_string(claude_dir.join("settings.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["enabledPlugins"]["new-plugin@marketplace"]
        .as_bool()
        .unwrap());
}
