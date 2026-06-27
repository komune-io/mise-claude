use clap::{Parser, Subcommand};

/// Declarative Claude Code environment manager.
#[derive(Debug, Parser)]
#[command(name = "chord", version, about)]
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
    /// Install all tools declared in `chord.toml`.
    Install {
        /// Suppress output when nothing changed (for shell hook use).
        #[arg(long)]
        quiet: bool,

        /// Document intent: all installs are idempotent by default (resolver skips already-installed tools).
        #[arg(long)]
        idempotent: bool,
    },

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

    /// List all tools currently installed by chord.
    List,

    /// Add a tool declaration to `chord.toml`.
    Add {
        /// Tool identifier in the form `<name>@<version>` (e.g. `context7@latest`).
        tool: String,
    },

    /// Remove a tool declaration from `chord.toml`.
    Remove {
        /// Tool name to remove (e.g. `context7`).
        tool: String,
    },

    /// Remove everything chord installed (per chord.lock), keeping chord.toml.
    Clean {
        /// Full project reset: also remove foreign artifacts and ALL Claude
        /// tool config in the project (.agents/, skills-lock.json, the whole
        /// .claude/skills/, .mcp.json, .claude/settings.json) — not just
        /// chord-owned state. Destructive: deletes config chord did not create.
        #[arg(short, long)]
        all: bool,
    },

    /// Migrate Claude tool declarations from .mise.toml to chord.toml.
    ///
    /// Reads the current directory's .mise.toml, finds all `claude:mcp/*`,
    /// `claude:skills.sh/*`, `claude:plugin/*`, and `claude:spec/*` entries,
    /// and writes a `chord.toml` with equivalent declarations.
    Migrate,

    /// Audit all Claude Code configuration (project + global).
    Inspect {
        /// Filter to a specific category (mcp, plugins, skills, commands, agents).
        #[arg(long)]
        section: Option<String>,

        /// Output as JSON.
        #[arg(long)]
        json: bool,

        /// Launch interactive TUI mode.
        #[arg(long)]
        tui: bool,
    },
}
