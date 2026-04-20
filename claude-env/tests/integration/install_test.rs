use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

fn shims_dir() -> std::path::PathBuf {
    // The shims directory is relative to the manifest, accessible via CARGO_MANIFEST_DIR.
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::path::PathBuf::from(manifest)
        .join("tests")
        .join("integration")
        .join("shims")
}

#[test]
fn install_single_mcp_tool() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    // Write claude-env.toml.
    fs::write(
        project_dir.path().join("claude-env.toml"),
        "[mcp]\ncontext7 = \"2.1.4\"\n",
    )
    .unwrap();

    // Build PATH with shims first.
    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", shims_dir().display(), original_path);

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("install")
        .current_dir(project_dir.path())
        .env("PATH", &new_path)
        .env("CLAUDE_ENV_HOME", packages_dir.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path());

    cmd.assert().success();

    // Assert npm was called with correct args.
    let npm_log = fs::read_to_string(log_dir.path().join("npm_calls.log")).unwrap();
    assert!(
        npm_log.contains("install"),
        "npm install should have been called"
    );
    assert!(
        npm_log.contains("@upstash/context7-mcp@2.1.4"),
        "expected versioned package in npm call, got: {npm_log}"
    );
    assert!(
        npm_log.contains("--prefix"),
        "expected --prefix in npm call"
    );

    // Assert .mcp.json was created with a context7 entry.
    let mcp_path = project_dir.path().join(".mcp.json");
    assert!(mcp_path.exists(), ".mcp.json should exist");
    let mcp_content = fs::read_to_string(&mcp_path).unwrap();
    let mcp_value: serde_json::Value = serde_json::from_str(&mcp_content).unwrap();
    assert!(
        mcp_value["mcpServers"]["context7"].is_object(),
        "context7 entry should exist in .mcp.json"
    );

    // Assert lockfile was created.
    let lock_path = project_dir.path().join("claude-env.lock");
    assert!(lock_path.exists(), "claude-env.lock should exist");
    let lock_content = fs::read_to_string(&lock_path).unwrap();
    assert!(
        lock_content.contains("context7"),
        "lockfile should contain context7"
    );
    assert!(
        lock_content.contains("2.1.4"),
        "lockfile should contain version 2.1.4"
    );
}

#[test]
fn install_idempotent_second_run_skips() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();
    let log_dir_1 = TempDir::new().unwrap();
    let log_dir_2 = TempDir::new().unwrap();

    fs::write(
        project_dir.path().join("claude-env.toml"),
        "[mcp]\ncontext7 = \"2.1.4\"\n",
    )
    .unwrap();

    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", shims_dir().display(), original_path);

    // First run.
    Command::cargo_bin("claude-env")
        .unwrap()
        .arg("install")
        .current_dir(project_dir.path())
        .env("PATH", &new_path)
        .env("CLAUDE_ENV_HOME", packages_dir.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir_1.path())
        .assert()
        .success();

    // Second run with a fresh log dir.
    Command::cargo_bin("claude-env")
        .unwrap()
        .arg("install")
        .current_dir(project_dir.path())
        .env("PATH", &new_path)
        .env("CLAUDE_ENV_HOME", packages_dir.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir_2.path())
        .assert()
        .success();

    // npm_calls.log should NOT exist in log_dir_2 (no npm calls on second run).
    let second_log = log_dir_2.path().join("npm_calls.log");
    assert!(
        !second_log.exists(),
        "npm should not be called on second (idempotent) run"
    );
}
