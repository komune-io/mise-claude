use clap::Parser;
use claude_env::cli::{Cli, Command};

fn main() {
    let cli = Cli::parse();

    if cli.verbose {
        eprintln!("[verbose] command: {:?}", cli.command);
    }

    match cli.command {
        Command::Install => {
            println!("not yet implemented: install");
        }
        Command::Update { tool } => {
            let target = tool.as_deref().unwrap_or("all");
            println!("not yet implemented: update {target}");
        }
        Command::Diff { tool } => {
            let target = tool.as_deref().unwrap_or("all");
            println!("not yet implemented: diff {target}");
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
