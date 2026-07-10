//! `chord install` core. Used by the CLI and the TUI.

use std::path::PathBuf;

use crate::core::installer::{InstallContext, InstallerSet};
use crate::core::lockfile::{LockedTool, Lockfile};
use crate::core::output::Reporter;
use crate::core::process::SystemCommandRunner;
use crate::core::resolver::{self, Action, PlannedAction, ToolType};

use super::{OpContext, OperationError};

/// Summary of an install run.
#[derive(Debug)]
pub struct InstallOutcome {
    pub installed: u32,
    pub skipped: u32,
    pub failed: u32,
}

impl InstallOutcome {
    pub fn exit_code(&self) -> i32 {
        if self.failed > 0 {
            1
        } else {
            0
        }
    }
}

/// Install every tool declared in chord.toml. Mirrors the previous
/// `main.rs::run_install` behavior.
pub fn install_all(ctx: &OpContext, quiet: bool) -> Result<InstallOutcome, OperationError> {
    let config = ctx.config_store.load()?;
    let mut lockfile = ctx.lockfile_store.load()?;

    let packages_dir = ctx.packages_dir.to_path_buf();
    let project_root = ctx.project_root.to_path_buf();
    let is_installed = |section: &str, name: &str| -> bool {
        if section == "skills" {
            return skill_installed(&project_root, &lockfile, name);
        }
        packages_dir.join(name).join("node_modules").exists()
            && lockfile.get(section, name).is_some()
    };
    let plan = resolver::resolve(&config, &lockfile, &is_installed);

    let mut reporter = if quiet {
        Reporter::new_quiet()
    } else {
        Reporter::new()
    };

    let runner = SystemCommandRunner::new(ctx.verbose);
    let install_ctx = InstallContext {
        project_root: ctx.project_root,
        packages_dir: &packages_dir,
        runner: &runner,
    };

    for action in &plan.actions {
        execute_action(
            action,
            &install_ctx,
            ctx.installers,
            &mut lockfile,
            &mut reporter,
        );
    }

    // Only persist the lockfile when at least one action succeeded:
    // `execute_action` increments `reporter.installed` on the same code path
    // that mutates `lockfile`. Skipping the write when nothing changed keeps
    // the file mtime stable.
    if reporter.installed > 0 {
        ctx.lockfile_store.save(&lockfile)?;
    }

    reporter.summary();

    Ok(InstallOutcome {
        installed: reporter.installed,
        skipped: reporter.skipped,
        failed: reporter.failed,
    })
}

/// Execute a single planned action and update lockfile/reporter accordingly.
fn execute_action(
    action: &PlannedAction,
    ctx: &InstallContext,
    installers: &InstallerSet,
    lockfile: &mut Lockfile,
    reporter: &mut Reporter,
) {
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

            let result = installers.for_tool(&action.tool_type).install(action, ctx);

            match result {
                Ok(install_result) => {
                    reporter.success(&action.name, &action.version, detail);
                    let section = section_name(&action.tool_type);

                    match action.tool_type {
                        ToolType::Skill => {
                            // Prune skills that disappeared from the source repo since
                            // last install (wildcard shrink case).
                            let owner_repo: String = if action.name.matches('/').count() >= 2 {
                                let idx = action.name.rfind('/').unwrap();
                                action.name[..idx].to_string()
                            } else {
                                action.name.clone()
                            };
                            let new_flat: std::collections::HashSet<String> = install_result
                                .materialized
                                .iter()
                                .map(|m| m.flat_name.clone())
                                .collect();
                            if let Some(prior) = lockfile.get("skills", &action.name) {
                                let prior_names: Vec<String> = match &prior.skills {
                                    Some(subs) => subs.iter().map(|s| s.name.clone()).collect(),
                                    None => vec![action
                                        .name
                                        .rsplit('/')
                                        .next()
                                        .unwrap_or(&action.name)
                                        .to_string()],
                                };
                                prune_orphan_skills(
                                    ctx.project_root,
                                    &owner_repo,
                                    &prior_names,
                                    &new_flat,
                                );
                            }
                            // Clear the entry and any prior expansion, then rewrite
                            // using the RESOLVED sha (install_result.commit), not the
                            // toml ref.
                            lockfile.remove_prefix("skills", &action.name);
                            write_skill_lock(
                                lockfile,
                                action,
                                &install_result,
                                &install_result.commit,
                            );
                        }
                        ToolType::Plugin => {
                            lockfile.set(
                                section,
                                &action.name,
                                LockedTool {
                                    package: None,
                                    version: action.version.clone(),
                                    integrity: None,
                                    resolved_at: Some(
                                        chrono::Utc::now().format("%Y-%m-%d").to_string(),
                                    ),
                                    skills: None,
                                },
                            );
                        }
                        _ => {
                            lockfile.set(
                                section,
                                &action.name,
                                LockedTool {
                                    package: Some(action.package.clone()),
                                    version: action.version.clone(),
                                    integrity: install_result.integrity,
                                    resolved_at: None,
                                    skills: None,
                                },
                            );
                        }
                    }
                }
                Err(e) => {
                    reporter.failure(&action.name, &action.version, &e.to_string());
                }
            }
        }
    }
}

/// A skill is "installed" if its lock row exists and the materialized
/// state is present: for a named skill, the `.claude/skills/<name>` symlink;
/// for a wildcard anchor row, every listed skill's symlink exists.
fn skill_installed(project_root: &std::path::Path, lockfile: &Lockfile, name: &str) -> bool {
    let Some(entry) = lockfile.get("skills", name) else {
        return false;
    };
    let link_exists = |owner_repo: &str, skill: &str| {
        project_root
            .join(".claude")
            .join("skills")
            .join(crate::core::skills::materialize::link_name(
                owner_repo, skill,
            ))
            .exists()
    };
    match &entry.skills {
        // Wildcard anchor row: `name` is the `owner/repo` store namespace.
        Some(subs) => !subs.is_empty() && subs.iter().all(|s| link_exists(name, &s.name)),
        None => {
            // Named skill `owner/repo/leaf`: split off the leaf skill name.
            let (owner_repo, leaf) = name.rsplit_once('/').unwrap_or((name, name));
            link_exists(owner_repo, leaf)
        }
    }
}

fn write_skill_lock(
    lockfile: &mut Lockfile,
    action: &PlannedAction,
    result: &crate::core::installer::InstallResult,
    resolved_sha: &str,
) {
    let now = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let is_wildcard = action.name.matches('/').count() == 1; // owner/repo
    if is_wildcard {
        let subs = result
            .materialized
            .iter()
            .map(|m| crate::core::lockfile::LockedSkill {
                name: m.flat_name.clone(),
                integrity: m.integrity.clone(),
            })
            .collect();
        lockfile.set(
            "skills",
            &action.name,
            LockedTool {
                package: None,
                version: resolved_sha.to_string(),
                integrity: None,
                resolved_at: Some(now),
                skills: Some(subs),
            },
        );
    } else {
        let integrity = result.materialized.first().map(|m| m.integrity.clone());
        lockfile.set(
            "skills",
            &action.name,
            LockedTool {
                package: None,
                version: resolved_sha.to_string(),
                integrity,
                resolved_at: Some(now),
                skills: None,
            },
        );
    }
}

/// Delete the `.chord` store dir and `.claude/skills` symlink for each skill
/// that was in the prior lock entry but is NOT in `new_flat_names` (i.e. it
/// vanished from a wildcard repo between installs). `owner_repo` is the
/// store namespace; `prior_names` are the flat skill names recorded before.
pub fn prune_orphan_skills(
    project_root: &std::path::Path,
    owner_repo: &str,
    prior_names: &[String],
    new_flat_names: &std::collections::HashSet<String>,
) {
    for old in prior_names {
        if new_flat_names.contains(old) {
            continue;
        }
        let store = crate::core::skills::materialize::store_path(project_root, owner_repo, old);
        let _ = std::fs::remove_dir_all(&store);
        let link_name = crate::core::skills::materialize::link_name(owner_repo, old);
        let _ = std::fs::remove_file(project_root.join(".claude").join("skills").join(link_name));
    }
}

pub(super) fn section_name(tool_type: &ToolType) -> &'static str {
    match tool_type {
        ToolType::Mcp => "mcp",
        ToolType::Cli => "cli",
        ToolType::Skill => "skills",
        ToolType::Plugin => "plugins",
    }
}

/// Install a single tool by name. Looks up the tool in chord.toml, builds
/// a plan, executes only the matching action, and updates the lockfile.
///
/// Returns [`OperationError::NotFound`] if the name is not present in any
/// chord.toml section.
pub fn install_one(
    name: &str,
    ctx: &OpContext,
    quiet: bool,
) -> Result<InstallOutcome, OperationError> {
    let config = ctx.config_store.load()?;

    if !config.mcp.contains_key(name)
        && !config.cli.contains_key(name)
        && !config.skills.contains_key(name)
        && !config.plugins.contains_key(name)
    {
        return Err(OperationError::NotFound(name.to_string()));
    }

    let mut lockfile = ctx.lockfile_store.load()?;

    let packages_dir = ctx.packages_dir.to_path_buf();
    let project_root = ctx.project_root.to_path_buf();
    let is_installed = |section: &str, n: &str| -> bool {
        if section == "skills" {
            return skill_installed(&project_root, &lockfile, n);
        }
        packages_dir.join(n).join("node_modules").exists() && lockfile.get(section, n).is_some()
    };
    let plan = resolver::resolve(&config, &lockfile, &is_installed);

    let mut reporter = if quiet {
        Reporter::new_quiet()
    } else {
        Reporter::new()
    };
    let runner = SystemCommandRunner::new(ctx.verbose);
    let install_ctx = InstallContext {
        project_root: ctx.project_root,
        packages_dir: &packages_dir,
        runner: &runner,
    };

    // Execute only the first plan action matching `name`. A chord.toml can
    // legally declare the same name in two sections, but `install_one` is a
    // per-name surgical operation — we install the first match and stop.
    if let Some(action) = plan.actions.iter().find(|a| a.name == name) {
        execute_action(
            action,
            &install_ctx,
            ctx.installers,
            &mut lockfile,
            &mut reporter,
        );
    }

    // See `install_all` for the invariant rationale behind this guard.
    if reporter.installed > 0 {
        ctx.lockfile_store.save(&lockfile)?;
    }

    reporter.summary();

    Ok(InstallOutcome {
        installed: reporter.installed,
        skipped: reporter.skipped,
        failed: reporter.failed,
    })
}

/// Compute the default packages directory (`$CHORD_HOME` or `~/.chord/packages`).
pub fn default_packages_dir() -> PathBuf {
    std::env::var("CHORD_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("."))
                .join(".chord")
                .join("packages")
        })
}
