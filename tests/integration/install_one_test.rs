use chord::operations::{install, OpContext, OperationError};
use std::fs;
use tempfile::TempDir;

#[test]
fn install_one_missing_tool_returns_not_found() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();

    fs::write(
        project.path().join("chord.toml"),
        "[mcp]\ncontext7 = \"latest\"\n",
    )
    .unwrap();

    let ctx = OpContext {
        project_root: project.path(),
        home_dir: home.path(),
        packages_dir: packages.path(),
        verbose: false,
    };

    let err = install::install_one("nope", &ctx, false).unwrap_err();
    assert!(matches!(err, OperationError::NotFound(_)));
}
