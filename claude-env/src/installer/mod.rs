pub mod cli_tool;
pub mod mcp;

use crate::error::InstallError;
use crate::resolver::PlannedAction;
use std::path::Path;

pub struct InstallContext<'a> {
    pub project_root: &'a Path,
    pub packages_dir: &'a Path,
    pub verbose: bool,
}

pub trait Installer {
    fn install(
        &self,
        action: &PlannedAction,
        ctx: &InstallContext,
    ) -> Result<InstallResult, InstallError>;
}

pub struct InstallResult {
    pub installed: bool,
    pub integrity: Option<String>,
}
