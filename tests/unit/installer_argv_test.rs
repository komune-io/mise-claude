//! Unit tests for the two installers that don't touch the filesystem
//! after spawning (SkillInstaller, PluginInstaller). Uses
//! RecordingCommandRunner to assert on argv shape without forking real
//! subprocesses.
//!
//! SkillInstaller now uses git under the hood (resolve_ref → fetch_commit →
//! discover → materialize). Its argv coverage is exercised by the offline
//! integration test `tests/integration/skill_install_test.rs`. Here we only
//! verify parse_skill_path behavior via parse errors (no npx, no git calls).
//!
//! McpInstaller and CliToolInstaller depend on `npm install` actually
//! creating the node_modules tree (so they can find the binary), which
//! the recording runner can't simulate. Those two stay covered by the
//! existing PATH-shim integration tests under tests/integration/.

use chord::core::installer::plugin::PluginInstaller;
use chord::core::installer::skill::SkillInstaller;
use chord::core::installer::{InstallContext, Installer};
use chord::core::process::RecordingCommandRunner;
use chord::core::resolver::{Action, PlannedAction, ToolType};
use std::path::Path;

fn plugin_action(name: &str) -> PlannedAction {
    PlannedAction {
        name: name.to_string(),
        package: name.to_string(),
        version: "latest".to_string(),
        tool_type: ToolType::Plugin,
        action: Action::Install,
    }
}

fn skill_action(name: &str) -> PlannedAction {
    PlannedAction {
        name: name.to_string(),
        package: name.to_string(),
        version: "latest".to_string(),
        tool_type: ToolType::Skill,
        action: Action::Install,
    }
}

// ── SkillInstaller parse error coverage ────────────────────────────────────

/// Installing a one-segment skill name should return an error without running
/// any subprocess (the error is raised during parse, before git is called).
#[test]
fn skill_installer_rejects_one_segment_name() {
    let runner = RecordingCommandRunner::new();
    let ctx = InstallContext {
        project_root: Path::new("."),
        packages_dir: Path::new("."),
        runner: &runner,
    };
    let action = skill_action("lonely");
    let err = SkillInstaller.install(&action, &ctx).unwrap_err();
    assert!(
        err.to_string().contains("lonely"),
        "error should mention the bad key, got: {err}"
    );
    // No subprocess was spawned.
    assert_eq!(runner.calls().len(), 0);
}

/// Installing a four-segment skill name should also be rejected at parse time.
#[test]
fn skill_installer_rejects_four_segment_name() {
    let runner = RecordingCommandRunner::new();
    let ctx = InstallContext {
        project_root: Path::new("."),
        packages_dir: Path::new("."),
        runner: &runner,
    };
    let action = skill_action("a/b/c/d");
    let err = SkillInstaller.install(&action, &ctx).unwrap_err();
    assert!(err.to_string().contains("a/b/c/d"), "got: {err}");
    assert_eq!(runner.calls().len(), 0);
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
    use chord::core::process::CommandError;
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
