use std::num::NonZeroU32;

use camino::Utf8PathBuf;
use clap::Parser;
use clap::builder::Styles;
use clap::builder::styling::{AnsiColor, Effects};
use karva_logging::{TerminalColor, VerbosityLevel};
use karva_metadata::{
    MaxFail, NoTestsMode, Options, RunIgnoredMode, SrcOptions, TerminalOptions, TestOptions,
};
use ruff_db::diagnostic::DiagnosticFormat;

const STYLES: Styles = Styles::styled()
    .header(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .usage(AnsiColor::Green.on_default().effects(Effects::BOLD))
    .literal(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
    .placeholder(AnsiColor::Cyan.on_default());

#[derive(clap::Args, Debug, Clone, Default)]
#[command(about = None, long_about = None)]
pub struct Verbosity {
    #[arg(
        long,
        short = 'v',
        help = "Use verbose output (or `-vv` and `-vvv` for more verbose output)",
        action = clap::ArgAction::Count,
        global = true,
        overrides_with = "quiet",
    )]
    verbose: u8,

    #[arg(
        long,
        short,
        help = "Use quiet output (or `-qq` for silent output)",
        action = clap::ArgAction::Count,
        global = true,
        overrides_with = "verbose",
    )]
    quiet: u8,
}

impl Verbosity {
    /// Returns the verbosity level based on the number of `-v` and `-q` flags.
    ///
    /// Returns `None` if the user did not specify any verbosity flags.
    pub fn level(&self) -> VerbosityLevel {
        // `--quiet` and `--verbose` are mutually exclusive in Clap, so we can just check one first.
        match self.quiet {
            0 => {}
            1 => return VerbosityLevel::Quiet,
            _ => return VerbosityLevel::Silent,
        }

        match self.verbose {
            0 => VerbosityLevel::Default,
            1 => VerbosityLevel::Verbose,
            2 => VerbosityLevel::ExtraVerbose,
            _ => VerbosityLevel::Trace,
        }
    }
}

#[derive(Debug, Parser)]
#[command(author, name = "karva", about = "A Python test runner.")]
#[command(version = karva_version::version())]
#[command(styles = STYLES)]
pub struct Args {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Debug, clap::Subcommand)]
pub enum Command {
    /// Run tests.
    Test(TestCommand),

    /// Manage snapshots created by `karva.assert_snapshot()`.
    Snapshot(SnapshotCommand),

    /// Manage the karva cache.
    Cache(CacheCommand),

    /// Display Karva's version
    Version,
}

#[derive(Debug, Parser)]
pub struct SnapshotCommand {
    #[command(subcommand)]
    pub action: SnapshotAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum SnapshotAction {
    /// Accept all (or filtered) pending snapshots.
    Accept(SnapshotFilterArgs),

    /// Reject all (or filtered) pending snapshots.
    Reject(SnapshotFilterArgs),

    /// List pending snapshots.
    Pending(SnapshotFilterArgs),

    /// Interactively review pending snapshots.
    Review(SnapshotFilterArgs),

    /// Remove snapshot files whose source test no longer exists.
    Prune(SnapshotPruneArgs),

    /// Delete all (or filtered) snapshot files (.snap and .snap.new).
    Delete(SnapshotDeleteArgs),
}

#[derive(Debug, Parser, Default)]
pub struct SnapshotFilterArgs {
    /// Optional paths to filter snapshots by directory or file.
    #[clap(value_name = "PATH")]
    pub paths: Vec<String>,
}

#[derive(Debug, Parser, Default)]
pub struct SnapshotPruneArgs {
    /// Optional paths to filter snapshots by directory or file.
    #[clap(value_name = "PATH")]
    pub paths: Vec<String>,

    /// Show which snapshots would be removed without deleting them.
    #[clap(long)]
    pub dry_run: bool,
}

#[derive(Debug, Parser, Default)]
pub struct SnapshotDeleteArgs {
    /// Optional paths to filter which snapshot files are deleted.
    #[clap(value_name = "PATH")]
    pub paths: Vec<String>,

    /// Show which snapshot files would be deleted without removing them.
    #[clap(long)]
    pub dry_run: bool,
}

#[derive(Debug, Parser)]
pub struct CacheCommand {
    #[command(subcommand)]
    pub action: CacheAction,
}

#[derive(Debug, clap::Subcommand)]
pub enum CacheAction {
    /// Remove all but the most recent test run from the cache.
    Prune,

    /// Remove the entire cache directory.
    Clean,
}

/// Shared test execution options that can be used by both main CLI and worker processes
#[derive(Debug, Parser, Clone, Default)]
pub struct SubTestCommand {
    /// List of files or directories to test.
    #[clap(
        help = "List of files, directories, or test functions to test [default: the project root]",
        value_name = "PATH"
    )]
    pub paths: Vec<String>,

    /// Control when colored output is used.
    #[arg(long)]
    pub color: Option<TerminalColor>,

    #[clap(flatten)]
    pub verbosity: Verbosity,

    /// The prefix of the test functions.
    #[clap(long, help_heading = "Filter options")]
    pub test_prefix: Option<String>,

    /// When set, .gitignore files will not be respected.
    #[clap(long, default_missing_value = "true", num_args=0..1, help_heading = "Filter options")]
    pub no_ignore: Option<bool>,

    /// When set, we will try to import functions in each test file as well as parsing the ast to find them.
    ///
    /// This is often slower, so it is not recommended for most projects.
    #[clap(long, default_missing_value = "true", num_args=0..1, help_heading = "Filter options")]
    pub try_import_fixtures: Option<bool>,

    /// Filter tests using a filterset expression.
    ///
    /// Predicates: `test(<matcher>)` matches the fully qualified test name;
    /// `tag(<matcher>)` matches any custom tag on the test.
    ///
    /// Matchers: `=exact`, `~substring`, `/regex/`, `#glob`. The default is
    /// substring for `test()` and exact for `tag()`. String bodies may be
    /// quoted (`"..."`) to allow spaces or reserved characters.
    ///
    /// Operators: `&` / `and`, `|` / `or`, `not` / `!`, and `-` as
    /// shorthand for "and not". Use parentheses for grouping. `and` binds
    /// tighter than `or`.
    ///
    /// When specified multiple times, a test runs if it matches any of the
    /// expressions (OR semantics across flags).
    ///
    /// Examples: `-E 'tag(slow)'`, `-E 'test(/^mod::test_login$/)'`,
    /// `-E 'tag(slow) & test(~login)'`,
    /// `-E '(tag(fast) | tag(unit)) - tag(flaky)'`.
    #[clap(short = 'E', long = "filter", help_heading = "Filter options")]
    pub filter_expressions: Vec<String>,

    /// Behavior when no tests are found to run [default: auto]
    ///
    /// `auto` fails if no filter expressions were given, and passes silently
    /// if filters were given (the filter may legitimately match nothing on
    /// some platforms or configurations).
    #[arg(long, value_name = "ACTION", env = "KARVA_NO_TESTS", help_heading = "Filter options")]
    pub no_tests: Option<NoTests>,

    /// Run ignored tests.
    #[arg(long, help_heading = "Filter options")]
    pub run_ignored: Option<RunIgnored>,

    /// Stop scheduling new tests after this many failures.
    ///
    /// Accepts a positive integer such as `--max-fail=3`. `--max-fail=1` is
    /// equivalent to the legacy `--fail-fast`, and `--no-fail-fast` clears
    /// the limit. When `--max-fail` is provided alongside `--fail-fast` or
    /// `--no-fail-fast`, `--max-fail` takes precedence.
    #[clap(long, value_name = "N", help_heading = "Runner options")]
    pub max_fail: Option<NonZeroU32>,

    /// Stop scheduling new tests after the first failure.
    ///
    /// Equivalent to `--max-fail=1`. Use `--no-fail-fast` to keep running
    /// after failures.
    #[clap(long, default_missing_value = "true", num_args=0..1, overrides_with = "no_fail_fast", help_heading = "Runner options")]
    pub fail_fast: Option<bool>,

    /// Run every test regardless of how many fail.
    ///
    /// Clears any `fail-fast` or `max-fail` value set in configuration. When
    /// `--max-fail` is provided alongside `--no-fail-fast`, `--max-fail`
    /// takes precedence.
    #[clap(long, action = clap::ArgAction::SetTrue, overrides_with = "fail_fast", help_heading = "Runner options")]
    pub no_fail_fast: bool,

    /// When set, the test will retry failed tests up to this number of times.
    #[clap(long, help_heading = "Runner options")]
    pub retry: Option<u32>,

    /// Update snapshots directly instead of creating pending `.snap.new` files.
    ///
    /// When set, `karva.assert_snapshot()` will write directly to `.snap` files,
    /// accepting any changes automatically.
    #[clap(long, default_missing_value = "true", num_args=0..1, help_heading = "Runner options")]
    pub snapshot_update: Option<bool>,

    /// The format to use for printing diagnostic messages.
    #[arg(long, help_heading = "Reporter options")]
    pub output_format: Option<OutputFormat>,

    /// Show Python stdout during test execution.
    #[clap(short = 's', long, default_missing_value = "true", num_args=0..1, help_heading = "Reporter options")]
    pub show_output: Option<bool>,

    /// When set, we will not show individual test case results during execution.
    #[clap(long, default_missing_value = "true", num_args=0..1, help_heading = "Reporter options")]
    pub no_progress: Option<bool>,
}

#[derive(Debug, Parser)]
pub struct TestCommand {
    #[clap(flatten)]
    pub sub_command: SubTestCommand,

    /// Re-run only the tests that failed in the previous run.
    #[clap(long, alias = "lf", help_heading = "Filter options")]
    pub last_failed: bool,

    /// Number of parallel workers (default: number of CPU cores)
    #[clap(short = 'n', long, help_heading = "Runner options")]
    pub num_workers: Option<usize>,

    /// Disable parallel execution (equivalent to `--num-workers 1`)
    #[clap(long, default_missing_value = "true", num_args=0..1, help_heading = "Runner options")]
    pub no_parallel: Option<bool>,

    /// Disable reading the karva cache for test duration history
    #[clap(long, default_missing_value = "true", num_args=0..1, help_heading = "Runner options")]
    pub no_cache: Option<bool>,

    /// Re-run tests when Python source files change.
    #[clap(long, help_heading = "Runner options")]
    pub watch: bool,

    /// Show the N slowest tests after the run completes.
    #[clap(long, value_name = "N", help_heading = "Reporter options")]
    pub durations: Option<usize>,

    /// The path to a `karva.toml` file to use for configuration.
    ///
    /// While karva configuration can be included in a `pyproject.toml` file, it is not allowed in this context.
    #[arg(
        long,
        env = "KARVA_CONFIG_FILE",
        value_name = "PATH",
        help_heading = "Config options"
    )]
    pub config_file: Option<Utf8PathBuf>,
}

impl TestCommand {
    pub fn verbosity(&self) -> &Verbosity {
        &self.sub_command.verbosity
    }
}

/// The diagnostic output format.
#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, PartialOrd, Ord, Default, clap::ValueEnum)]
pub enum OutputFormat {
    /// Print diagnostics verbosely, with context and helpful hints (default).
    #[default]
    #[value(name = "full")]
    Full,

    /// Print diagnostics concisely, one per line.
    #[value(name = "concise")]
    Concise,
}

impl From<OutputFormat> for DiagnosticFormat {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Full => Self::Full,
            OutputFormat::Concise => Self::Concise,
        }
    }
}

impl From<OutputFormat> for karva_metadata::OutputFormat {
    fn from(value: OutputFormat) -> Self {
        match value {
            OutputFormat::Full => Self::Full,
            OutputFormat::Concise => Self::Concise,
        }
    }
}

impl SubTestCommand {
    pub fn into_options(self) -> Options {
        // `--no-fail-fast` forces `fail_fast = false` and clears any
        // `max-fail` limit from config. `overrides_with` guarantees
        // `--fail-fast` and `--no-fail-fast` cannot both be active.
        // An explicit `--max-fail=N` still wins over `--no-fail-fast`.
        let fail_fast = if self.no_fail_fast {
            Some(false)
        } else {
            self.fail_fast
        };

        let max_fail = match (self.max_fail, self.no_fail_fast) {
            (Some(n), _) => Some(MaxFail::from(n)),
            (None, true) => Some(MaxFail::unlimited()),
            (None, false) => None,
        };

        Options {
            src: Some(SrcOptions {
                respect_ignore_files: self.no_ignore.map(|no_ignore| !no_ignore),
                include: Some(self.paths),
            }),
            terminal: Some(TerminalOptions {
                output_format: self.output_format.map(Into::into),
                show_python_output: self.show_output,
            }),
            test: Some(TestOptions {
                test_function_prefix: self.test_prefix,
                fail_fast,
                max_fail,
                try_import_fixtures: self.try_import_fixtures,
                retry: self.retry,
                no_tests: self.no_tests.map(Into::into),
            }),
        }
    }
}

impl TestCommand {
    pub fn into_options(self) -> Options {
        self.sub_command.into_options()
    }
}

/// Whether to run ignored/skipped tests.
#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum RunIgnored {
    /// Run only ignored tests.
    Only,

    /// Run both ignored and non-ignored tests.
    All,
}

impl RunIgnored {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Only => "only",
            Self::All => "all",
        }
    }
}

impl From<RunIgnored> for RunIgnoredMode {
    fn from(value: RunIgnored) -> Self {
        match value {
            RunIgnored::Only => Self::Only,
            RunIgnored::All => Self::All,
        }
    }
}

/// Behavior when no tests match filters.
#[derive(Copy, Clone, Hash, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum NoTests {
    /// Automatically determine behavior: fail if no filter expressions were
    /// given, pass silently if filters were given.
    Auto,

    /// Silently exit with code 0.
    Pass,

    /// Produce a warning and exit with code 0.
    Warn,

    /// Produce an error message and exit with a non-zero code.
    Fail,
}

impl NoTests {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Pass => "pass",
            Self::Warn => "warn",
            Self::Fail => "fail",
        }
    }
}

impl From<NoTests> for NoTestsMode {
    fn from(value: NoTests) -> Self {
        match value {
            NoTests::Auto => Self::Auto,
            NoTests::Pass => Self::Pass,
            NoTests::Warn => Self::Warn,
            NoTests::Fail => Self::Fail,
        }
    }
}
