use chord::output::Reporter;

#[test]
fn quiet_reporter_skip_increments_counter_without_panic() {
    let mut reporter = Reporter::new_quiet();
    reporter.skip("context7", "2.1.4");
    assert_eq!(reporter.skipped, 1);
    assert_eq!(reporter.installed, 0);
    assert_eq!(reporter.failed, 0);
}

#[test]
fn quiet_reporter_exit_code_is_zero_when_all_skipped() {
    let mut reporter = Reporter::new_quiet();
    reporter.skip("context7", "2.1.4");
    assert_eq!(reporter.exit_code(), 0);
}

#[test]
fn quiet_reporter_exit_code_is_one_when_failed() {
    let mut reporter = Reporter::new_quiet();
    reporter.failure("context7", "2.1.4", "npm not found");
    assert_eq!(reporter.exit_code(), 1);
}

#[test]
fn default_reporter_is_not_quiet() {
    let reporter = Reporter::new();
    assert_eq!(reporter.installed, 0);
}
