use serde_json::{json, Value};
use std::io;
use std::path::Path;

/// Toggle a plugin's enabled state in ~/.claude/settings.json.
/// If currently_enabled is true, removes the key. Otherwise adds it.
pub fn toggle_plugin(home_dir: &Path, plugin_id: &str, currently_enabled: bool) -> io::Result<()> {
    let settings_path = home_dir.join(".claude").join("settings.json");

    let mut settings: Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path)?;
        serde_json::from_str(&content).map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?
    } else {
        json!({})
    };

    let plugins = settings
        .as_object_mut()
        .unwrap()
        .entry("enabledPlugins")
        .or_insert_with(|| json!({}));

    if currently_enabled {
        if let Some(obj) = plugins.as_object_mut() {
            obj.remove(plugin_id);
        }
    } else {
        if let Some(obj) = plugins.as_object_mut() {
            obj.insert(plugin_id.to_string(), json!(true));
        }
    }

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let content = serde_json::to_string_pretty(&settings)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    std::fs::write(&settings_path, content)
}
