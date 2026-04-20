use crate::config::Config;
use crate::lockfile::Lockfile;
use crate::registry::Registry;

/// The action to take for a single tool.
#[derive(Debug, PartialEq)]
pub enum Action {
    Install,
    Upgrade,
    Skip,
}

/// The category a tool belongs to.
#[derive(Debug, PartialEq)]
pub enum ToolType {
    Mcp,
    Cli,
    Skill,
    Plugin,
}

/// A single resolved action for one tool.
#[derive(Debug)]
pub struct PlannedAction {
    pub name: String,
    pub package: String,
    pub version: String,
    pub tool_type: ToolType,
    pub action: Action,
}

/// The full set of actions to execute.
#[derive(Debug)]
pub struct Plan {
    pub actions: Vec<PlannedAction>,
}

/// Resolve the set of actions required to bring the environment in sync.
///
/// `is_installed` is a callback `(section, name) -> bool` that reports whether
/// a given tool is currently installed on disk.  This indirection keeps the
/// resolver pure and easy to test.
pub fn resolve(
    config: &Config,
    lockfile: &Lockfile,
    is_installed: &dyn Fn(&str, &str) -> bool,
) -> Plan {
    let registry = Registry::default();
    let mut actions: Vec<PlannedAction> = Vec::new();

    // (section_name, tool_type, map_of_entries)
    // ToolType does not implement Copy so we use a closure to construct each variant.
    let sections: &[(&str, fn() -> ToolType, &std::collections::BTreeMap<String, String>)] = &[
        ("mcp", || ToolType::Mcp, &config.mcp),
        ("cli", || ToolType::Cli, &config.cli),
        ("skills", || ToolType::Skill, &config.skills),
        ("plugins", || ToolType::Plugin, &config.plugins),
    ];

    for (section, make_type, map) in sections {
        for (name, requested_version) in *map {
            let package = registry.resolve_alias(name).to_string();
            let locked = lockfile.get(section, name);

            let action = match locked {
                None => Action::Install,
                Some(entry) if entry.version != *requested_version => Action::Upgrade,
                Some(_) if !is_installed(section, name) => Action::Install,
                Some(_) => Action::Skip,
            };

            actions.push(PlannedAction {
                name: name.clone(),
                package,
                version: requested_version.clone(),
                tool_type: make_type(),
                action,
            });
        }
    }

    Plan { actions }
}
