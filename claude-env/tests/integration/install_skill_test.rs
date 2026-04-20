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
fn install_skill() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    // Write claude-env.toml with a skill entry.
    fs::write(
        project_dir.path().join("claude-env.toml"),
        "[skills]\n\"vercel-labs/next-skills/next-best-practices\" = \"latest\"\n",
    )
    .unwrap();

    // Build PATH with shims first so our fake npx is found.
    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", shims_dir().display(), original_path);

    let mut cmd = Command::cargo_bin("claude-env").unwrap();
    cmd.arg("install")
        .current_dir(project_dir.path())
        .env("PATH", &new_path)
        .env("CLAUDE_ENV_HOME", packages_dir.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path());

    cmd.assert().success();

    // Assert npx was called with the correct arguments.
    let npx_log = fs::read_to_string(log_dir.path().join("npx_calls.log")).unwrap();
    assert!(
        npx_log.contains("skills add vercel-labs/next-skills"),
        "expected 'skills add vercel-labs/next-skills' in npx call, got: {npx_log}"
    );
    assert!(
        npx_log.contains("--skill next-best-practices"),
        "expected '--skill next-best-practices' in npx call, got: {npx_log}"
    );
    assert!(
        npx_log.contains("-a claude-code"),
        "expected '-a claude-code' in npx call, got: {npx_log}"
    );
    assert!(
        npx_log.contains("-y"),
        "expected '-y' in npx call, got: {npx_log}"
    );

    // Assert lockfile was created with the skill entry.
    let lock_path = project_dir.path().join("claude-env.lock");
    assert!(lock_path.exists(), "claude-env.lock should exist");
    let lock_content = fs::read_to_string(&lock_path).unwrap();
    assert!(
        lock_content.contains("next-best-practices"),
        "lockfile should contain next-best-practices"
    );
    assert!(
        lock_content.contains("latest"),
        "lockfile should contain version latest"
    );
}
