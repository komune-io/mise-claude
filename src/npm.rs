use std::cmp::Ordering;
use std::collections::HashMap;

use crate::error::NpmError;

const DEFAULT_REGISTRY: &str = "https://registry.npmjs.org";

/// Client for querying the npm registry.
pub struct NpmClient {
    registry_url: String,
}

impl Default for NpmClient {
    fn default() -> Self {
        Self {
            registry_url: DEFAULT_REGISTRY.to_string(),
        }
    }
}

impl NpmClient {
    /// Create a client that uses a custom registry URL (useful for testing with a mock server).
    pub fn with_registry(url: &str) -> Self {
        Self {
            registry_url: url.to_string(),
        }
    }

    /// Fetch package metadata from the registry.
    ///
    /// # Errors
    /// Returns [`NpmError::Request`] if the HTTP request fails.
    /// Returns [`NpmError::Parse`] if the response cannot be parsed.
    pub fn fetch_metadata(&self, package: &str) -> Result<PackageMetadata, NpmError> {
        let url = format!("{}/{}", self.registry_url, package);
        let mut response = ureq::get(&url)
            .call()
            .map_err(|e| NpmError::Request(package.to_string(), e.to_string()))?;
        let body: serde_json::Value = response
            .body_mut()
            .read_json()
            .map_err(|e| NpmError::Request(package.to_string(), e.to_string()))?;
        PackageMetadata::from_json(body).map_err(|e| match e {
            NpmError::Parse(_, msg) => NpmError::Parse(package.to_string(), msg),
            other => other,
        })
    }
}

/// Parsed metadata for an npm package, holding version → integrity mappings.
pub struct PackageMetadata {
    /// Map of version string → sha512 integrity hash.
    versions: HashMap<String, Option<String>>,
}

impl PackageMetadata {
    /// Parse npm registry JSON into [`PackageMetadata`].
    ///
    /// Expects the standard npm registry response shape with a top-level `"versions"` object.
    pub fn from_json(json: serde_json::Value) -> Result<Self, NpmError> {
        let versions_obj = json
            .get("versions")
            .and_then(|v| v.as_object())
            .ok_or_else(|| {
                NpmError::Parse(
                    String::new(),
                    "missing or invalid 'versions' field".to_string(),
                )
            })?;

        let mut versions = HashMap::new();
        for (version, meta) in versions_obj {
            let integrity = meta
                .get("dist")
                .and_then(|d| d.get("integrity"))
                .and_then(|i| i.as_str())
                .map(|s| s.to_string());
            versions.insert(version.clone(), integrity);
        }

        Ok(Self { versions })
    }

    /// Return all non-prerelease versions, sorted by semver ascending.
    pub fn stable_versions(&self) -> Vec<&str> {
        let mut stable: Vec<&str> = self
            .versions
            .keys()
            .filter(|v| !is_prerelease(v))
            .map(|s| s.as_str())
            .collect();
        stable.sort_by(|a, b| compare_semver(a, b));
        stable
    }

    /// Return the highest stable version, or `None` if there are none.
    pub fn latest_stable(&self) -> Option<String> {
        self.stable_versions().last().map(|s| s.to_string())
    }

    /// Return the sha512 integrity hash for the given version, if available.
    pub fn integrity_for(&self, version: &str) -> Option<String> {
        self.versions.get(version)?.clone()
    }
}

/// Returns `true` if the version string represents a prerelease (contains `-`).
pub fn is_prerelease(version: &str) -> bool {
    version.contains('-')
}

/// Extract the major, minor, and patch numbers from a semver string.
/// Non-numeric components are treated as 0.
pub fn parse_semver_parts(v: &str) -> (u64, u64, u64) {
    let mut parts = v.splitn(3, '.');
    let major = parts
        .next()
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);
    let minor = parts
        .next()
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);
    // patch may have pre-release suffix; take numeric prefix only
    let patch_raw = parts.next().unwrap_or("0");
    let patch = patch_raw
        .split('-')
        .next()
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);
    (major, minor, patch)
}

/// Compare two version strings by their semver parts.
pub fn compare_semver(a: &str, b: &str) -> Ordering {
    parse_semver_parts(a).cmp(&parse_semver_parts(b))
}
