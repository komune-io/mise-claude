use serde_json::{json, Value};
use std::path::Path;

/// Represents an MCP stdio server entry.
pub struct McpEntry {
    pub command: String,
    pub args: Vec<String>,
}

/// Read .mcp.json from `project_root`, returning a parsed JSON Value.
/// If the file does not exist, returns an empty `{"mcpServers": {}}` object.
fn read_mcp_json(project_root: &Path) -> std::io::Result<Value> {
    let path = project_root.join(".mcp.json");
    if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        let value: Value = serde_json::from_str(&content)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        Ok(value)
    } else {
        Ok(json!({ "mcpServers": {} }))
    }
}

/// Write a JSON Value to .mcp.json in `project_root`, pretty-printed.
fn write_mcp_json(project_root: &Path, value: &Value) -> std::io::Result<()> {
    let path = project_root.join(".mcp.json");
    let content = serde_json::to_string_pretty(value)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
    std::fs::write(path, content)
}

/// Ensure a server entry exists in `.mcp.json`.
///
/// Returns `true` if the entry was added, `false` if it already existed.
pub fn ensure_server(project_root: &Path, name: &str, entry: &McpEntry) -> std::io::Result<bool> {
    let mut root = read_mcp_json(project_root)?;

    let servers = root
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "missing or invalid 'mcpServers' key",
            )
        })?;

    if servers.contains_key(name) {
        return Ok(false);
    }

    servers.insert(
        name.to_string(),
        json!({
            "type": "stdio",
            "command": entry.command,
            "args": entry.args,
        }),
    );

    write_mcp_json(project_root, &root)?;
    Ok(true)
}

/// Remove a server entry from `.mcp.json`.
///
/// If the file or the entry does not exist, this is a no-op.
pub fn remove_server(project_root: &Path, name: &str) -> std::io::Result<()> {
    let path = project_root.join(".mcp.json");
    if !path.exists() {
        return Ok(());
    }

    let mut root = read_mcp_json(project_root)?;

    let servers = root
        .get_mut("mcpServers")
        .and_then(|v| v.as_object_mut())
        .ok_or_else(|| {
            std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "missing or invalid 'mcpServers' key",
            )
        })?;

    servers.remove(name);

    write_mcp_json(project_root, &root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn make_entry(command: &str, args: &[&str]) -> McpEntry {
        McpEntry {
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
        }
    }

    #[test]
    fn write_server_to_empty_mcp_json() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        let added = ensure_server(root, "my-server", &make_entry("npx", &["-y", "my-pkg"])).unwrap();
        assert!(added, "expected true when adding to empty file");

        let content = std::fs::read_to_string(root.join(".mcp.json")).unwrap();
        let value: Value = serde_json::from_str(&content).unwrap();

        let server = &value["mcpServers"]["my-server"];
        assert_eq!(server["type"], "stdio");
        assert_eq!(server["command"], "npx");
        assert_eq!(server["args"], json!(["-y", "my-pkg"]));
    }

    #[test]
    fn merge_server_into_existing_mcp_json() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        ensure_server(root, "server-a", &make_entry("cmd-a", &["arg1"])).unwrap();
        let added = ensure_server(root, "server-b", &make_entry("cmd-b", &["arg2"])).unwrap();
        assert!(added, "expected true when adding a new server to existing file");

        let content = std::fs::read_to_string(root.join(".mcp.json")).unwrap();
        let value: Value = serde_json::from_str(&content).unwrap();

        assert_eq!(value["mcpServers"]["server-a"]["command"], "cmd-a");
        assert_eq!(value["mcpServers"]["server-b"]["command"], "cmd-b");
    }

    #[test]
    fn skip_if_server_already_exists() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        let first = ensure_server(root, "dup", &make_entry("npx", &["pkg"])).unwrap();
        assert!(first);

        let second = ensure_server(root, "dup", &make_entry("npx", &["pkg"])).unwrap();
        assert!(!second, "expected false when server already exists");
    }

    #[test]
    fn remove_server_removes_entry() {
        let dir = TempDir::new().unwrap();
        let root = dir.path();

        ensure_server(root, "to-remove", &make_entry("npx", &["pkg"])).unwrap();
        remove_server(root, "to-remove").unwrap();

        let content = std::fs::read_to_string(root.join(".mcp.json")).unwrap();
        let value: Value = serde_json::from_str(&content).unwrap();

        assert!(
            value["mcpServers"].get("to-remove").is_none(),
            "server entry should have been removed"
        );
    }
}
