// Renderer: formats the AuditReport for terminal or JSON output.

use crate::core::inspect::{AuditReport, Management, Scope};

const DIM: &str = "\x1b[90m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BOLD: &str = "\x1b[1m";
const RESET: &str = "\x1b[0m";

/// Print the audit report to the terminal with colour-coded symbols.
pub fn render_terminal(report: &AuditReport) {
    let mut first = true;
    for (category, entries) in &report.entries {
        let visible: Vec<_> = entries
            .iter()
            .filter(|e| e.enabled || e.management == Management::Managed || e.drift)
            .collect();
        if visible.is_empty() {
            continue;
        }

        if !first {
            println!();
        }
        first = false;

        println!("{BOLD}{}{RESET}", category.label());

        // Group entries by source path to reduce noise
        let mut last_source: Option<String> = None;

        for entry in entries {
            // Static output: only show enabled/managed/drift items by default
            if !entry.enabled && entry.management != Management::Managed && !entry.drift {
                continue;
            }

            let symbol = if entry.drift {
                format!("{YELLOW}⚠{RESET}")
            } else if entry.management == Management::Managed {
                format!("{GREEN}✓{RESET}")
            } else if entry.enabled {
                format!("{GREEN}●{RESET}")
            } else {
                format!("{DIM}●{RESET}")
            };

            let scope_tag = if entry.drift {
                format!("{YELLOW}missing{RESET}")
            } else {
                match &entry.scope {
                    Some(Scope::Project) => format!("{GREEN}project{RESET}"),
                    Some(Scope::Global) => format!("{DIM}global{RESET}"),
                    None => String::new(),
                }
            };

            let mgmt_tag = if entry.drift {
                format!("{YELLOW}not installed{RESET}")
            } else if entry.management == Management::Managed {
                format!("{DIM}managed{RESET}")
            } else {
                String::new()
            };

            // Show source group header when it changes. Three group keys,
            // in priority order:
            //   1. plugin-cache items group under "plugin <name>@<marketplace>"
            //   2. skill repos installed via the git-native installer group under
            //      "skills from <owner>/<repo>"
            //   3. everything else groups under its own file path
            let current_source = entry
                .from_plugin
                .as_deref()
                .map(|p| format!("plugin {}", p))
                .or_else(|| {
                    entry
                        .source_repo
                        .as_deref()
                        .map(|s| format!("skills from {}", s))
                })
                .or_else(|| entry.path.clone());
            if current_source.as_deref() != last_source.as_deref() {
                if let Some(label) = &current_source {
                    println!("  {DIM}{label}{RESET}");
                }
                last_source = current_source;
            }

            // Main line: symbol + name + optional version
            let version_str = entry
                .version
                .as_ref()
                .map(|v| format!(" {DIM}@{v}{RESET}"))
                .unwrap_or_default();

            let tags = [scope_tag, mgmt_tag]
                .into_iter()
                .filter(|t| !t.is_empty())
                .collect::<Vec<_>>()
                .join(" ");

            let tags_str = if tags.is_empty() {
                String::new()
            } else {
                format!(" {DIM}({RESET}{tags}{DIM}){RESET}")
            };

            println!("    {symbol} {}{version_str}{tags_str}", entry.name);

            if let Some(overridden_by) = &entry.overridden_by {
                println!("      {DIM}└─ overridden by {overridden_by}{RESET}");
            }
        }
    }

    if first {
        println!("{DIM}No Claude Code configuration found.{RESET}");
    }
}

/// Print the audit report as pretty-printed JSON to stdout.
pub fn render_json(report: &AuditReport) {
    let mut map = serde_json::Map::new();

    for (category, entries) in &report.entries {
        let json_entries: Vec<serde_json::Value> = entries
            .iter()
            .map(|entry| {
                let scope_str = if entry.drift {
                    serde_json::Value::String("MISSING".to_string())
                } else {
                    match &entry.scope {
                        Some(Scope::Project) => serde_json::Value::String("project".to_string()),
                        Some(Scope::Global) => serde_json::Value::String("global".to_string()),
                        None => serde_json::Value::Null,
                    }
                };

                let source_str = if entry.management == Management::Managed {
                    "managed"
                } else {
                    "manual"
                };

                serde_json::json!({
                    "name": entry.name,
                    "version": entry.version,
                    "scope": scope_str,
                    "source": source_str,
                    "path": entry.path,
                    "drift": entry.drift,
                    "enabled": entry.enabled,
                    "overridden_by": entry.overridden_by,
                })
            })
            .collect();

        map.insert(
            category.cli_name().to_string(),
            serde_json::Value::Array(json_entries),
        );
    }

    let output = serde_json::Value::Object(map);
    println!(
        "{}",
        serde_json::to_string_pretty(&output)
            .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
    );
}
