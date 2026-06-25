use chord::core::process::{
    CommandError, CommandRunner, RecordingCommandRunner, SystemCommandRunner,
};
use std::path::Path;

// ── SystemCommandRunner ────────────────────────────────────────────────────

#[test]
fn system_runner_succeeds_for_zero_exit() {
    let runner = SystemCommandRunner::new(false);
    let cwd = Path::new(".");
    let result = runner.run("true", &[], cwd, &[]);
    assert!(result.is_ok(), "expected Ok, got {result:?}");
}

#[test]
fn system_runner_returns_non_zero_exit() {
    let runner = SystemCommandRunner::new(false);
    let cwd = Path::new(".");
    let err = runner.run("false", &[], cwd, &[]).unwrap_err();
    assert!(
        matches!(err, CommandError::NonZeroExit(_, _)),
        "got {err:?}"
    );
}

#[test]
fn system_runner_returns_spawn_error_for_missing_binary() {
    let runner = SystemCommandRunner::new(false);
    let cwd = Path::new(".");
    let err = runner
        .run("/no/such/binary/here-12345", &[], cwd, &[])
        .unwrap_err();
    assert!(matches!(err, CommandError::Spawn(_, _)), "got {err:?}");
}

#[test]
fn system_runner_passes_env_overrides() {
    // Use `sh -c 'echo "$CHORD_TEST"'` and rely on the exit status only —
    // stdio is inherited, we just check that the env var was passed
    // through correctly by running `[ "$CHORD_TEST" = "yes" ]`.
    let runner = SystemCommandRunner::new(false);
    let cwd = Path::new(".");
    let result = runner.run(
        "sh",
        &["-c", "[ \"$CHORD_TEST\" = \"yes\" ]"],
        cwd,
        &[("CHORD_TEST", "yes")],
    );
    assert!(result.is_ok(), "env override didn't propagate: {result:?}");
}

// ── RecordingCommandRunner ─────────────────────────────────────────────────

#[test]
fn recording_runner_captures_calls() {
    let runner = RecordingCommandRunner::new();
    let cwd = std::env::temp_dir();

    runner
        .run("npm", &["install", "foo@1.0"], &cwd, &[])
        .unwrap();
    runner
        .run("npx", &["skills", "add", "vercel-labs/skills"], &cwd, &[])
        .unwrap();

    let calls = runner.calls();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].cmd, "npm");
    assert_eq!(calls[0].args, vec!["install", "foo@1.0"]);
    assert_eq!(calls[1].cmd, "npx");
    assert_eq!(calls[1].args, vec!["skills", "add", "vercel-labs/skills"]);
}

#[test]
fn recording_runner_returns_scripted_results_in_order() {
    let runner = RecordingCommandRunner::with_results(vec![
        Ok(()),
        Err(CommandError::Spawn(
            "boom".to_string(),
            std::io::Error::other("test"),
        )),
        Ok(()),
    ]);
    let cwd = Path::new(".");

    assert!(runner.run("a", &[], cwd, &[]).is_ok());
    assert!(matches!(
        runner.run("b", &[], cwd, &[]).unwrap_err(),
        CommandError::Spawn(_, _)
    ));
    assert!(runner.run("c", &[], cwd, &[]).is_ok());

    // Queue drained → further calls default to Ok.
    assert!(runner.run("d", &[], cwd, &[]).is_ok());
    assert_eq!(runner.calls().len(), 4);
}

#[test]
fn recording_runner_captures_env_overrides() {
    let runner = RecordingCommandRunner::new();
    let cwd = Path::new(".");

    runner
        .run(
            "sh",
            &["-c", "echo hi"],
            cwd,
            &[("PATH", "/custom/bin"), ("OTHER", "value")],
        )
        .unwrap();

    let calls = runner.calls();
    assert_eq!(
        calls[0].env,
        vec![
            ("PATH".to_string(), "/custom/bin".to_string()),
            ("OTHER".to_string(), "value".to_string())
        ]
    );
}
