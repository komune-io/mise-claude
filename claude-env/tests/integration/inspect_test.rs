use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

/// Set up a project dir and a fake home dir with representative fixtures.
///
/// Project dir contains:
/// - `claude-env.toml`          with [mcp] context7 + [plugins] code-review entry
/// - `.mcp.json`                with two servers: context7-mcp (matches config) + manual-mcp (manual)
/// - `.claude/settings.json`    with enabledPlugins containing code-review@claude-code-plugins
/// - `.claude/skills/my-skill/SKILL.md`
/// - `.claude/commands/review.md`
fn setup_project() -> (TempDir, TempDir) {
    let project_dir = TempDir::new().unwrap();
    let home_dir = TempDir::new().unwrap();

    // claude-env.toml: declare one MCP tool and one plugin
    fs::write(
        project_dir.path().join("claude-env.toml"),
        "[mcp]\ncontext7 = \"2.1.4\"\n\n[plugins]\n\"anthropics/claude-code/code-review@claude-code-plugins\" = \"latest\"\n",
    )
    .unwrap();

    // .mcp.json: context7-mcp (managed, matches config) + manual-mcp (manual, not in config)
    let mcp_json = serde_json::json!({
        "mcpServers": {
            "context7-mcp": {
                "type": "stdio",
                "command": "npx",
                "args": ["-y", "@upstash/context7-mcp@2.1.4"]
            },
            "manual-mcp": {
                "type": "stdio",
                "command": "npx",
                "args": ["-y", "some-manual-mcp"]
            }
        }
    });
    fs::write(
        project_dir.path().join(".mcp.json"),
        serde_json::to_string_pretty(&mcp_json).unwrap(),
    )
    .unwrap();

    // .claude/settings.json with enabledPlugins object
    let settings_dir = project_dir.path().join(".claude");
    fs::create_dir_all(&settings_dir).unwrap();
    let settings_json = serde_json::json!({
        "enabledPlugins": {
            "code-review@claude-code-plugins": true
        }
    });
    fs::write(
        settings_dir.join("settings.json"),
        serde_json::to_string_pretty(&settings_json).unwrap(),
    )
    .unwrap();

    // .claude/skills/my-skill/SKILL.md
    let skill_dir = settings_dir.join("skills").join("my-skill");
    fs::create_dir_all(&skill_dir).unwrap();
    fs::write(skill_dir.join("SKILL.md"), "# my-skill\n").unwrap();

    // .claude/commands/review.md
    let commands_dir = settings_dir.join("commands");
    fs::create_dir_all(&commands_dir).unwrap();
    fs::write(commands_dir.join("review.md"), "# review command\n").unwrap();

    (project_dir, home_dir)
}

#[test]
fn inspect_shows_all_categories() {
    let (project_dir, home_dir) = setup_project();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("inspect")
        .current_dir(project_dir.path())
        .env("HOME", home_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // MCP Servers section
    assert!(stdout.contains("MCP Servers"), "stdout should contain 'MCP Servers', got:\n{stdout}");
    assert!(stdout.contains("context7-mcp"), "stdout should contain 'context7-mcp', got:\n{stdout}");
    assert!(stdout.contains("manual-mcp"), "stdout should contain 'manual-mcp', got:\n{stdout}");
    assert!(stdout.contains("manual"), "stdout should contain 'manual', got:\n{stdout}");

    // Plugins section
    assert!(stdout.contains("Plugins"), "stdout should contain 'Plugins', got:\n{stdout}");
    assert!(
        stdout.contains("code-review@claude-code-plugins"),
        "stdout should contain 'code-review@claude-code-plugins', got:\n{stdout}"
    );

    // Skills section
    assert!(stdout.contains("Skills"), "stdout should contain 'Skills', got:\n{stdout}");
    assert!(stdout.contains("my-skill"), "stdout should contain 'my-skill', got:\n{stdout}");

    // Commands section
    assert!(stdout.contains("Commands"), "stdout should contain 'Commands', got:\n{stdout}");
    assert!(stdout.contains("review"), "stdout should contain 'review', got:\n{stdout}");
}

#[test]
fn inspect_section_filter() {
    let (project_dir, home_dir) = setup_project();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("inspect")
        .arg("--section")
        .arg("mcp")
        .current_dir(project_dir.path())
        .env("HOME", home_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("MCP Servers"), "stdout should contain 'MCP Servers', got:\n{stdout}");
    assert!(!stdout.contains("Plugins"), "stdout should NOT contain 'Plugins', got:\n{stdout}");
    assert!(!stdout.contains("Skills"), "stdout should NOT contain 'Skills', got:\n{stdout}");
}

#[test]
fn inspect_json_output() {
    let (project_dir, home_dir) = setup_project();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("inspect")
        .arg("--json")
        .current_dir(project_dir.path())
        .env("HOME", home_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    let json: serde_json::Value =
        serde_json::from_str(&stdout).expect("stdout should be valid JSON");

    // Top-level keys are arrays
    assert!(json["mcp"].is_array(), "json[\"mcp\"] should be an array, got:\n{stdout}");
    assert!(json["plugins"].is_array(), "json[\"plugins\"] should be an array, got:\n{stdout}");
    assert!(json["skills"].is_array(), "json[\"skills\"] should be an array, got:\n{stdout}");

    // context7-mcp should appear as managed
    let mcp_arr = json["mcp"].as_array().unwrap();
    let context7_entry = mcp_arr
        .iter()
        .find(|e| e["name"] == "context7-mcp")
        .expect("mcp array should contain context7-mcp");
    assert_eq!(
        context7_entry["source"], "managed",
        "context7-mcp should be 'managed', got:\n{stdout}"
    );

    // manual-mcp should appear as manual
    let manual_entry = mcp_arr
        .iter()
        .find(|e| e["name"] == "manual-mcp")
        .expect("mcp array should contain manual-mcp");
    assert_eq!(
        manual_entry["source"], "manual",
        "manual-mcp should be 'manual', got:\n{stdout}"
    );
}

#[test]
fn inspect_drift_shown_for_missing_tool() {
    let project_dir = TempDir::new().unwrap();
    let home_dir = TempDir::new().unwrap();

    // Config declares shadcn but no .mcp.json entry for it
    fs::write(
        project_dir.path().join("claude-env.toml"),
        "[mcp]\nshadcn = \"latest\"\n",
    )
    .unwrap();

    // .mcp.json is empty (no servers)
    let mcp_json = serde_json::json!({ "mcpServers": {} });
    fs::write(
        project_dir.path().join(".mcp.json"),
        serde_json::to_string_pretty(&mcp_json).unwrap(),
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("inspect")
        .current_dir(project_dir.path())
        .env("HOME", home_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("shadcn"), "stdout should contain 'shadcn', got:\n{stdout}");
    assert!(stdout.contains("MISSING"), "stdout should contain 'MISSING', got:\n{stdout}");
}
