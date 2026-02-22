use std::io::Write;
use std::process::Stdio;

use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

#[test]
fn test_inline_snapshot_creates_value() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_hello():
    karva.assert_snapshot("hello world", inline="")
        "#,
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--snapshot-update"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_hello ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");

    let source = context.read_file("test.py");
    insta::assert_snapshot!(source, @r#"
    import karva

    def test_hello():
        karva.assert_snapshot("hello world", inline="hello world")
    "#);
}

#[test]
fn test_inline_snapshot_matches() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_hello():
    karva.assert_snapshot("hello world", inline="hello world")
        "#,
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_hello ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");
}

#[test]
fn test_inline_snapshot_mismatch_no_update() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_hello():
    karva.assert_snapshot("goodbye", inline="hello")
        "#,
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @r#"
    success: false
    exit_code: 1
    ----- stdout -----
    test test::test_hello ... FAILED

    diagnostics:

    error[test-failure]: Test `test_hello` failed
     --> test.py:4:5
      |
    2 | import karva
    3 |
    4 | def test_hello():
      |     ^^^^^^^^^^
    5 |     karva.assert_snapshot("goodbye", inline="hello")
      |
    info: Test failed here
     --> test.py:5:5
      |
    4 | def test_hello():
    5 |     karva.assert_snapshot("goodbye", inline="hello")
      |     ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
      |
    info: Inline snapshot mismatch for 'test_hello'.
          ────────────┬───────────────────────────
              1       │ -hello
                    1 │ +goodbye
          ────────────┴───────────────────────────

    test result: FAILED. 0 passed; 1 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    "#);
}

#[test]
fn test_inline_snapshot_mismatch_updates_source() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_hello():
    karva.assert_snapshot("goodbye", inline="hello")
        "#,
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--snapshot-update"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_hello ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");

    let source = context.read_file("test.py");
    insta::assert_snapshot!(source, @r#"
    import karva

    def test_hello():
        karva.assert_snapshot("goodbye", inline="goodbye")
    "#);
}

#[test]
fn test_inline_snapshot_multiline() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_lines():
    karva.assert_snapshot("line 1\nline 2\nline 3", inline="")
        "#,
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--snapshot-update"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_lines ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");

    let source = context.read_file("test.py");
    insta::assert_snapshot!(source, @r#"

    import karva

    def test_lines():
        karva.assert_snapshot("line 1/nline 2/nline 3", inline="""/
            line 1
            line 2
            line 3
        """)
    "#);
}

#[test]
fn test_inline_snapshot_multiline_matches() {
    let context = TestContext::with_file(
        "test.py",
        "
import karva

def test_lines():
    karva.assert_snapshot(\"line 1\\nline 2\", inline=\"\"\"\\\n        line 1\n        line 2\n    \"\"\")\n",
    );

    assert_cmd_snapshot!(context.command_no_parallel(), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_lines ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");
}

#[test]
fn test_inline_snapshot_multiple_per_test() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_multi():
    with karva.snapshot_settings(allow_duplicates=True):
        karva.assert_snapshot("first", inline="")
        karva.assert_snapshot("second", inline="")
        "#,
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--snapshot-update"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_multi ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");

    let source = context.read_file("test.py");
    insta::assert_snapshot!(source, @r#"
    import karva

    def test_multi():
        with karva.snapshot_settings(allow_duplicates=True):
            karva.assert_snapshot("first", inline="first")
            karva.assert_snapshot("second", inline="second")
    "#);
}

#[test]
fn test_inline_snapshot_accept() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_hello():
    karva.assert_snapshot("hello world", inline="")
        "#,
    );

    let _ = context.command_no_parallel().output();

    let source_before = context.read_file("test.py");
    assert!(
        source_before.contains(r#"inline="""#),
        "Expected source to still have empty inline"
    );

    assert_cmd_snapshot!(context.snapshot("accept"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Accepted: <temp_dir>/snapshots/test__test_hello_inline_5.snap.new

    1 snapshot(s) accepted.

    ----- stderr -----
    ");

    let source_after = context.read_file("test.py");
    insta::assert_snapshot!(source_after, @r#"
    import karva

    def test_hello():
        karva.assert_snapshot("hello world", inline="hello world")
    "#);
}

#[test]
fn test_inline_snapshot_reject() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_hello():
    karva.assert_snapshot("hello world", inline="")
        "#,
    );

    let _ = context.command_no_parallel().output();

    assert_cmd_snapshot!(context.snapshot("reject"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    Rejected: <temp_dir>/snapshots/test__test_hello_inline_5.snap.new

    1 snapshot(s) rejected.

    ----- stderr -----
    ");

    let source = context.read_file("test.py");
    insta::assert_snapshot!(source, @r#"
    import karva

    def test_hello():
        karva.assert_snapshot("hello world", inline="")
    "#);
}

#[test]
fn test_inline_snapshot_with_backslash() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_backslash():
    karva.assert_snapshot("path\\to\\file", inline="")
        "#,
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--snapshot-update"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_backslash ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    ");

    let source = context.read_file("test.py");
    insta::assert_snapshot!(source, @r#"

    import karva

    def test_backslash():
        karva.assert_snapshot("path\/to\/file", inline="path\/to\/file")
    "#);
}

#[test]
fn test_inline_snapshot_with_quotes() {
    let context = TestContext::with_file(
        "test.py",
        "
import karva

def test_quotes():
    karva.assert_snapshot('say \"hi\"', inline=\"\")
        ",
    );

    assert_cmd_snapshot!(context.command_no_parallel().arg("--snapshot-update"), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    test test::test_quotes ... ok

    test result: ok. 1 passed; 0 failed; 0 skipped; finished in [TIME]

    ----- stderr -----
    "#);

    let source = context.read_file("test.py");
    assert!(
        source.contains("say \\\"hi\\\""),
        "Expected escaped double quotes in inline value, got: {source}"
    );
}

#[test]
fn test_inline_snapshot_pending() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_hello():
    karva.assert_snapshot("hello world", inline="")
        "#,
    );

    let _ = context.command_no_parallel().output();

    assert_cmd_snapshot!(context.snapshot("pending"), @r"
    success: true
    exit_code: 0
    ----- stdout -----
    <temp_dir>/snapshots/test__test_hello_inline_5.snap.new

    1 pending snapshot(s).

    ----- stderr -----
    ");
}

#[test]
fn test_inline_review_accept_first_then_review_accept_second() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_first():
    karva.assert_snapshot("hello", inline="")

def test_second():
    karva.assert_snapshot("world", inline="")
        "#,
    );

    let _ = context.command_no_parallel().output();

    // Review: accept first, skip second
    let mut child = context
        .snapshot("review")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn review");
    child
        .stdin
        .take()
        .expect("no stdin")
        .write_all(b"a\ns\n")
        .expect("write failed");
    let _ = child.wait_with_output();

    let source = context.read_file("test.py");
    assert!(
        source.contains(r#"inline="hello""#),
        "Expected first inline rewritten to 'hello', got:\n{source}"
    );
    assert!(
        source.contains(r#"karva.assert_snapshot("world", inline="")"#),
        "Expected second inline still empty, got:\n{source}"
    );

    let pending = context
        .root()
        .join("snapshots/test__test_second_inline_8.snap.new");
    assert!(pending.exists(), "Expected second .snap.new to still exist");

    // Review again: accept second
    let mut child = context
        .snapshot("review")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn review");
    child
        .stdin
        .take()
        .expect("no stdin")
        .write_all(b"a\n")
        .expect("write failed");
    let _ = child.wait_with_output();

    let source = context.read_file("test.py");
    assert!(
        source.contains(r#"inline="hello""#),
        "Expected first inline still 'hello', got:\n{source}"
    );
    assert!(
        source.contains(r#"inline="world""#),
        "Expected second inline rewritten to 'world', got:\n{source}"
    );
    assert!(
        !context
            .root()
            .join("snapshots/test__test_first_inline_5.snap.new")
            .exists(),
        "Expected no pending first snapshot"
    );
    assert!(
        !context
            .root()
            .join("snapshots/test__test_second_inline_8.snap.new")
            .exists(),
        "Expected no pending second snapshot"
    );
}

#[test]
fn test_inline_review_accept_first_then_rerun_accept_second() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_first():
    karva.assert_snapshot("hello", inline="")

def test_second():
    karva.assert_snapshot("world", inline="")
        "#,
    );

    let _ = context.command_no_parallel().output();

    // Review: accept first, skip second
    let mut child = context
        .snapshot("review")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn review");
    child
        .stdin
        .take()
        .expect("no stdin")
        .write_all(b"a\ns\n")
        .expect("write failed");
    let _ = child.wait_with_output();

    let source = context.read_file("test.py");
    assert!(
        source.contains(r#"inline="hello""#),
        "Expected first inline rewritten, got:\n{source}"
    );

    // Re-run tests — first passes, second fails again
    let _ = context.command_no_parallel().output();

    // Accept remaining
    let output = context.snapshot("accept").output().expect("accept failed");
    assert!(output.status.success(), "Expected accept to succeed");

    let source = context.read_file("test.py");
    assert!(
        source.contains(r#"inline="hello""#),
        "Expected first inline still correct, got:\n{source}"
    );
    assert!(
        source.contains(r#"inline="world""#),
        "Expected second inline rewritten to 'world', got:\n{source}"
    );
    assert!(
        !context
            .root()
            .join("snapshots/test__test_second_inline_8.snap.new")
            .exists(),
        "Expected no pending second snapshot"
    );
}

#[test]
fn test_inline_accept_multiline_shifts_lines() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_first():
    karva.assert_snapshot("line1\nline2\nline3", inline="")

def test_second():
    karva.assert_snapshot("world", inline="")
        "#,
    );

    let _ = context.command_no_parallel().output();

    // Review: accept first (multiline), skip second
    let mut child = context
        .snapshot("review")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn review");
    child
        .stdin
        .take()
        .expect("no stdin")
        .write_all(b"a\ns\n")
        .expect("write failed");
    let _ = child.wait_with_output();

    // First should be rewritten to triple-quoted multiline
    let source = context.read_file("test.py");
    assert!(
        source.contains("inline=\"\"\""),
        "Expected first inline rewritten to triple-quoted, got:\n{source}"
    );
    assert!(
        source.contains(r#"karva.assert_snapshot("world", inline="")"#),
        "Expected second inline still empty, got:\n{source}"
    );

    // Second .snap.new should still exist (with original line number)
    let pending = context
        .root()
        .join("snapshots/test__test_second_inline_8.snap.new");
    assert!(pending.exists(), "Expected second .snap.new to still exist");

    // Review again: accept second (stale line number, but find_inline_argument searches forward)
    let mut child = context
        .snapshot("review")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn review");
    child
        .stdin
        .take()
        .expect("no stdin")
        .write_all(b"a\n")
        .expect("write failed");
    let _ = child.wait_with_output();

    let source = context.read_file("test.py");
    assert!(
        source.contains(r#"inline="world""#),
        "Expected second inline rewritten to 'world', got:\n{source}"
    );
    assert!(!pending.exists(), "Expected no pending second snapshot");
}

#[test]
fn test_inline_multiline_accept_rerun_duplicate_pending() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_first():
    karva.assert_snapshot("line1\nline2\nline3", inline="")

def test_second():
    karva.assert_snapshot("world", inline="")
        "#,
    );

    let _ = context.command_no_parallel().output();

    // Review: accept first (multiline), skip second
    let mut child = context
        .snapshot("review")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn review");
    child
        .stdin
        .take()
        .expect("no stdin")
        .write_all(b"a\ns\n")
        .expect("write failed");
    let _ = child.wait_with_output();

    // Old .snap.new for second test at line 8 still exists
    let old_pending = context
        .root()
        .join("snapshots/test__test_second_inline_8.snap.new");
    assert!(old_pending.exists(), "Expected old .snap.new at line 8");

    // Re-run tests — first passes, second fails again (now at shifted line 12)
    let _ = context.command_no_parallel().output();

    // New .snap.new at shifted line 12 should exist alongside old one at line 8
    let new_pending = context
        .root()
        .join("snapshots/test__test_second_inline_12.snap.new");
    assert!(
        new_pending.exists(),
        "Expected new .snap.new at shifted line 12"
    );
    assert!(
        old_pending.exists(),
        "Expected old .snap.new at line 8 to still exist"
    );

    // Accept all — should handle duplicate pending snapshots gracefully
    let output = context.snapshot("accept").output().expect("accept failed");
    assert!(output.status.success(), "Expected accept to succeed");

    let source = context.read_file("test.py");
    assert!(
        source.contains(r#"inline="world""#),
        "Expected second inline rewritten to 'world', got:\n{source}"
    );
    assert!(!old_pending.exists(), "Expected old .snap.new removed");
    assert!(!new_pending.exists(), "Expected new .snap.new removed");
}

/// When a multiline inline accept shifts line numbers, the stale line from a
/// subsequent `.snap.new` may land before an intervening test's `assert_snapshot`
/// call. `find_inline_argument` must not corrupt the intervening inline.
#[test]
fn test_inline_multiline_accept_does_not_corrupt_intervening_inline() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_first():
    karva.assert_snapshot("a\nb\nc", inline="")

def test_middle():
    karva.assert_snapshot("fixed", inline="fixed")

def test_third():
    karva.assert_snapshot("hello", inline="")
        "#,
    );

    // Run tests — first and third fail, middle passes
    let _ = context.command_no_parallel().output();

    // Accept all pending snapshots at once.
    // Alphabetical order processes test_first before test_third.
    // Accepting test_first (multiline) adds 4 lines, shifting test_middle
    // from line 8 to line 12 — but test_third's .snap.new still says line 11.
    // Searching forward from stale line 11 must NOT find test_middle's inline.
    let output = context.snapshot("accept").output().expect("accept failed");
    assert!(output.status.success(), "Expected accept to succeed");

    let source = context.read_file("test.py");

    // Middle's inline must remain unchanged
    assert!(
        source.contains(r#"karva.assert_snapshot("fixed", inline="fixed")"#),
        "Middle inline was corrupted! Got:\n{source}"
    );

    // Third's inline must be rewritten to "hello"
    assert!(
        source.contains(r#"karva.assert_snapshot("hello", inline="hello")"#),
        "Third inline not rewritten correctly! Got:\n{source}"
    );
}

/// Same as above, but using review (accept first, skip third, review again).
#[test]
fn test_inline_multiline_review_does_not_corrupt_intervening_inline() {
    let context = TestContext::with_file(
        "test.py",
        r#"
import karva

def test_first():
    karva.assert_snapshot("a\nb\nc", inline="")

def test_middle():
    karva.assert_snapshot("fixed", inline="fixed")

def test_third():
    karva.assert_snapshot("hello", inline="")
        "#,
    );

    let _ = context.command_no_parallel().output();

    // Review: accept first (multiline), skip third
    let mut child = context
        .snapshot("review")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn review");
    child
        .stdin
        .take()
        .expect("no stdin")
        .write_all(b"a\ns\n")
        .expect("write failed");
    let _ = child.wait_with_output();

    // Review again: accept third (stale line, intervening inline present)
    let mut child = context
        .snapshot("review")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("failed to spawn review");
    child
        .stdin
        .take()
        .expect("no stdin")
        .write_all(b"a\n")
        .expect("write failed");
    let _ = child.wait_with_output();

    let source = context.read_file("test.py");

    // Middle's inline must remain unchanged
    assert!(
        source.contains(r#"karva.assert_snapshot("fixed", inline="fixed")"#),
        "Middle inline was corrupted by review! Got:\n{source}"
    );

    // Third's inline must be rewritten to "hello"
    assert!(
        source.contains(r#"karva.assert_snapshot("hello", inline="hello")"#),
        "Third inline not rewritten by review! Got:\n{source}"
    );
}
