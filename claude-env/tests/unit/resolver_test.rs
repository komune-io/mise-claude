use claude_env::config::Config;
use claude_env::lockfile::{Lockfile, LockedTool};
use claude_env::resolver::{resolve, Action};

#[test]
fn fresh_install_no_lockfile() {
    let config = Config::parse(
        r#"
        [mcp]
        context7 = "latest"
        "#,
    )
    .unwrap();
    let lockfile = Lockfile::new();

    let plan = resolve(&config, &lockfile, &|_section, _name| false);

    assert_eq!(plan.actions.len(), 1);
    assert_eq!(plan.actions[0].name, "context7");
    assert_eq!(plan.actions[0].action, Action::Install);
}

#[test]
fn skip_when_lockfile_matches_installed() {
    let config = Config::parse(
        r#"
        [mcp]
        context7 = "latest"
        "#,
    )
    .unwrap();

    let mut lockfile = Lockfile::new();
    lockfile.set(
        "mcp",
        "context7",
        LockedTool {
            package: None,
            version: "latest".to_string(),
            integrity: None,
            resolved_at: None,
        },
    );

    let plan = resolve(&config, &lockfile, &|_section, _name| true);

    assert_eq!(plan.actions.len(), 1);
    assert_eq!(plan.actions[0].action, Action::Skip);
}

#[test]
fn upgrade_when_config_version_differs_from_lock() {
    let config = Config::parse(
        r#"
        [mcp]
        context7 = "2.0.0"
        "#,
    )
    .unwrap();

    let mut lockfile = Lockfile::new();
    lockfile.set(
        "mcp",
        "context7",
        LockedTool {
            package: None,
            version: "1.0.0".to_string(),
            integrity: None,
            resolved_at: None,
        },
    );

    let plan = resolve(&config, &lockfile, &|_section, _name| true);

    assert_eq!(plan.actions.len(), 1);
    assert_eq!(plan.actions[0].action, Action::Upgrade);
}

#[test]
fn install_when_lockfile_matches_but_not_installed() {
    let config = Config::parse(
        r#"
        [mcp]
        context7 = "latest"
        "#,
    )
    .unwrap();

    let mut lockfile = Lockfile::new();
    lockfile.set(
        "mcp",
        "context7",
        LockedTool {
            package: None,
            version: "latest".to_string(),
            integrity: None,
            resolved_at: None,
        },
    );

    // Locked version matches config but tool is NOT installed on disk.
    let plan = resolve(&config, &lockfile, &|_section, _name| false);

    assert_eq!(plan.actions.len(), 1);
    assert_eq!(plan.actions[0].action, Action::Install);
}

#[test]
fn alias_is_resolved_to_package_name() {
    let config = Config::parse(
        r#"
        [mcp]
        context7 = "latest"
        "#,
    )
    .unwrap();
    let lockfile = Lockfile::new();

    let plan = resolve(&config, &lockfile, &|_section, _name| false);

    assert_eq!(plan.actions[0].package, "@upstash/context7-mcp");
}

#[test]
fn unknown_name_uses_name_as_package() {
    let config = Config::parse(
        r#"
        [mcp]
        some-unknown-tool = "1.2.3"
        "#,
    )
    .unwrap();
    let lockfile = Lockfile::new();

    let plan = resolve(&config, &lockfile, &|_section, _name| false);

    assert_eq!(plan.actions[0].package, "some-unknown-tool");
}
