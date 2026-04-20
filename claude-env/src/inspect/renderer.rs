// Renderer: formats the AuditReport for terminal or JSON output.

use crate::inspect::{AuditReport, Management, Scope};

/// Print the audit report to the terminal with colour-coded symbols.
pub fn render_terminal(report: &AuditReport) {
    for (category, entries) in &report.entries {
        if entries.is_empty() {
            continue;
        }

        println!("\n{}", category.label());

        for entry in entries {
            // Symbol
            let symbol = if entry.drift {
                "\x1b[33m⚠\x1b[0m"
            } else if entry.management == Management::Managed {
                "\x1b[32m✓\x1b[0m"
            } else {
                "●"
            };

            // Version
            let version = entry.version.as_deref().unwrap_or("—");

            // Scope
            let scope = if entry.drift {
                "MISSING".to_string()
            } else {
                match &entry.scope {
                    Some(Scope::Project) => "project".to_string(),
                    Some(Scope::Global) => "global".to_string(),
                    None => "—".to_string(),
                }
            };

            // Management label
            let management = if entry.drift {
                "declared in claude-env.toml but not installed"
            } else if entry.management == Management::Managed {
                "managed (claude-env.toml)"
            } else {
                "manual"
            };

            println!(
                "  {} {:<35} {:<8} {:<10} {}",
                symbol, entry.name, version, scope, management
            );

            // Path (only when present)
            if let Some(path) = &entry.path {
                println!("    → {}", path);
            }

            // Override annotation
            if let Some(overridden_by) = &entry.overridden_by {
                println!("    └─ overridden by {} config", overridden_by);
            }
        }
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
    println!("{}", serde_json::to_string_pretty(&output).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e)));
}
