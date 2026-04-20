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
fn install_plugin() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    // Write claude-env.toml with a plugin entry.
    fs::write(
        project_dir.path().join("claude-env.toml"),
        "[plugins]\n\"anthropics/claude-code/code-review@claude-code-plugins\" = \"latest\"\n",
    )
    .unwrap();

    // Build PATH with shims first so our fake claude is found.
    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", shims_dir().display(), original_path);

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("install")
        .current_dir(project_dir.path())
        .env("PATH", &new_path)
        .env("CLAUDE_ENV_HOME", packages_dir.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path());

    cmd.assert().success();

    // Assert claude was called with the correct arguments.
    let claude_log =
        fs::read_to_string(log_dir.path().join("claude_calls.log")).unwrap();

    let lines: Vec<&str> = claude_log.lines().collect();

    assert!(
        lines.len() >= 2,
        "expected at least 2 claude calls, got: {claude_log}"
    );

    assert!(
        lines[0].contains("plugin marketplace add anthropics/claude-code"),
        "expected 'plugin marketplace add anthropics/claude-code' in first call, got: {}",
        lines[0]
    );

    assert!(
        lines[1].contains("plugin install code-review@claude-code-plugins"),
        "expected 'plugin install code-review@claude-code-plugins' in second call, got: {}",
        lines[1]
    );
    assert!(
        lines[1].contains("--scope project"),
        "expected '--scope project' in second call, got: {}",
        lines[1]
    );

    // Assert lockfile was created with the plugin entry.
    let lock_path = project_dir.path().join("claude-env.lock");
    assert!(lock_path.exists(), "claude-env.lock should exist");
    let lock_content = fs::read_to_string(&lock_path).unwrap();
    assert!(
        lock_content.contains("code-review"),
        "lockfile should contain code-review"
    );
    assert!(
        lock_content.contains("latest"),
        "lockfile should contain version latest"
    );
}
