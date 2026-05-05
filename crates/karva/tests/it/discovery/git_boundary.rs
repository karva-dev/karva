use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

/// Karva must not walk past a `.git` directory when searching for `pyproject.toml`.
///
/// The scenario mirrors a project cloned into a subdirectory of another git
/// repo: the inner project has its own `.git` and no `pyproject.toml`, but the
/// outer repo has one. Without the `.git` boundary check, karva would find the
/// outer `pyproject.toml` and resolve test paths relative to the wrong root.
#[test]
fn test_discovery_stops_at_git_boundary() {
    // Layout:
    //   <root>/                   ← outer project
    //     .git/                   ← outer git root (should NOT be crossed)
    //     pyproject.toml          ← outer config (tool.karva sets wrong prefix)
    //     subproject/             ← inner project (cloned repo)
    //       .git/                 ← inner git root (boundary that stops the walk)
    //       test_inner.py         ← karva is invoked from here
    let context = TestContext::with_files([
        (
            "pyproject.toml",
            r#"
[project]
name = "outer-project"

[tool.karva.profile.default.test]
test-function-prefix = "outer_"
"#,
        ),
        (
            "subproject/test_inner.py",
            r"
def test_inner(): pass
",
        ),
    ]);

    // Create .git directories: one at the root (outer repo) and one inside the
    // subproject (inner cloned repo). The inner .git is the boundary that should
    // stop discovery before reaching the outer pyproject.toml.
    std::fs::create_dir_all(context.root().join(".git"))
        .expect("Failed to create outer .git directory");
    std::fs::create_dir_all(context.root().join("subproject").join(".git"))
        .expect("Failed to create inner .git directory");

    // Run karva from inside the subproject. Discovery starts at `subproject/`,
    // finds .git there, and stops — it must NOT reach the outer pyproject.toml
    // that sets test-function-prefix = "outer_". The default prefix "test_" is
    // used, so test_inner() is found and run.
    let mut cmd = context.karva_command_in(context.root().join("subproject"));
    cmd.arg("test");

    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] test_inner::test_inner
    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}

/// When `pyproject.toml` is inside the same git repo (no boundary crossed),
/// discovery should still find it as before.
#[test]
fn test_discovery_finds_pyproject_within_git_repo() {
    // Layout:
    //   <root>/                   ← project root with .git and pyproject.toml
    //     .git/
    //     pyproject.toml          ← config with tool.karva (sets prefix = "spec")
    //     tests/
    //       test_feature.py       ← karva is invoked from here
    let context = TestContext::with_files([
        (
            "pyproject.toml",
            r#"
[project]
name = "my-project"

[tool.karva.profile.default.test]
test-function-prefix = "spec"
"#,
        ),
        (
            "tests/test_feature.py",
            r"
def spec_feature(): pass
def test_should_not_run(): pass
",
        ),
    ]);

    // Place .git at the root — pyproject.toml is also at the root, so discovery
    // finds it at the same level as the .git and returns it immediately.
    std::fs::create_dir_all(context.root().join(".git")).expect("Failed to create .git directory");

    let mut cmd = context.karva_command_in(context.root().join("tests"));
    cmd.arg("test");

    assert_cmd_snapshot!(cmd, @"
    success: true
    exit_code: 0
    ----- stdout -----
        Starting 1 test across 1 worker
            PASS [TIME] tests.test_feature::spec_feature
    ────────────
         Summary [TIME] 1 test run: 1 passed, 0 skipped

    ----- stderr -----
    ");
}
