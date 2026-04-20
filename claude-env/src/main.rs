use clap::Parser;
use claude_env::cli::{Cli, Command};
use claude_env::config::Config;
use claude_env::installer::mcp::McpInstaller;
use claude_env::installer::{InstallContext, Installer};
use claude_env::lockfile::{LockedTool, Lockfile};
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
            println!("not yet implemented: list");
        }
        Command::Add { tool } => {
            println!("not yet implemented: add {tool}");
        }
        Command::Remove { tool } => {
            println!("not yet implemented: remove {tool}");
        }
    }
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

    let mut installed = 0usize;
    let mut failed = 0usize;
    let mut skipped = 0usize;

    // 5. Execute each action.
    for action in &plan.actions {
        match &action.action {
            Action::Skip => {
                println!("  skip    {} ({})", action.name, action.version);
                skipped += 1;
            }
            Action::Install | Action::Upgrade => {
                let verb = match &action.action {
                    Action::Install => "install",
                    Action::Upgrade => "upgrade",
                    _ => unreachable!(),
                };

                match action.tool_type {
                    ToolType::Mcp => match mcp_installer.install(action, &ctx) {
                        Ok(result) => {
                            println!(
                                "  {verb}   {} @ {} {}",
                                action.name,
                                action.version,
                                if result.installed { "✓" } else { "(already present)" }
                            );

                            // Determine the section for the lockfile.
                            let section = section_name(&action.tool_type);
                            lockfile.set(
                                section,
                                &action.name,
                                LockedTool {
                                    package: Some(action.package.clone()),
                                    version: action.version.clone(),
                                    integrity: result.integrity,
                                    resolved_at: None,
                                },
                            );
                            installed += 1;
                        }
                        Err(e) => {
                            eprintln!("  error   {} : {e}", action.name);
                            failed += 1;
                        }
                    },
                    _ => {
                        println!(
                            "  skip    {} (type not yet implemented)",
                            action.name
                        );
                        skipped += 1;
                    }
                }
            }
        }
    }

    // 6. Write updated lockfile.
    if installed > 0 {
        if let Err(e) = lockfile.write_to_file(&lock_path) {
            eprintln!("error: failed to write lockfile: {e}");
        }
    }

    // 7. Print summary.
    println!(
        "\n{installed} installed, {failed} failed, {skipped} skipped"
    );

    // 8. Exit 1 if any failures.
    if failed > 0 {
        process::exit(1);
    }
}

fn section_name(tool_type: &ToolType) -> &'static str {
    match tool_type {
        ToolType::Mcp => "mcp",
        ToolType::Cli => "cli",
        ToolType::Skill => "skills",
        ToolType::Plugin => "plugins",
    }
}
