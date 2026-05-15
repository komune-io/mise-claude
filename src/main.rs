use chord::cli::{Cli, Command};
use chord::config::Config;
use chord::lockfile::Lockfile;
use chord::migrate;
use clap::Parser;
use std::path::PathBuf;
use std::process;

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        eprintln!("[verbose] command: {:?}", cli.command);
    }

    match cli.command {
        Command::Install { quiet, idempotent } => {
            let project_root = PathBuf::from(".");
            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            let packages_dir = chord::operations::install::default_packages_dir();
            let ctx = chord::operations::OpContext {
                project_root: &project_root,
                home_dir: &home_dir,
                packages_dir: &packages_dir,
                verbose: cli.verbose,
            };
            match chord::operations::install::install_all(&ctx, quiet || idempotent) {
                Ok(outcome) => process::exit(outcome.exit_code()),
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(2);
                }
            }
        }
        Command::Update { tool } => {
            let target = tool.as_deref().unwrap_or("all");
            println!("not yet implemented: update {target}");
        }
        Command::Diff { tool } => {
            println!("not yet implemented: diff {tool}");
        }
        Command::List => {
            let config_path = PathBuf::from("chord.toml");
            let lock_path = PathBuf::from("chord.lock");

            let config = Config::from_file(&config_path).unwrap_or_default();
            let lockfile = Lockfile::from_file(&lock_path).unwrap_or_default();

            let packages_dir = chord::operations::install::default_packages_dir();

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
                    let status = if installed {
                        "✓ installed"
                    } else {
                        "✗ missing"
                    };
                    println!("  {:<25} {:<12} {}", name, locked_ver, status);
                }
            }
        }
        Command::Add { tool } => {
            let project_root = PathBuf::from(".");
            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            let packages_dir = chord::operations::install::default_packages_dir();
            let ctx = chord::operations::OpContext {
                project_root: &project_root,
                home_dir: &home_dir,
                packages_dir: &packages_dir,
                verbose: cli.verbose,
            };

            let spec = match chord::operations::add::AddSpec::parse(&tool) {
                Ok(s) => s,
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(2);
                }
            };

            match chord::operations::add::add(&spec, &ctx) {
                Ok(outcome) => process::exit(outcome.exit_code()),
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(2);
                }
            }
        }
        Command::Remove { tool } => {
            let project_root = PathBuf::from(".");
            let home_dir = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            let packages_dir = chord::operations::install::default_packages_dir();
            let ctx = chord::operations::OpContext {
                project_root: &project_root,
                home_dir: &home_dir,
                packages_dir: &packages_dir,
                verbose: cli.verbose,
            };
            match chord::operations::remove::remove(&tool, &ctx) {
                Ok(outcome) => {
                    println!("removed {tool} (from [{}])", outcome.section);
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(2);
                }
            }
        }
        Command::Migrate => {
            let project_dir = PathBuf::from(".");
            let total;
            let config = match migrate::migrate(&project_dir) {
                Ok(c) => {
                    total = c.mcp.len() + c.cli.len() + c.skills.len() + c.plugins.len();
                    c
                }
                Err(e) => {
                    eprintln!("error: {e}");
                    process::exit(2);
                }
            };
            if project_dir.join("chord.toml").exists() {
                eprintln!("error: chord.toml already exists — remove it first to re-run migrate");
                process::exit(2);
            }
            if let Err(e) = migrate::write_chord_toml(&config, &project_dir) {
                eprintln!("error: failed to write chord.toml: {e}");
                process::exit(2);
            }
            println!("✓ Found {} claude: tools in .mise.toml", total);
            println!("✓ Written chord.toml");
            println!("→ Remove claude:mcp/*, claude:skills.sh/*, claude:plugin/*, claude:spec/* from .mise.toml");
            println!("→ Keep `chord = \"latest\"` — that installs the chord binary itself");
        }
        Command::Inspect { section, json, tui } => {
            let project_root = std::path::PathBuf::from(".");
            let home_dir = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
            let config_path = std::path::PathBuf::from("chord.toml");
            let config = Config::from_file(&config_path).unwrap_or_default();

            if tui {
                if let Err(e) = chord::tui::run_tui(&project_root, &home_dir, &config) {
                    eprintln!("TUI error: {e}");
                    process::exit(1);
                }
            } else {
                chord::inspect::run_inspect(
                    &project_root,
                    &home_dir,
                    &config,
                    section.as_deref(),
                    json,
                );
            }
        }
    }
}
