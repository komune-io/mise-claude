use clap::Parser;
use claude_env::cli::{Cli, Command};
use claude_env::config::Config;
use claude_env::installer::cli_tool::CliToolInstaller;
use claude_env::installer::mcp::McpInstaller;
use claude_env::installer::plugin::PluginInstaller;
use claude_env::installer::skill::SkillInstaller;
use claude_env::installer::{InstallContext, Installer};
use claude_env::lockfile::{LockedTool, Lockfile};
use claude_env::mcp_config;
use claude_env::output::Reporter;
use claude_env::resolver::{self, Action, ToolType};
use std::path::PathBuf;
use std::process;

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        eprintln!("[verbose] command: {:?}", cli.command);
    }

    match cli.command {
        Command::Install => {
            run_install(cli.verbose);
        }
        Command::Update { tool } => {
            let target = tool.as_deref().unwrap_or("all");
            println!("not yet implemented: update {target}");
        }
        Command::Diff { tool } => {
            println!("not yet implemented: diff {tool}");
        }
        Command::List => {
            let config_path = PathBuf::from("claude-env.toml");
            let lock_path = PathBuf::from("claude-env.lock");

            let config = Config::from_file(&config_path).unwrap_or_default();
            let lockfile = Lockfile::from_file(&lock_path).unwrap_or_default();

            let packages_dir: PathBuf = std::env::var("CLAUDE_ENV_HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| {
                    dirs::home_dir()
                        .unwrap_or_else(|| PathBuf::from("."))
                        .join(".claude-env")
                        .join("packages")
                });

            println!("  {:<25} {:<12} {}", "TOOL", "VERSION", "STATUS");
            println!("  {}", "─".repeat(50));

            for (section, tools) in [
                ("mcp", &config.mcp),
                ("cli", &config.cli),
                ("skills", &config.skills),
                ("plugins", &config.plugins),
            ] {
                for (name, _requested) in tools {
                    let locked_ver = lockfile
                        .get(section, name)
                        .map(|l| l.version.as_str())
                        .unwrap_or("?");
                    let installed = packages_dir.join(name).join("node_modules").exists();
                    let status = if installed { "✓ installed" } else { "✗ missing" };
                    println!("  {:<25} {:<12} {}", name, locked_ver, status);
                }
            }
        }
        Command::Add { tool } => {
            println!("not yet implemented: add {tool}");
        }
        Command::Remove { tool } => {
            run_remove(&tool, cli.verbose);
        }
        Command::Inspect { section, json } => {
            let project_root = std::path::PathBuf::from(".");
            let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            let config_path = std::path::PathBuf::from("claude-env.toml");
            let config = Config::from_file(&config_path).unwrap_or_default();

            claude_env::inspect::run_inspect(
                &project_root,
                &home_dir,
                &config,
                section.as_deref(),
                json,
            );
        }
    }
}

fn run_remove(tool: &str, _verbose: bool) {
    // 1. Read claude-env.toml.
    let config_path = PathBuf::from("claude-env.toml");
    let mut config = match Config::from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to read claude-env.toml: {e}");
            process::exit(2);
        }
    };

    // 2. Find which section the tool belongs to.
    let section = if config.mcp.contains_key(tool) {
        "mcp"
    } else if config.cli.contains_key(tool) {
        "cli"
    } else if config.skills.contains_key(tool) {
        "skills"
    } else if config.plugins.contains_key(tool) {
        "plugins"
    } else {
        eprintln!("error: tool '{tool}' not found in claude-env.toml");
        process::exit(2);
    };

    // 3. Remove from in-memory config and rewrite TOML.
    match section {
        "mcp" => {
            config.mcp.remove(tool);
        }
        "cli" => {
            config.cli.remove(tool);
        }
        "skills" => {
            config.skills.remove(tool);
        }
        "plugins" => {
            config.plugins.remove(tool);
        }
        _ => unreachable!(),
    }

    let toml_content = match toml::to_string_pretty(&config) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to serialize claude-env.toml: {e}");
            process::exit(2);
        }
    };
    if let Err(e) = std::fs::write(&config_path, &toml_content) {
        eprintln!("error: failed to write claude-env.toml: {e}");
        process::exit(2);
    }

    // 4. If MCP tool, remove from .mcp.json.
    if section == "mcp" {
        let project_root = PathBuf::from(".");
        if let Err(e) = mcp_config::remove_server(&project_root, tool) {
            eprintln!("error: failed to update .mcp.json: {e}");
            process::exit(2);
        }
    }

    // 5. Remove package directory if it exists.
    let packages_dir: PathBuf = std::env::var("CLAUDE_ENV_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".claude-env")
                .join("packages")
        });
    let pkg_dir = packages_dir.join(tool);
    if pkg_dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(&pkg_dir) {
            eprintln!("warning: failed to remove package directory: {e}");
        }
    }

    // 6. Remove from lockfile and rewrite.
    let lock_path = PathBuf::from("claude-env.lock");
    let mut lockfile = Lockfile::from_file(&lock_path).unwrap_or_default();
    lockfile.remove(section, tool);
    if let Err(e) = lockfile.write_to_file(&lock_path) {
        eprintln!("error: failed to write lockfile: {e}");
        process::exit(2);
    }

    // 7. Report.
    println!("removed {tool} (from [{section}])");
}

fn run_install(verbose: bool) {
    // 1. Read claude-env.toml from current dir.
    let config_path = PathBuf::from("claude-env.toml");
    let config = match Config::from_file(&config_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("error: failed to read claude-env.toml: {e}");
            process::exit(2);
        }
    };

    // 2. Determine packages_dir.
    let packages_dir: PathBuf = std::env::var("CLAUDE_ENV_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".claude-env")
                .join("packages")
        });

    // 3. Read lockfile (empty if missing).
    let lock_path = PathBuf::from("claude-env.lock");
    let mut lockfile = match Lockfile::from_file(&lock_path) {
        Ok(lf) => lf,
        Err(e) => {
            eprintln!("error: failed to read claude-env.lock: {e}");
            process::exit(2);
        }
    };

    // 4. Resolve plan.
    let is_installed = |section: &str, name: &str| -> bool {
        packages_dir
            .join(name)
            .join("node_modules")
            .exists()
            && lockfile.get(section, name).is_some()
    };

    let plan = resolver::resolve(&config, &lockfile, &is_installed);

    let project_root = PathBuf::from(".");
    let ctx = InstallContext {
        project_root: &project_root,
        packages_dir: &packages_dir,
        verbose,
    };

    let mcp_installer = McpInstaller::default();
    let cli_installer = CliToolInstaller::default();
    let skill_installer = SkillInstaller;
    let plugin_installer = PluginInstaller;

    let mut reporter = Reporter::new();

    // 5. Execute each action.
    for action in &plan.actions {
        match &action.action {
            Action::Skip => {
                reporter.skip(&action.name, &action.version);
            }
            Action::Install | Action::Upgrade => {
                let detail = match &action.action {
                    Action::Install => "installed",
                    Action::Upgrade => "upgraded",
                    _ => unreachable!(),
                };

                let install_result = match action.tool_type {
                    ToolType::Mcp => mcp_installer.install(action, &ctx),
                    ToolType::Cli => cli_installer.install(action, &ctx),
                    ToolType::Skill => skill_installer.install(action, &ctx),
                    ToolType::Plugin => plugin_installer.install(action, &ctx),
                };

                match install_result {
                    Ok(result) => {
                        reporter.success(&action.name, &action.version, detail);

                        // Determine the section for the lockfile.
                        let section = section_name(&action.tool_type);
                        let locked_tool = if action.tool_type == ToolType::Skill
                            || action.tool_type == ToolType::Plugin
                        {
                            LockedTool {
                                package: None,
                                version: action.version.clone(),
                                integrity: None,
                                resolved_at: Some(
                                    chrono::Utc::now().format("%Y-%m-%d").to_string(),
                                ),
                            }
                        } else {
                            LockedTool {
                                package: Some(action.package.clone()),
                                version: action.version.clone(),
                                integrity: result.integrity,
                                resolved_at: None,
                            }
                        };
                        lockfile.set(section, &action.name, locked_tool);
                    }
                    Err(e) => {
                        reporter.failure(&action.name, &action.version, &e.to_string());
                    }
                }
            }
        }
    }

    // 6. Write updated lockfile.
    if reporter.installed > 0 {
        if let Err(e) = lockfile.write_to_file(&lock_path) {
            eprintln!("error: failed to write lockfile: {e}");
        }
    }

    // 7. Print summary and exit.
    reporter.summary();
    process::exit(reporter.exit_code());
}

fn section_name(tool_type: &ToolType) -> &'static str {
    match tool_type {
        ToolType::Mcp => "mcp",
        ToolType::Cli => "cli",
        ToolType::Skill => "skills",
        ToolType::Plugin => "plugins",
    }
}
