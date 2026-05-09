use std::collections::HashMap;

/// Per-tool overrides for install behaviour.
#[derive(Debug, Clone)]
pub struct ToolOverride {
    /// Override the binary name used to locate the installed executable.
    pub bin_name: Option<String>,
    /// Shell command to run after the tool is installed. May contain the
    /// `${PROJECT_ROOT}` placeholder.
    pub post_install: Option<String>,
    /// Additional npm packages that must be installed alongside the tool.
    pub extra_deps: Vec<String>,
}

impl ToolOverride {
    /// Returns the resolved `post_install` command with `${PROJECT_ROOT}`
    /// replaced by `project_root`, or `None` if no command is configured.
    pub fn resolve_post_install(&self, project_root: &str) -> Option<String> {
        self.post_install
            .as_ref()
            .map(|cmd| cmd.replace("${PROJECT_ROOT}", project_root))
    }
}

/// Central registry: friendly-name aliases and per-package overrides.
pub struct Registry {
    pub aliases: HashMap<&'static str, &'static str>,
    pub overrides: HashMap<&'static str, ToolOverride>,
}

impl Default for Registry {
    fn default() -> Self {
        let mut aliases: HashMap<&'static str, &'static str> = HashMap::new();
        aliases.insert("context7", "@upstash/context7-mcp");
        aliases.insert("chrome-devtools", "chrome-devtools-mcp");
        aliases.insert("shadcn", "shadcn");
        aliases.insert("gsd", "get-shit-done-cc");
        aliases.insert("bmad", "bmad-method");
        aliases.insert("openspec", "@fission-ai/openspec");

        let mut overrides: HashMap<&'static str, ToolOverride> = HashMap::new();
        overrides.insert(
            "shadcn",
            ToolOverride {
                bin_name: Some("shadcn".to_string()),
                post_install: Some("shadcn mcp init --client claude".to_string()),
                extra_deps: vec!["tinyexec@1.0.2".to_string()],
            },
        );
        overrides.insert(
            "get-shit-done-cc",
            ToolOverride {
                bin_name: None,
                post_install: Some("get-shit-done-cc --claude --local".to_string()),
                extra_deps: vec![],
            },
        );
        overrides.insert(
            "bmad-method",
            ToolOverride {
                bin_name: None,
                post_install: Some(
                    "bmad-method install --directory ${PROJECT_ROOT} --modules bmm --tools claude-code --yes"
                        .to_string(),
                ),
                extra_deps: vec![],
            },
        );
        overrides.insert(
            "@fission-ai/openspec",
            ToolOverride {
                bin_name: None,
                post_install: Some("openspec init --tools claude".to_string()),
                extra_deps: vec![],
            },
        );

        Self { aliases, overrides }
    }
}

impl Registry {
    /// Resolves a friendly alias to its canonical package name.
    /// If `name` is not a known alias it is returned unchanged.
    pub fn resolve_alias<'a>(&'a self, name: &'a str) -> &'a str {
        self.aliases.get(name).copied().unwrap_or(name)
    }

    /// Returns the `ToolOverride` for `package`, if one exists.
    pub fn get_override(&self, package: &str) -> Option<&ToolOverride> {
        self.overrides.get(package)
    }
}
