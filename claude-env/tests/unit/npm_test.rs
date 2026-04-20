use claude_env::npm::PackageMetadata;
use serde_json::json;

fn registry_json_with_versions(versions: &[(&str, Option<&str>)]) -> serde_json::Value {
    let mut vers = serde_json::Map::new();
    for (v, integrity) in versions {
        let dist = match integrity {
            Some(hash) => json!({ "dist": { "integrity": hash } }),
            None => json!({ "dist": {} }),
        };
        vers.insert(v.to_string(), dist);
    }
    json!({ "versions": vers })
}

#[test]
fn parse_registry_response() {
    let data = registry_json_with_versions(&[
        ("1.0.0", Some("sha512-aaa")),
        ("1.1.0", Some("sha512-bbb")),
        ("2.0.0-beta.1", Some("sha512-ccc")),
        ("2.0.0", Some("sha512-ddd")),
    ]);

    let meta = PackageMetadata::from_json(data).expect("should parse");
    let stable = meta.stable_versions();

    assert_eq!(stable, vec!["1.0.0", "1.1.0", "2.0.0"]);
}

#[test]
fn latest_stable_version() {
    let data = registry_json_with_versions(&[
        ("1.0.0", Some("sha512-aaa")),
        ("1.1.0", Some("sha512-bbb")),
        ("2.0.0-rc.1", Some("sha512-ccc")),
    ]);

    let meta = PackageMetadata::from_json(data).expect("should parse");
    assert_eq!(meta.latest_stable(), Some("1.1.0".to_string()));
}

#[test]
fn integrity_for_version() {
    let data = registry_json_with_versions(&[
        ("1.0.0", Some("sha512-abc123")),
        ("1.1.0", None),
    ]);

    let meta = PackageMetadata::from_json(data).expect("should parse");

    assert_eq!(
        meta.integrity_for("1.0.0"),
        Some("sha512-abc123".to_string())
    );
    // Version with no integrity field in dist
    assert_eq!(meta.integrity_for("1.1.0"), None);
    // Completely missing version
    assert_eq!(meta.integrity_for("9.9.9"), None);
}

#[test]
fn filter_prerelease_versions() {
    let data = registry_json_with_versions(&[
        ("1.0.0", None),
        ("1.0.1-alpha.1", None),
        ("1.1.0-beta.2", None),
        ("1.1.0-rc.1", None),
        ("1.1.0", None),
        ("2.0.0", None),
    ]);

    let meta = PackageMetadata::from_json(data).expect("should parse");
    let stable = meta.stable_versions();

    assert_eq!(stable, vec!["1.0.0", "1.1.0", "2.0.0"]);
    // Ensure no prerelease leaked through
    for v in &stable {
        assert!(
            !v.contains('-'),
            "prerelease version leaked into stable: {v}"
        );
    }
}
