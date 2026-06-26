use chord::core::process::RecordingCommandRunner;
use chord::core::skills::git::{fetch_commit, resolve_ref};
use chord::core::skills::github_url;
use std::path::Path;

#[test]
fn github_url_builds_https_dot_git() {
    assert_eq!(
        github_url("anthropics/skills"),
        "https://github.com/anthropics/skills.git"
    );
}

#[test]
fn resolve_ref_passes_through_a_full_sha_without_ls_remote() {
    let sha = "05754626092947d4e0d1c1bb75297be2fdfa1949";
    let runner = RecordingCommandRunner::new();
    let out = resolve_ref(&runner, "url", sha, Path::new("/tmp")).unwrap();
    assert_eq!(out, sha);
    assert!(
        runner.calls().is_empty(),
        "a full SHA must not trigger ls-remote"
    );
}

#[test]
fn resolve_ref_runs_ls_remote_and_parses_first_column() {
    let runner = RecordingCommandRunner::with_stdout(vec![Ok(
        "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef\trefs/heads/main\n".to_string(),
    )]);
    let out = resolve_ref(
        &runner,
        "https://github.com/o/r.git",
        "main",
        Path::new("/tmp"),
    )
    .unwrap();
    assert_eq!(out, "deadbeefdeadbeefdeadbeefdeadbeefdeadbeef");
    let calls = runner.calls();
    assert_eq!(calls[0].cmd, "git");
    assert_eq!(
        calls[0].args,
        vec!["ls-remote", "https://github.com/o/r.git", "main"]
    );
}

#[test]
fn resolve_ref_latest_queries_head() {
    let runner = RecordingCommandRunner::with_stdout(vec![Ok(
        "abcabcabcabcabcabcabcabcabcabcabcabcabca\tHEAD\n".to_string(),
    )]);
    resolve_ref(&runner, "url", "latest", Path::new("/tmp")).unwrap();
    assert_eq!(runner.calls()[0].args, vec!["ls-remote", "url", "HEAD"]);
}

#[test]
fn resolve_ref_errors_on_empty_ls_remote() {
    let runner = RecordingCommandRunner::with_stdout(vec![Ok(String::new())]);
    let err = resolve_ref(&runner, "url", "nope", Path::new("/tmp"));
    assert!(err.is_err());
}

#[test]
fn fetch_commit_issues_init_remote_fetch_checkout() {
    let sha = "05754626092947d4e0d1c1bb75297be2fdfa1949";
    let runner = RecordingCommandRunner::new();
    let tmp = std::env::temp_dir().join(format!("chord-fetch-test-{sha}"));
    let _ = std::fs::remove_dir_all(&tmp);
    let dir = fetch_commit(&runner, "https://github.com/o/r.git", sha, &tmp).unwrap();
    let calls = runner.calls();
    let argvs: Vec<Vec<String>> = calls
        .iter()
        .map(|c| {
            let mut v = vec![c.cmd.clone()];
            v.extend(c.args.clone());
            v
        })
        .collect();
    assert_eq!(argvs[0], vec!["git", "init"]);
    assert_eq!(argvs[1], vec!["git", "remote", "add", "origin", "https://github.com/o/r.git"]);
    assert_eq!(argvs[2], vec!["git", "fetch", "--depth", "1", "origin", sha]);
    assert_eq!(argvs[3], vec!["git", "checkout", "--detach", "FETCH_HEAD"]);
    for c in &calls {
        assert_eq!(c.cwd, dir);
    }
    let _ = std::fs::remove_dir_all(&tmp);
}
