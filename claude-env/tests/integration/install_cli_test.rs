use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn shims_dir() -> std::path::PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::path::PathBuf::from(manifest)
        .join("tests")
        .join("integration")
        .join("shims")
}

#[test]
fn install_cli_tool_runs_post_install() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    // Write claude-env.toml with a CLI tool.
    fs::write(
        project_dir.path().join("claude-env.toml"),
        "[cli]\nget-shit-done-cc = \"1.22.4\"\n",
    )
    .unwrap();

    // Build PATH with shims first so our fake npm and get-shit-done-cc are found.
    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", shims_dir().display(), original_path);

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("install")
        .current_dir(project_dir.path())
        .env("PATH", &new_path)
        .env("CLAUDE_ENV_HOME", packages_dir.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path());

    cmd.assert().success();

    // Assert npm was called with the correct package and version.
    let npm_log = fs::read_to_string(log_dir.path().join("npm_calls.log")).unwrap();
    assert!(
        npm_log.contains("install"),
        "npm install should have been called"
    );
    assert!(
        npm_log.contains("get-shit-done-cc@1.22.4"),
        "expected versioned package in npm call, got: {npm_log}"
    );
    assert!(
        npm_log.contains("--prefix"),
        "expected --prefix in npm call"
    );

    // Assert post_install was called with the expected arguments.
    let post_install_log =
        fs::read_to_string(log_dir.path().join("post_install_calls.log")).unwrap();
    assert!(
        post_install_log.contains("get-shit-done-cc --claude --local"),
        "expected post_install call with --claude --local, got: {post_install_log}"
    );

    // Assert .mcp.json does NOT exist (CLI type skips MCP config).
    let mcp_path = project_dir.path().join(".mcp.json");
    assert!(
        !mcp_path.exists(),
        ".mcp.json should NOT exist for CLI tool installs"
    );

    // Assert lockfile was created with the CLI tool entry.
    let lock_path = project_dir.path().join("claude-env.lock");
    assert!(lock_path.exists(), "claude-env.lock should exist");
    let lock_content = fs::read_to_string(&lock_path).unwrap();
    assert!(
        lock_content.contains("get-shit-done-cc"),
        "lockfile should contain get-shit-done-cc"
    );
    assert!(
        lock_content.contains("1.22.4"),
        "lockfile should contain version 1.22.4"
    );
}
