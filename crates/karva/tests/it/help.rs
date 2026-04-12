use insta_cmd::assert_cmd_snapshot;

use crate::common::TestContext;

#[test]
fn test_help() {
    let context = TestContext::new();

    assert_cmd_snapshot!(context.command().arg("--help"), @r#"
    success: true
    exit_code: 0
    ----- stdout -----
    Run tests

    Usage: karva test [OPTIONS] [PATH]...

    Arguments:
      [PATH]...
              List of files, directories, or test functions to test [default: the project root]

    Options:
          --color <COLOR>
              Control when colored output is used

              Possible values:
              - auto:   Display colors if the output goes to an interactive terminal
              - always: Always display colors
              - never:  Never display colors

      -v, --verbose...
              Use verbose output (or `-vv` and `-vvv` for more verbose output)

      -q, --quiet...
              Use quiet output (or `-qq` for silent output)

      -h, --help
              Print help (see a summary with '-h')

    Filter options:
          --test-prefix <TEST_PREFIX>
              The prefix of the test functions

          --no-ignore
              When set, .gitignore files will not be respected

          --try-import-fixtures
              When set, we will try to import functions in each test file as well as parsing the ast to
              find them.

              This is often slower, so it is not recommended for most projects.

      -E, --filter <FILTER_EXPRESSIONS>
              Filter tests using a filterset expression.

              Predicates: `test(<matcher>)` matches the fully qualified test name; `tag(<matcher>)`
              matches any custom tag on the test.

              Matchers: `=exact`, `~substring`, `/regex/`, `#glob`. The default is substring for
              `test()` and exact for `tag()`. String bodies may be quoted (`"..."`) to allow spaces or
              reserved characters.

              Operators: `&` / `and`, `|` / `or`, `not` / `!`, and `-` as shorthand for "and not". Use
              parentheses for grouping. `and` binds tighter than `or`.

              When specified multiple times, a test runs if it matches any of the expressions (OR
              semantics across flags).

              Examples: `-E 'tag(slow)'`, `-E 'test(/^mod::test_login$/)'`, `-E 'tag(slow) &
              test(~login)'`, `-E '(tag(fast) | tag(unit)) - tag(flaky)'`.

          --run-ignored <RUN_IGNORED>
              Run ignored tests

              Possible values:
              - only: Run only ignored tests
              - all:  Run both ignored and non-ignored tests

          --last-failed
              Re-run only the tests that failed in the previous run

    Runner options:
          --max-fail <N>
              Stop scheduling new tests after this many failures.

              Accepts a positive integer such as `--max-fail=3`. `--max-fail=1` is equivalent to the
              legacy `--fail-fast`, and `--no-fail-fast` clears the limit. When `--max-fail` is provided
              alongside `--fail-fast` or `--no-fail-fast`, `--max-fail` takes precedence.

          --fail-fast
              Stop scheduling new tests after the first failure.

              Equivalent to `--max-fail=1`. Use `--no-fail-fast` to keep running after failures.

          --no-fail-fast
              Run every test regardless of how many fail.

              Clears any `fail-fast` or `max-fail` value set in configuration. When `--max-fail` is
              provided alongside `--no-fail-fast`, `--max-fail` takes precedence.

          --retry <RETRY>
              When set, the test will retry failed tests up to this number of times

          --snapshot-update
              Update snapshots directly instead of creating pending `.snap.new` files.

              When set, `karva.assert_snapshot()` will write directly to `.snap` files, accepting any
              changes automatically.

      -n, --num-workers <NUM_WORKERS>
              Number of parallel workers (default: number of CPU cores)

          --no-parallel
              Disable parallel execution (equivalent to `--num-workers 1`)

          --no-cache
              Disable reading the karva cache for test duration history

          --watch
              Re-run tests when Python source files change

    Reporter options:
          --output-format <OUTPUT_FORMAT>
              The format to use for printing diagnostic messages

              Possible values:
              - full:    Print diagnostics verbosely, with context and helpful hints (default)
              - concise: Print diagnostics concisely, one per line

      -s, --show-output
              Show Python stdout during test execution

          --no-progress
              When set, we will not show individual test case results during execution

          --durations <N>
              Show the N slowest tests after the run completes

    Config options:
          --config-file <PATH>
              The path to a `karva.toml` file to use for configuration.

              While karva configuration can be included in a `pyproject.toml` file, it is not allowed in
              this context.

              [env: KARVA_CONFIG_FILE=]

    ----- stderr -----
    "#);
}
