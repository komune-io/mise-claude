use clap::{Parser, Subcommand};

/// Declarative Claude Code environment manager.
#[derive(Debug, Parser)]
#[command(name = "claude-env", version, about)]
pub struct Cli {
    /// Enable verbose output.
    #[arg(short, long, global = true)]
    pub verbose: bool,

    #[command(subcommand)]
    pub command: Command,
}

/// Available subcommands.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Install all tools declared in `claude-env.toml`.
    Install,

    /// Update installed tools to their latest matching version.
    Update {
        /// Update only this specific tool (e.g. `context7`). Omit to update all.
        tool: Option<String>,
    },

    /// Show the diff between declared config and what is currently installed.
    Diff {
        /// Tool to show changelog for.
        tool: String,
    },

    /// List all tools currently installed by claude-env.
    List,

    /// Add a tool declaration to `claude-env.toml`.
    Add {
        /// Tool identifier in the form `<name>@<version>` (e.g. `context7@latest`).
        tool: String,
    },

    /// Remove a tool declaration from `claude-env.toml`.
    Remove {
        /// Tool name to remove (e.g. `context7`).
        tool: String,
    },
}
