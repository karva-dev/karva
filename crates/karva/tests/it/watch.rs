use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

#[test]
fn test_watch_and_dry_run_conflict() {
    let context = TestContext::with_file("test.py", "def test_1(): pass");

    assert_cmd_snapshot!(context.command().args(["--watch", "--dry-run"]), @r"
    success: false
    exit_code: 2
    ----- stdout -----

    ----- stderr -----
    Karva failed
      Cause: `--watch` and `--dry-run` cannot be used together
    ");
}

#[cfg(unix)]
#[test]
fn test_watch_runs_and_can_be_killed() {
    use std::time::Duration;

    let context = TestContext::with_file("test.py", "def test_1(): pass");

    let mut child = context
        .command()
        .args(["--watch", "--no-parallel"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("Failed to spawn karva with --watch");

    // Wait for the first test run to complete
    std::thread::sleep(Duration::from_secs(5));

    child.kill().expect("Failed to kill watch process");

    let output = child
        .wait_with_output()
        .expect("Failed to wait on child process");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stdout.contains("test test::test_1 ... ok"),
        "Expected test output, got: {stdout}"
    );
}
