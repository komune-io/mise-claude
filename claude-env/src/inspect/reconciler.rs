// Reconciler: compares discovered items against declared config to detect drift.

use std::collections::{HashMap, HashSet};

use crate::config::Config;
use crate::inspect::{AuditEntry, Category, DiscoveredItem, Management, Scope};
use crate::registry::Registry;

/// Derive the set of names that a config key can match against discovered item names.
///
/// - **MCP**: friendly name + resolved package + bare package (without scope prefix).
/// - **Plugins**: short form `"<plugin>@<marketplace>"` + the full key.
/// - **Skills**: leaf segment (last `/`-separated part) + the full key.
/// - Everything else: just the key itself.
fn match_names_for(category: &Category, key: &str, registry: &Registry) -> Vec<String> {
    match category {
        Category::Mcp => {
            let resolved = registry.resolve_alias(key);
            let mut names = vec![key.to_string(), resolved.to_string()];
            // e.g. "@upstash/context7-mcp" → bare "context7-mcp"
            if let Some(bare) = resolved.strip_prefix('@').and_then(|s| s.split_once('/').map(|(_, b)| b)) {
                names.push(bare.to_string());
            }
            names
        }
        Category::Plugins => {
            // key like "anthropics/claude-code/code-review@claude-code-plugins"
            // short form: last segment before the final '/' is "<plugin>@<marketplace>"
            let short = key
                .rsplit('/')
                .next()
                .unwrap_or(key)
                .to_string();
            let mut names = vec![short];
            if names[0] != key {
                names.push(key.to_string());
            }
            names
        }
        Category::Skills => {
            // key like "vercel-labs/next-skills/next-best-practices"
            let leaf = key
                .rsplit('/')
                .next()
                .unwrap_or(key)
                .to_string();
            let mut names = vec![leaf];
            if names[0] != key {
                names.push(key.to_string());
            }
            names
        }
        // Commands and Agents have no config section; use key as-is.
        _ => vec![key.to_string()],
    }
}

/// Cross-reference `discovered` items with the `config` for `category`.
///
/// Returns a vec of `AuditEntry` that includes:
/// - Discovered items tagged as `Managed` or `Manual`.
/// - Config entries that were never discovered (drift entries).
/// - Override annotations when the same name appears at both scopes.
pub fn reconcile(
    category: Category,
    discovered: &[DiscoveredItem],
    config: &Config,
) -> Vec<AuditEntry> {
    let registry = Registry::default();
    let section = category.cli_name();

    // Build a mapping: config_key → Vec<possible match names>
    // Only sections that exist in Config (mcp, cli, skills, plugins) are relevant.
    let config_section: &std::collections::BTreeMap<String, String> = match section {
        "mcp" => &config.mcp,
        "cli" => &config.cli,
        "skills" => &config.skills,
        "plugins" => &config.plugins,
        _ => {
            // Commands / Agents: no config section, everything is Manual.
            return discovered
                .iter()
                .map(|item| AuditEntry {
                    name: item.name.clone(),
                    version: item.version.clone(),
                    scope: Some(item.scope.clone()),
                    management: Management::Manual,
                    path: Some(item.source_path.clone()),
                    drift: false,
                    overridden_by: None,
                })
                .collect();
        }
    };

    // config_key → (match_names, matched: bool)
    let mut config_entries: Vec<(String, Vec<String>, bool)> = config_section
        .keys()
        .map(|k| {
            let names = match_names_for(&category, k, &registry);
            (k.clone(), names, false)
        })
        .collect();

    // Build lookup: each possible match name → index into config_entries
    let mut name_to_config: HashMap<String, usize> = HashMap::new();
    for (idx, (_, names, _)) in config_entries.iter().enumerate() {
        for n in names {
            name_to_config.insert(n.clone(), idx);
        }
    }

    // Process discovered items
    let mut entries: Vec<AuditEntry> = Vec::new();

    for item in discovered {
        let management = if let Some(&idx) = name_to_config.get(&item.name) {
            config_entries[idx].2 = true; // mark matched
            Management::Managed
        } else {
            Management::Manual
        };

        entries.push(AuditEntry {
            name: item.name.clone(),
            version: item.version.clone(),
            scope: Some(item.scope.clone()),
            management,
            path: Some(item.source_path.clone()),
            drift: false,
            overridden_by: None,
        });
    }

    // Unmatched config entries → drift
    for (key, _, matched) in &config_entries {
        if !matched {
            entries.push(AuditEntry {
                name: key.clone(),
                version: None,
                scope: None,
                path: None,
                management: Management::Managed,
                drift: true,
                overridden_by: None,
            });
        }
    }

    // Override detection: same name at both Project and Global → mark Global entry.
    // Build a set of names that appear at Project scope.
    let project_names: HashSet<String> = entries
        .iter()
        .filter(|e| e.scope == Some(Scope::Project))
        .map(|e| e.name.clone())
        .collect();

    for entry in &mut entries {
        if entry.scope == Some(Scope::Global) && project_names.contains(&entry.name) {
            entry.overridden_by = Some("project".to_string());
        }
    }

    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_item(name: &str, scope: Scope) -> DiscoveredItem {
        DiscoveredItem {
            name: name.to_string(),
            version: Some("1.0.0".to_string()),
            scope,
            source_path: "/some/path".to_string(),
        }
    }
}
