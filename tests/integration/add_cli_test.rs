use assert_cmd::Command;
use std::fs;
use tempfile::TempDir;

#[test]
fn add_unknown_section_exits_2() {
    let project = TempDir::new().unwrap();
    fs::write(project.path().join("chord.toml"), "").unwrap();

    let mut cmd = Command::cargo_bin("chord").unwrap();
    cmd.arg("add")
        .arg("bogus:foo@latest")
        .current_dir(project.path());

    cmd.assert().failure().code(2);
}

#[test]
fn add_duplicate_exits_2_without_mutating_toml() {
    let project = TempDir::new().unwrap();
    let original = "[mcp]\ncontext7 = \"latest\"\n";
    fs::write(project.path().join("chord.toml"), original).unwrap();

    let mut cmd = Command::cargo_bin("chord").unwrap();
    cmd.arg("add")
        .arg("mcp:context7@1.0.0")
        .current_dir(project.path());

    cmd.assert().failure().code(2);

    let after = fs::read_to_string(project.path().join("chord.toml")).unwrap();
    assert_eq!(after, original);
}
