use chord::inspect::Scope;
use chord::operations::scope::set_plugin_enabled;
use chord::operations::OpContext;
use std::fs;
use tempfile::TempDir;

fn make_ctx<'a>(project: &'a TempDir, home: &'a TempDir, packages: &'a TempDir) -> OpContext<'a> {
    OpContext {
        project_root: project.path(),
        home_dir: home.path(),
        packages_dir: packages.path(),
        verbose: false,
    }
}

#[test]
fn set_plugin_enabled_writes_global_settings_when_scope_global() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();

    let ctx = make_ctx(&project, &home, &packages);
    set_plugin_enabled("demo@market", Scope::Global, true, &ctx).unwrap();

    let content = fs::read_to_string(home.path().join(".claude/settings.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["enabledPlugins"]["demo@market"].as_bool().unwrap());
}

#[test]
fn set_plugin_enabled_writes_project_settings_when_scope_project() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();

    let ctx = make_ctx(&project, &home, &packages);
    set_plugin_enabled("demo@market", Scope::Project, true, &ctx).unwrap();

    let project_settings = project.path().join(".claude/settings.json");
    assert!(project_settings.exists());
    let global_settings = home.path().join(".claude/settings.json");
    assert!(!global_settings.exists(), "global should be untouched");
}

#[test]
fn set_plugin_enabled_false_removes_key() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();

    fs::create_dir_all(home.path().join(".claude")).unwrap();
    fs::write(
        home.path().join(".claude/settings.json"),
        r#"{"enabledPlugins":{"demo@market":true,"other@m":true}}"#,
    )
    .unwrap();

    let ctx = make_ctx(&project, &home, &packages);
    set_plugin_enabled("demo@market", Scope::Global, false, &ctx).unwrap();

    let content = fs::read_to_string(home.path().join(".claude/settings.json")).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["enabledPlugins"].get("demo@market").is_none());
    assert!(json["enabledPlugins"]["other@m"].as_bool().unwrap());
}
