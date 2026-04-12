use crate::common::TestContext;

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
        stdout.contains("PASS") && stdout.contains("test::test_1"),
        "Expected test output, got: {stdout}"
    );
}
