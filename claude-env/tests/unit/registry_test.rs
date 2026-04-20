use claude_env::registry::Registry;

#[test]
fn resolve_known_alias() {
    let reg = Registry::default();
    assert_eq!(reg.resolve_alias("context7"), "@upstash/context7-mcp");
    assert_eq!(reg.resolve_alias("chrome-devtools"), "chrome-devtools-mcp");
    assert_eq!(reg.resolve_alias("shadcn"), "shadcn");
    assert_eq!(reg.resolve_alias("gsd"), "get-shit-done-cc");
    assert_eq!(reg.resolve_alias("bmad"), "bmad-method");
    assert_eq!(reg.resolve_alias("openspec"), "@fission-ai/openspec");
}

#[test]
fn unknown_alias_passes_through() {
    let reg = Registry::default();
    assert_eq!(reg.resolve_alias("@someorg/custom-mcp"), "@someorg/custom-mcp");
    assert_eq!(reg.resolve_alias("my-tool"), "my-tool");
}

#[test]
fn get_override_for_known_tool() {
    let reg = Registry::default();
    let ov = reg.get_override("shadcn").expect("shadcn should have an override");
    assert_eq!(ov.bin_name.as_deref(), Some("shadcn"));
    assert!(ov.post_install.as_deref().unwrap().contains("shadcn mcp init"));
    assert!(ov.extra_deps.contains(&"tinyexec@1.0.2".to_string()));
}

#[test]
fn get_override_returns_none_for_unknown() {
    let reg = Registry::default();
    assert!(reg.get_override("some-unknown-package").is_none());
}

#[test]
fn post_install_substitutes_project_root() {
    let reg = Registry::default();
    let ov = reg
        .get_override("bmad-method")
        .expect("bmad-method should have an override");
    let resolved = ov
        .resolve_post_install("/my/project")
        .expect("bmad-method should have a post_install");
    assert!(resolved.contains("/my/project"));
    assert!(!resolved.contains("${PROJECT_ROOT}"));
}
