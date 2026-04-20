use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn list_shows_installed_tools() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();

    // Write claude-env.toml with two MCP tools.
    fs::write(
        project_dir.path().join("claude-env.toml"),
        "[mcp]\ncontext7 = \"2.1.4\"\nmemory = \"1.0.0\"\n",
    )
    .unwrap();

    // Write matching lockfile.
    fs::write(
        project_dir.path().join("claude-env.lock"),
        "[mcp]\n\
        [mcp.context7]\nversion = \"2.1.4\"\n\n\
        [mcp.memory]\nversion = \"1.0.0\"\n",
    )
    .unwrap();

    // Create fake installed package dirs (context7 installed, memory missing).
    let context7_modules = packages_dir.path().join("context7").join("node_modules");
    fs::create_dir_all(&context7_modules).unwrap();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("list")
        .current_dir(project_dir.path())
        .env("CLAUDE_ENV_HOME", packages_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    // Both tool names should appear.
    assert!(stdout.contains("context7"), "stdout should contain context7, got: {stdout}");
    assert!(stdout.contains("memory"), "stdout should contain memory, got: {stdout}");

    // Versions from lockfile.
    assert!(stdout.contains("2.1.4"), "stdout should contain 2.1.4, got: {stdout}");
    assert!(stdout.contains("1.0.0"), "stdout should contain 1.0.0, got: {stdout}");

    // context7 is installed, memory is not.
    assert!(stdout.contains("✓ installed"), "stdout should show installed status, got: {stdout}");
    assert!(stdout.contains("✗ missing"), "stdout should show missing status, got: {stdout}");
}

#[test]
fn list_empty_config_shows_header_only() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();

    // No claude-env.toml, no lockfile.
    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("list")
        .current_dir(project_dir.path())
        .env("CLAUDE_ENV_HOME", packages_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("TOOL"), "stdout should contain header, got: {stdout}");
    assert!(stdout.contains("VERSION"), "stdout should contain header, got: {stdout}");
    assert!(stdout.contains("STATUS"), "stdout should contain header, got: {stdout}");
}

#[test]
fn list_shows_question_mark_for_unlocked_tools() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();

    // Config with a tool but no lockfile.
    fs::write(
        project_dir.path().join("claude-env.toml"),
        "[mcp]\ncontext7 = \"latest\"\n",
    )
    .unwrap();

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("list")
        .current_dir(project_dir.path())
        .env("CLAUDE_ENV_HOME", packages_dir.path());

    let output = cmd.assert().success();
    let stdout = String::from_utf8(output.get_output().stdout.clone()).unwrap();

    assert!(stdout.contains("context7"), "stdout should contain tool name, got: {stdout}");
    assert!(stdout.contains('?'), "stdout should show '?' for unlocked tool, got: {stdout}");
}
