use assert_cmd::Command;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use tempfile::TempDir;

fn shims_dir() -> std::path::PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::path::PathBuf::from(manifest)
        .join("tests")
        .join("integration")
        .join("shims")
}

fn fixtures_dir() -> std::path::PathBuf {
    let manifest = env!("CARGO_MANIFEST_DIR");
    std::path::PathBuf::from(manifest)
        .join("tests")
        .join("integration")
        .join("fixtures")
}

#[test]
fn full_install_all_tool_types() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    // Read fixture and write to project dir.
    let fixture = fs::read_to_string(fixtures_dir().join("full_config.toml")).unwrap();
    fs::write(project_dir.path().join("claude-env.toml"), &fixture).unwrap();

    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!("{}:{}", shims_dir().display(), original_path);

    let output = Command::cargo_bin("claude-env")
        .unwrap()
        .arg("install")
        .current_dir(project_dir.path())
        .env("PATH", &new_path)
        .env("CLAUDE_ENV_HOME", packages_dir.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path())
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        output.status.success(),
        "expected exit 0, got: {}\nstdout: {}\nstderr: {}",
        output.status,
        stdout,
        String::from_utf8_lossy(&output.stderr)
    );

    // Assert summary line: 4 installed, 0 failed, 0 skipped.
    assert!(
        stdout.contains("4 installed, 0 failed, 0 skipped"),
        "expected '4 installed, 0 failed, 0 skipped' in stdout, got: {stdout}"
    );

    // Assert .mcp.json was created (MCP tool).
    let mcp_path = project_dir.path().join(".mcp.json");
    assert!(mcp_path.exists(), ".mcp.json should exist after full install");

    // Assert npx was called with 'skills add' (skill tool).
    let npx_log = fs::read_to_string(log_dir.path().join("npx_calls.log")).unwrap();
    assert!(
        npx_log.contains("skills add"),
        "npx_calls.log should contain 'skills add', got: {npx_log}"
    );

    // Assert claude was called for plugin marketplace add and plugin install.
    let claude_log = fs::read_to_string(log_dir.path().join("claude_calls.log")).unwrap();
    assert!(
        claude_log.contains("plugin marketplace add"),
        "claude_calls.log should contain 'plugin marketplace add', got: {claude_log}"
    );
    assert!(
        claude_log.contains("plugin install"),
        "claude_calls.log should contain 'plugin install', got: {claude_log}"
    );

    // Assert npm was called with the CLI tool.
    let npm_log = fs::read_to_string(log_dir.path().join("npm_calls.log")).unwrap();
    assert!(
        npm_log.contains("get-shit-done-cc"),
        "npm_calls.log should contain 'get-shit-done-cc', got: {npm_log}"
    );

    // Assert lockfile was created with entries for all sections.
    let lock_path = project_dir.path().join("claude-env.lock");
    assert!(lock_path.exists(), "claude-env.lock should exist");
    let lock_content = fs::read_to_string(&lock_path).unwrap();

    assert!(
        lock_content.contains("context7"),
        "lockfile should contain context7"
    );
    assert!(
        lock_content.contains("next-best-practices"),
        "lockfile should contain next-best-practices"
    );
    assert!(
        lock_content.contains("code-review"),
        "lockfile should contain code-review"
    );
    assert!(
        lock_content.contains("get-shit-done-cc"),
        "lockfile should contain get-shit-done-cc"
    );
}

#[test]
fn partial_failure_exits_with_code_1() {
    let project_dir = TempDir::new().unwrap();
    let packages_dir = TempDir::new().unwrap();
    let log_dir = TempDir::new().unwrap();

    // Config with a valid MCP tool + a broken CLI tool (nonexistent package).
    // We use a custom npm shim that fails for "broken-tool" but succeeds for context7.
    fs::write(
        project_dir.path().join("claude-env.toml"),
        "[mcp]\ncontext7 = \"2.1.4\"\n\n[cli]\nbroken-tool = \"9.9.9\"\n",
    )
    .unwrap();

    // Create a custom shims dir with a patched npm that fails for broken-tool.
    let custom_shims_dir = TempDir::new().unwrap();

    // Write a custom npm shim that exits 1 for broken-tool.
    let custom_npm = custom_shims_dir.path().join("npm");
    fs::write(
        &custom_npm,
        r#"#!/bin/bash
echo "$@" >> "${CLAUDE_ENV_TEST_LOG}/npm_calls.log"
if [[ "$@" == *"broken-tool"* ]]; then
    echo "npm ERR! 404 Not Found: broken-tool@9.9.9" >&2
    exit 1
fi
if [[ "$1" == "install" ]]; then
    prefix=""
    prev=""
    for arg in "$@"; do
        if [[ "$prev" == "--prefix" ]]; then
            prefix="$arg"
        fi
        prev="$arg"
    done
    if [[ -n "$prefix" ]]; then
        mkdir -p "$prefix/node_modules/.bin"
        pkg=$(echo "$2" | sed 's/@[^/]*$//' | sed 's/.*\///')
        cat > "$prefix/node_modules/.bin/$pkg" <<'SHIM'
#!/bin/bash
echo "$0 $@" >> "${CLAUDE_ENV_TEST_LOG}/post_install_calls.log"
exit 0
SHIM
        chmod +x "$prefix/node_modules/.bin/$pkg"
    fi
fi
exit 0
"#,
    )
    .unwrap();
    fs::set_permissions(&custom_npm, fs::Permissions::from_mode(0o755)).unwrap();

    // Symlink the standard npx, claude, and get-shit-done-cc shims.
    let std_shims = shims_dir();
    for shim_name in &["npx", "claude", "get-shit-done-cc"] {
        std::os::unix::fs::symlink(
            std_shims.join(shim_name),
            custom_shims_dir.path().join(shim_name),
        )
        .unwrap();
    }

    let original_path = std::env::var("PATH").unwrap_or_default();
    let new_path = format!(
        "{}:{}",
        custom_shims_dir.path().display(),
        original_path
    );

    let output = Command::cargo_bin("claude-env")
        .unwrap()
        .arg("install")
        .current_dir(project_dir.path())
        .env("PATH", &new_path)
        .env("CLAUDE_ENV_HOME", packages_dir.path())
        .env("CLAUDE_ENV_TEST_LOG", log_dir.path())
        .output()
        .unwrap();

    // Assert exit code 1 due to partial failure.
    assert_eq!(
        output.status.code(),
        Some(1),
        "expected exit code 1, got: {:?}\nstdout: {}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Assert summary shows 1 installed + 1 failed.
    assert!(
        stdout.contains("1 installed, 1 failed, 0 skipped"),
        "expected '1 installed, 1 failed, 0 skipped' in stdout, got: {stdout}"
    );

    // Assert context7 (valid tool) was still installed.
    let mcp_path = project_dir.path().join(".mcp.json");
    assert!(
        mcp_path.exists(),
        ".mcp.json should exist (context7 was installed successfully)"
    );

    let lock_path = project_dir.path().join("claude-env.lock");
    assert!(lock_path.exists(), "lockfile should exist");
    let lock_content = fs::read_to_string(&lock_path).unwrap();
    assert!(
        lock_content.contains("context7"),
        "lockfile should contain context7 (partial success)"
    );
}
