use claude_env::config::Config;
use claude_env::error::ConfigError;
use tempfile::NamedTempFile;
use std::io::Write;

// ---------------------------------------------------------------------------
// Parsing from string
// ---------------------------------------------------------------------------

#[test]
fn empty_string_yields_default_config() {
    let cfg = Config::from_str("").unwrap();
    assert!(cfg.mcp.is_empty());
    assert!(cfg.skills.is_empty());
    assert!(cfg.plugins.is_empty());
    assert!(cfg.cli.is_empty());
}

#[test]
fn minimal_config_only_mcp_section() {
    let toml = r#"
        [mcp]
        context7 = "latest"
    "#;
    let cfg = Config::from_str(toml).unwrap();
    assert_eq!(cfg.mcp.len(), 1);
    assert_eq!(cfg.mcp["context7"], "latest");
    assert!(cfg.skills.is_empty());
    assert!(cfg.plugins.is_empty());
    assert!(cfg.cli.is_empty());
}

#[test]
fn full_config_all_four_sections() {
    let toml = r#"
        [mcp]
        context7 = "latest"
        memory = "^1.0"

        [skills]
        my-skill = "latest"

        [plugins]
        my-plugin = "^2.3"

        [cli]
        some-tool = "1.2.3"
    "#;
    let cfg = Config::from_str(toml).unwrap();

    assert_eq!(cfg.mcp.len(), 2);
    assert_eq!(cfg.mcp["context7"], "latest");
    assert_eq!(cfg.mcp["memory"], "^1.0");

    assert_eq!(cfg.skills.len(), 1);
    assert_eq!(cfg.skills["my-skill"], "latest");

    assert_eq!(cfg.plugins.len(), 1);
    assert_eq!(cfg.plugins["my-plugin"], "^2.3");

    assert_eq!(cfg.cli.len(), 1);
    assert_eq!(cfg.cli["some-tool"], "1.2.3");
}

#[test]
fn invalid_toml_returns_parse_error() {
    let bad = "this is [not valid toml {{";
    let err = Config::from_str(bad).unwrap_err();
    assert!(
        matches!(err, ConfigError::Parse(_)),
        "expected ConfigError::Parse, got {err:?}"
    );
}

#[test]
fn unknown_top_level_section_returns_parse_error() {
    let toml = r#"
        [unknown_section]
        foo = "bar"
    "#;
    let err = Config::from_str(toml).unwrap_err();
    assert!(
        matches!(err, ConfigError::Parse(_)),
        "expected ConfigError::Parse for unknown section, got {err:?}"
    );
}

#[test]
fn unknown_sibling_field_in_known_section_still_errors() {
    // The mcp section values are BTreeMap<String,String> so any string value
    // is fine, but an unknown top-level key outside the four sections must fail.
    let toml = r#"
        [mcp]
        context7 = "latest"

        [extra]
        foo = "bar"
    "#;
    let err = Config::from_str(toml).unwrap_err();
    assert!(matches!(err, ConfigError::Parse(_)));
}

// ---------------------------------------------------------------------------
// Parsing from file
// ---------------------------------------------------------------------------

#[test]
fn from_file_reads_valid_toml() {
    let mut file = NamedTempFile::new().unwrap();
    writeln!(
        file,
        r#"
[mcp]
context7 = "latest"
"#
    )
    .unwrap();

    let cfg = Config::from_file(file.path()).unwrap();
    assert_eq!(cfg.mcp.len(), 1);
    assert_eq!(cfg.mcp["context7"], "latest");
}

#[test]
fn from_file_returns_io_error_for_missing_file() {
    let err = Config::from_file(std::path::Path::new("/nonexistent/path/claude-env.toml"))
        .unwrap_err();
    assert!(
        matches!(err, ConfigError::Io(_)),
        "expected ConfigError::Io, got {err:?}"
    );
}

// ---------------------------------------------------------------------------
// BTreeMap ordering guarantee (entries are sorted by key)
// ---------------------------------------------------------------------------

#[test]
fn mcp_entries_are_sorted_by_key() {
    let toml = r#"
        [mcp]
        zebra = "1"
        alpha = "2"
        mango = "3"
    "#;
    let cfg = Config::from_str(toml).unwrap();
    let keys: Vec<&str> = cfg.mcp.keys().map(String::as_str).collect();
    assert_eq!(keys, vec!["alpha", "mango", "zebra"]);
}
