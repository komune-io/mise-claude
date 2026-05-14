//! Per-scope plugin enable/disable. Writes either project or global
//! `.claude/settings.json` based on the requested [`Scope`].

use serde_json::{json, Value};

use crate::inspect::Scope;

use super::{OpContext, OperationError};

/// Enable or disable a plugin in the requested scope.
///
/// Reads `<scope>/.claude/settings.json` (creating it if absent), inserts or
/// removes the plugin key from `enabledPlugins`, and writes back.
///
/// Both add and remove are idempotent: enabling an already-enabled plugin is
/// a no-op write; disabling a missing key is a no-op write.
pub fn set_plugin_enabled(
    plugin_id: &str,
    scope: Scope,
    enabled: bool,
    ctx: &OpContext,
) -> Result<(), OperationError> {
    let base = match scope {
        Scope::Project => ctx.project_root,
        Scope::Global => ctx.home_dir,
    };
    let settings_path = base.join(".claude").join("settings.json");

    let mut settings: Value = if settings_path.exists() {
        let content = std::fs::read_to_string(&settings_path).map_err(OperationError::Settings)?;
        serde_json::from_str(&content).map_err(|e| {
            OperationError::Settings(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
        })?
    } else {
        json!({})
    };

    let plugins = settings
        .as_object_mut()
        .ok_or_else(|| {
            OperationError::Settings(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "settings.json root is not an object",
            ))
        })?
        .entry("enabledPlugins")
        .or_insert_with(|| json!({}));

    let obj = plugins.as_object_mut().ok_or_else(|| {
        OperationError::Settings(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "enabledPlugins is not an object",
        ))
    })?;
    if enabled {
        obj.insert(plugin_id.to_string(), json!(true));
    } else {
        obj.remove(plugin_id);
    }

    if let Some(parent) = settings_path.parent() {
        std::fs::create_dir_all(parent).map_err(OperationError::Settings)?;
    }

    let content = serde_json::to_string_pretty(&settings).map_err(|e| {
        OperationError::Settings(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
    })?;
    std::fs::write(&settings_path, content).map_err(OperationError::Settings)?;

    Ok(())
}
