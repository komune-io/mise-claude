//! Unit tests for the two installers that don't touch the filesystem
//! after spawning (SkillInstaller, PluginInstaller). Uses
//! RecordingCommandRunner to assert on argv shape without forking real
//! subprocesses.
//!
//! McpInstaller and CliToolInstaller depend on `npm install` actually
//! creating the node_modules tree (so they can find the binary), which
//! the recording runner can't simulate. Those two stay covered by the
//! existing PATH-shim integration tests under tests/integration/.

use chord::installer::plugin::PluginInstaller;
use chord::installer::skill::SkillInstaller;
use chord::installer::{InstallContext, Installer};
use chord::process::RecordingCommandRunner;
use chord::resolver::{Action, PlannedAction, ToolType};
use std::path::Path;

fn skill_action(name: &str) -> PlannedAction {
    PlannedAction {
        name: name.to_string(),
        package: name.to_string(),
        version: "latest".to_string(),
        tool_type: ToolType::Skill,
        action: Action::Install,
    }
}

fn plugin_action(name: &str) -> PlannedAction {
    PlannedAction {
        name: name.to_string(),
        package: name.to_string(),
        version: "latest".to_string(),
        tool_type: ToolType::Plugin,
        action: Action::Install,
    }
}

// ── SkillInstaller ─────────────────────────────────────────────────────────

#[test]
fn skill_installer_passes_specific_skill_name() {
    let runner = RecordingCommandRunner::new();
    let project = Path::new(".");
    let packages = Path::new(".");
    let ctx = InstallContext {
        project_root: project,
        packages_dir: packages,
        runner: &runner,
    };

    let action = skill_action("vercel-labs/next-skills/next-best-practices");
    SkillInstaller.install(&action, &ctx).unwrap();

    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].cmd, "npx");
    assert_eq!(
        calls[0].args,
        vec![
            "skills",
            "add",
            "vercel-labs/next-skills",
            "--skill",
            "next-best-practices",
            "-a",
            "claude-code",
            "-y",
        ]
    );
    assert!(calls[0].env.is_empty(), "no env overrides expected");
}

#[test]
fn skill_installer_passes_wildcard_for_two_segment_key() {
    let runner = RecordingCommandRunner::new();
    let ctx = InstallContext {
        project_root: Path::new("."),
        packages_dir: Path::new("."),
        runner: &runner,
    };

    let action = skill_action("mattpocock/skills");
    SkillInstaller.install(&action, &ctx).unwrap();

    let calls = runner.calls();
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].cmd, "npx");
    assert!(
        calls[0].args.iter().any(|a| a == "*"),
        "expected --skill '*' for wildcard, got {:?}",
        calls[0].args
    );
}

#[test]
fn skill_installer_propagates_non_zero_exit_as_install_error() {
    use chord::process::CommandError;
    use std::process::Command;

    // Build a real non-zero ExitStatus by running `false`.
    let bad_status = Command::new("false").status().unwrap();
    let runner = RecordingCommandRunner::with_results(vec![Err(CommandError::NonZeroExit(
        "npx".to_string(),
        bad_status,
    ))]);
    let ctx = InstallContext {
        project_root: Path::new("."),
        packages_dir: Path::new("."),
        runner: &runner,
    };

    let action = skill_action("mattpocock/skills");
    let err = SkillInstaller.install(&action, &ctx).unwrap_err();
    assert!(err.to_string().contains("npx skills add"), "got: {err}");
}

// ── PluginInstaller ────────────────────────────────────────────────────────

#[test]
fn plugin_installer_runs_marketplace_add_then_install() {
    let runner = RecordingCommandRunner::new();
    let ctx = InstallContext {
        project_root: Path::new("."),
        packages_dir: Path::new("."),
        runner: &runner,
    };

    let action = plugin_action("obra/superpowers-marketplace/superpowers@superpowers-marketplace");
    PluginInstaller.install(&action, &ctx).unwrap();

    let calls = runner.calls();
    assert_eq!(calls.len(), 2);

    assert_eq!(calls[0].cmd, "claude");
    assert_eq!(
        calls[0].args,
        vec![
            "plugin",
            "marketplace",
            "add",
            "obra/superpowers-marketplace"
        ]
    );

    assert_eq!(calls[1].cmd, "claude");
    assert_eq!(
        calls[1].args,
        vec![
            "plugin",
            "install",
            "superpowers@superpowers-marketplace",
            "--scope",
            "project",
        ]
    );
}

#[test]
fn plugin_installer_fails_fast_when_marketplace_add_fails() {
    use chord::process::CommandError;
    use std::process::Command;

    let bad_status = Command::new("false").status().unwrap();
    let runner = RecordingCommandRunner::with_results(vec![Err(CommandError::NonZeroExit(
        "claude".to_string(),
        bad_status,
    ))]);
    let ctx = InstallContext {
        project_root: Path::new("."),
        packages_dir: Path::new("."),
        runner: &runner,
    };

    let action = plugin_action("obra/superpowers-marketplace/superpowers@superpowers-marketplace");
    let err = PluginInstaller.install(&action, &ctx).unwrap_err();
    assert!(
        err.to_string().contains("claude plugin marketplace add"),
        "got: {err}"
    );

    // Only the marketplace-add call was recorded; the second call never fired.
    assert_eq!(runner.calls().len(), 1);
}
