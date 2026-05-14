use chord::operations::{install, OpContext};
use std::fs;
use tempfile::TempDir;

#[test]
fn install_all_with_empty_config_succeeds() {
    let project = TempDir::new().unwrap();
    let home = TempDir::new().unwrap();
    let packages = TempDir::new().unwrap();

    fs::write(project.path().join("chord.toml"), "").unwrap();

    let ctx = OpContext {
        project_root: project.path(),
        home_dir: home.path(),
        packages_dir: packages.path(),
        verbose: false,
    };

    let outcome = install::install_all(&ctx, false).unwrap();
    assert_eq!(outcome.installed, 0);
    assert_eq!(outcome.failed, 0);
}
