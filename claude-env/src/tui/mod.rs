pub mod actions;
pub mod app;
pub mod handler;
pub mod tree;
pub mod ui;

use std::io;
use std::path::Path;
use crate::config::Config;

pub fn run_tui(
    project_root: &Path,
    home_dir: &Path,
    config: &Config,
) -> io::Result<()> {
    println!("TUI not yet implemented");
    Ok(())
}
