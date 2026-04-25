// Scanner: discovers installed Claude Code configuration from project and global scopes.

use std::fs;
use std::path::Path;

use serde_json::Value;

use std::collections::HashSet;

use super::{DiscoveredItem, Scope};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Read and parse a JSON file. Returns None on any error (missing, invalid, etc).
fn read_json(path: &Path) -> Option<Value> {
    let contents = fs::read_to_string(path).ok()?;
    serde_json::from_str(&contents).ok()
}

/// Extract keys from `json["mcpServers"]` as DiscoveredItems.
fn extract_mcp_servers(json: &Value, scope: Scope, source: &str) -> Vec<DiscoveredItem> {
    let servers = match json.get("mcpServers").and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return vec![],
    };
    servers
        .keys()
        .map(|name| DiscoveredItem {
            name: name.clone(),
            version: None,
            scope: scope.clone(),
            source_path: source.to_string(),
            from_plugin: None,
        })
        .collect()
}

/// Extract keys from `json["enabledPlugins"]` as DiscoveredItems.
fn extract_plugins(json: &Value, scope: Scope, source: &str) -> Vec<DiscoveredItem> {
    let plugins = match json.get("enabledPlugins").and_then(|v| v.as_object()) {
        Some(obj) => obj,
        None => return vec![],
    };
    plugins
        .keys()
        .map(|name| DiscoveredItem {
            name: name.clone(),
            version: None,
            scope: scope.clone(),
            source_path: source.to_string(),
            from_plugin: None,
        })
        .collect()
}

/// Collect all enabled plugin identifiers from project + global settings.
/// Returns set of "plugin@marketplace" strings.
pub fn collect_enabled_plugins(project_root: &Path, home_dir: &Path) -> HashSet<String> {
    let mut enabled = HashSet::new();

    for settings_path in [
        project_root.join(".claude").join("settings.json"),
        home_dir.join(".claude").join("settings.json"),
    ] {
        if let Some(json) = read_json(&settings_path) {
            if let Some(plugins) = json.get("enabledPlugins").and_then(|v| v.as_object()) {
                for key in plugins.keys() {
                    enabled.insert(key.clone());
                }
            }
        }
    }

    enabled
}

/// Scan `dir/*/marker` — each subdirectory that contains `marker` yields one item.
/// The item name is the subdirectory name; source is the path to the marker file.
fn scan_md_dirs(
    dir: &Path,
    scope: Scope,
    prefix: &str,
    marker: &str,
    items: &mut Vec<DiscoveredItem>,
) {
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    for entry in read_dir.flatten() {
        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }
        let marker_path = entry_path.join(marker);
        if !marker_path.is_file() {
            continue;
        }
        let dir_name = match entry_path.file_name().and_then(|n| n.to_str()) {
            Some(n) => n.to_string(),
            None => continue,
        };
        let name = if prefix.is_empty() {
            dir_name
        } else {
            format!("{}/{}", prefix, dir_name)
        };
        items.push(DiscoveredItem {
            name,
            version: None,
            scope: scope.clone(),
            source_path: marker_path.to_string_lossy().into_owned(),
            from_plugin: None,
        });
    }
}

/// Scan `dir` recursively for `*.md` files. Each file yields one item.
/// The item name is the file stem; source is the path to the file.
fn scan_md_files_recursive(dir: &Path, scope: Scope, prefix: &str, items: &mut Vec<DiscoveredItem>) {
    scan_md_files_in_dir(dir, dir, scope, prefix, items, true);
}

/// Scan `dir` (non-recursively) for `*.md` files. Each file yields one item.
fn scan_md_files_flat(dir: &Path, scope: Scope, prefix: &str, items: &mut Vec<DiscoveredItem>) {
    scan_md_files_in_dir(dir, dir, scope, prefix, items, false);
}

fn scan_md_files_in_dir(
    root: &Path,
    dir: &Path,
    scope: Scope,
    prefix: &str,
    items: &mut Vec<DiscoveredItem>,
    recursive: bool,
) {
    let read_dir = match fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };
    for entry in read_dir.flatten() {
        let entry_path = entry.path();
        if entry_path.is_dir() {
            if recursive {
                scan_md_files_in_dir(root, &entry_path, scope.clone(), prefix, items, true);
            }
            continue;
        }
        if entry_path.extension().and_then(|e| e.to_str()) != Some("md") {
            continue;
        }
        let stem = match entry_path.file_stem().and_then(|s| s.to_str()) {
            Some(s) => s.to_string(),
            None => continue,
        };
        // Build name: for recursive, use relative path from root dir (without extension)
        let name = if recursive && entry_path.parent() != Some(root) {
            // Relative path from root, without .md extension
            let rel = entry_path.strip_prefix(root).unwrap_or(&entry_path);
            let rel_str = rel.to_string_lossy();
            // Remove .md suffix
            let rel_no_ext = rel_str.trim_end_matches(".md");
            if prefix.is_empty() {
                rel_no_ext.to_string()
            } else {
                format!("{}/{}", prefix, rel_no_ext)
            }
        } else {
            if prefix.is_empty() {
                stem
            } else {
                format!("{}/{}", prefix, stem)
            }
        };
        items.push(DiscoveredItem {
            name,
            version: None,
            scope: scope.clone(),
            source_path: entry_path.to_string_lossy().into_owned(),
            from_plugin: None,
        });
    }
}

/// Walk `~/.claude/plugins/cache/<marketplace>/<plugin>/<version>/` and scan
/// the given `subdir` (e.g., "skills", "commands", "agents") inside each plugin.
///
/// For skills: looks for `*/SKILL.md` (marker-based).
/// For commands: looks for `**/*.md` (recursive).
/// For agents: looks for `*.md` (flat).
fn scan_plugin_cache(
    home_dir: &Path,
    subdir: &str,
    mode: ScanMode,
    items: &mut Vec<DiscoveredItem>,
) {
    let cache_dir = home_dir.join(".claude").join("plugins").join("cache");
    let marketplaces = match fs::read_dir(&cache_dir) {
        Ok(rd) => rd,
        Err(_) => return,
    };

    for mp_entry in marketplaces.flatten() {
        let mp_path = mp_entry.path();
        if !mp_path.is_dir() {
            continue;
        }
        let mp_name = mp_entry.file_name().to_string_lossy().to_string();

        let plugins = match fs::read_dir(&mp_path) {
            Ok(rd) => rd,
            Err(_) => continue,
        };

        for plugin_entry in plugins.flatten() {
            let plugin_path = plugin_entry.path();
            if !plugin_path.is_dir() {
                continue;
            }
            let plugin_name = plugin_entry.file_name().to_string_lossy().to_string();

            let version_dir = match latest_version_dir(&plugin_path) {
                Some(d) => d,
                None => continue,
            };

            let target_dir = version_dir.join(subdir);
            if !target_dir.is_dir() {
                continue;
            }

            // Use plugin identifier as source (for grouping in renderer)
            let source = format!("plugin {}@{}", plugin_name, mp_name);

            let mut plugin_items = Vec::new();
            match mode {
                ScanMode::SkillMarker => {
                    scan_md_dirs(&target_dir, Scope::Global, "", "SKILL.md", &mut plugin_items);
                }
                ScanMode::Recursive => {
                    scan_md_files_recursive(&target_dir, Scope::Global, "", &mut plugin_items);
                }
                ScanMode::Flat => {
                    scan_md_files_flat(&target_dir, Scope::Global, "", &mut plugin_items);
                }
            }

            // Tag items with plugin origin for grouping and enabled detection
            let plugin_id = format!("{}@{}", plugin_name, mp_name);
            for item in &mut plugin_items {
                item.source_path = source.clone();
                item.from_plugin = Some(plugin_id.clone());
            }
            items.extend(plugin_items);
        }
    }
}

enum ScanMode {
    SkillMarker,
    Recursive,
    Flat,
}

/// Pick the latest version directory inside a plugin dir.
/// Sorts entries and takes the last (works for semver and hash-based names).
fn latest_version_dir(plugin_dir: &Path) -> Option<std::path::PathBuf> {
    let mut versions: Vec<_> = fs::read_dir(plugin_dir)
        .ok()?
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    versions.sort_by_key(|e| e.file_name());
    versions.last().map(|e| e.path())
}

// ---------------------------------------------------------------------------
// Public scanners
// ---------------------------------------------------------------------------

/// Scan for MCP servers from project `.mcp.json` and global `~/.claude/settings.json`.
pub fn scan_mcp(project_root: &Path, home_dir: &Path) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();

    // Project scope: .mcp.json
    let project_mcp = project_root.join(".mcp.json");
    if let Some(json) = read_json(&project_mcp) {
        let source = project_mcp.to_string_lossy().into_owned();
        items.extend(extract_mcp_servers(&json, Scope::Project, &source));
    }

    // Global scope: ~/.claude/settings.json
    let global_settings = home_dir.join(".claude").join("settings.json");
    if let Some(json) = read_json(&global_settings) {
        let source = global_settings.to_string_lossy().into_owned();
        items.extend(extract_mcp_servers(&json, Scope::Global, &source));
    }

    items
}

/// Scan for plugins from project `.claude/settings.json` and global `~/.claude/settings.json`.
pub fn scan_plugins(project_root: &Path, home_dir: &Path) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();

    // Project scope: .claude/settings.json
    let project_settings = project_root.join(".claude").join("settings.json");
    if let Some(json) = read_json(&project_settings) {
        let source = project_settings.to_string_lossy().into_owned();
        items.extend(extract_plugins(&json, Scope::Project, &source));
    }

    // Global scope: ~/.claude/settings.json
    let global_settings = home_dir.join(".claude").join("settings.json");
    if let Some(json) = read_json(&global_settings) {
        let source = global_settings.to_string_lossy().into_owned();
        items.extend(extract_plugins(&json, Scope::Global, &source));
    }

    items
}

/// Scan for skills from `.claude/skills/*/SKILL.md` (project) and `~/.claude/skills/*/SKILL.md` (global).
pub fn scan_skills(project_root: &Path, home_dir: &Path) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();

    let project_skills_dir = project_root.join(".claude").join("skills");
    scan_md_dirs(&project_skills_dir, Scope::Project, "", "SKILL.md", &mut items);

    let global_skills_dir = home_dir.join(".claude").join("skills");
    scan_md_dirs(&global_skills_dir, Scope::Global, "", "SKILL.md", &mut items);

    // Plugin cache: ~/.claude/plugins/cache/<mp>/<plugin>/<ver>/skills/
    scan_plugin_cache(home_dir, "skills", ScanMode::SkillMarker, &mut items);

    items
}

/// Scan for commands from `.claude/commands/**/*.md` (project) and `~/.claude/commands/**/*.md` (global).
pub fn scan_commands(project_root: &Path, home_dir: &Path) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();

    let project_commands_dir = project_root.join(".claude").join("commands");
    scan_md_files_recursive(&project_commands_dir, Scope::Project, "", &mut items);

    let global_commands_dir = home_dir.join(".claude").join("commands");
    scan_md_files_recursive(&global_commands_dir, Scope::Global, "", &mut items);

    // Plugin cache: ~/.claude/plugins/cache/<mp>/<plugin>/<ver>/commands/
    scan_plugin_cache(home_dir, "commands", ScanMode::Recursive, &mut items);

    items
}

/// Scan for agents from `.claude/agents/*.md` (project, flat) and `~/.claude/agents/*.md` (global, flat).
pub fn scan_agents(project_root: &Path, home_dir: &Path) -> Vec<DiscoveredItem> {
    let mut items = Vec::new();

    let project_agents_dir = project_root.join(".claude").join("agents");
    scan_md_files_flat(&project_agents_dir, Scope::Project, "", &mut items);

    let global_agents_dir = home_dir.join(".claude").join("agents");
    scan_md_files_flat(&global_agents_dir, Scope::Global, "", &mut items);

    // Plugin cache: ~/.claude/plugins/cache/<mp>/<plugin>/<ver>/agents/
    scan_plugin_cache(home_dir, "agents", ScanMode::Flat, &mut items);

    items
}
