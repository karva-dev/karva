mod watch;

use std::collections::HashMap;
use std::fmt::Write;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use karva_cache::{AggregatedResults, DisplayFlakyTests};
use karva_cli::{OutputFormat, TestCommand};
use karva_logging::{Printer, Stdout, set_colored_override, setup_tracing};
use karva_metadata::filter::FiltersetSet;
use karva_metadata::{CovReport, NoTestsMode, ProjectMetadata, ProjectOptionsOverrides};
use karva_project::Project;
use karva_project::path::absolute;
use karva_python_semantic::current_python_version;

use crate::ExitStatus;
use crate::utils::cwd;

pub fn test(args: TestCommand) -> Result<ExitStatus> {
    let verbosity = args.verbosity().level();

    set_colored_override(args.sub_command.color);

    let _guard = setup_tracing(verbosity);

    let cwd = cwd().map_err(|_| {
        anyhow::anyhow!(
            "The current working directory contains non-Unicode characters. karva only supports Unicode paths."
        )
    })?;

    tracing::debug!(cwd = %cwd, "Working directory");

    let python_version = current_python_version();

    let config_file = args.config_file.as_ref().map(|path| absolute(path, &cwd));

    let mut project_metadata = if let Some(config_file) = &config_file {
        ProjectMetadata::from_config_file(config_file.clone(), &cwd, python_version)?
    } else {
        ProjectMetadata::discover(&cwd, python_version)?
    };

    let sub_command = args.sub_command.clone();
    let watch = args.watch;
    let durations = args.durations;
    let last_failed = args.last_failed;
    let no_cache = args.no_cache.unwrap_or(false);
    let num_workers = if args.no_parallel.unwrap_or(false) || args.no_capture {
        1
    } else {
        args.num_workers
            .unwrap_or_else(|| karva_static::max_parallelism().get())
    };

    let profile = args.profile.clone();
    let project_options_overrides =
        ProjectOptionsOverrides::new(config_file, args.into_options()).with_profile(profile);
    project_metadata
        .apply_overrides(&project_options_overrides)
        .map_err(|err| anyhow::anyhow!("{err}"))?;

    let project = Project::from_metadata(project_metadata);

    let printer = Printer::new(
        project.settings().terminal().status_level,
        project.settings().terminal().final_status_level,
    );

    FiltersetSet::new(&sub_command.filter_expressions).context("invalid `--filter` expression")?;

    let config = karva_runner::ParallelTestConfig {
        num_workers,
        no_cache,
        create_ctrlc_handler: true,
        last_failed,
    };

    if watch {
        watch::run_watch_loop(&project, &config, &sub_command, printer, durations)?;
        return Ok(ExitStatus::Success);
    }

    let start_time = Instant::now();

    let karva_runner::RunOutput {
        results: result,
        coverage_files,
    } = karva_runner::run_parallel_tests(&project, &config, &sub_command, printer)?;

    print_test_output(
        printer,
        start_time,
        &result,
        sub_command.output_format.as_ref(),
        durations,
    )?;

    let coverage_total = if coverage_files.is_empty() {
        None
    } else {
        let show_missing = matches!(project.settings().coverage().report, CovReport::TermMissing);
        match karva_coverage::combine_and_report(project.cwd(), &coverage_files, show_missing) {
            Ok(total) => total,
            Err(err) => {
                tracing::error!("Coverage report failed: {err:#}");
                None
            }
        }
    };

    let coverage_below_threshold = if let Some(total) = coverage_total
        && let Some(threshold) = project.settings().coverage().fail_under
        && total < threshold
    {
        let mut stdout = printer.stream_for_message().lock();
        writeln!(
            stdout,
            "\ncoverage failure: required total coverage of {threshold}% not reached, total coverage was {total:.2}%",
        )?;
        true
    } else {
        false
    };

    if no_tests_collected(&result) {
        let has_filters = !sub_command.filter_expressions.is_empty();
        match project.settings().test().no_tests {
            NoTestsMode::Pass => return Ok(ExitStatus::Success),
            NoTestsMode::Auto if has_filters => return Ok(ExitStatus::Success),
            NoTestsMode::Warn => {
                let mut stdout = printer.stream_for_message().lock();
                writeln!(stdout, "warning: no tests to run")?;
                return Ok(ExitStatus::Success);
            }
            NoTestsMode::Auto | NoTestsMode::Fail => {
                let mut stdout = printer.stream_for_message().lock();
                writeln!(stdout, "error: no tests to run")?;
                writeln!(stdout, "(hint: use `--no-tests` to customize)")?;
                return Ok(ExitStatus::Failure);
            }
        }
    }

    if result.stats.is_success() && result.diagnostics.is_empty() && !coverage_below_threshold {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::Failure)
    }
}

fn no_tests_collected(result: &AggregatedResults) -> bool {
    result.stats.total() == 0 && result.diagnostics.is_empty()
}

/// Print test output: diagnostics, durations, and result summary.
pub fn print_test_output(
    printer: Printer,
    start_time: Instant,
    result: &AggregatedResults,
    output_format: Option<&OutputFormat>,
    durations: Option<usize>,
) -> Result<()> {
    let mut details = printer.stream_for_details().lock();
    let is_concise = matches!(output_format, Some(OutputFormat::Concise));

    let has_diagnostics = !result.diagnostics.is_empty();
    let has_durations = durations.is_some_and(|n| n > 0) && !result.durations.is_empty();
    let has_preceding_test_lines = result.stats.total() > 0;

    write_diagnostics_block(
        &mut details,
        result,
        is_concise,
        /* needs_leading_blank = */ has_preceding_test_lines,
    )?;

    write_durations_block(
        &mut details,
        &result.durations,
        durations,
        // Both diagnostics blocks (concise and non-concise) end on a blank
        // line, so we only need a leading blank when nothing came between
        // the test result lines and us.
        /* needs_leading_blank = */
        has_preceding_test_lines && !has_diagnostics,
    )?;

    drop(details);

    let mut summary = printer
        .stream_for_summary(result.stats.is_success(), result.stats.flaky() > 0)
        .lock();
    // The summary only needs an explicit leading blank when nothing in the
    // details stream provided one — i.e. there were test lines above but
    // neither diagnostics nor durations.
    if has_preceding_test_lines && !has_diagnostics && !has_durations && summary.is_enabled() {
        writeln!(summary)?;
    }
    write!(summary, "{}", result.stats.display(start_time))?;
    write!(summary, "{}", DisplayFlakyTests::new(&result.flaky_tests))?;

    Ok(())
}

fn write_diagnostics_block(
    stdout: &mut Stdout,
    result: &AggregatedResults,
    is_concise: bool,
    needs_leading_blank: bool,
) -> Result<()> {
    if result.diagnostics.is_empty() {
        return Ok(());
    }

    if needs_leading_blank && stdout.is_enabled() {
        writeln!(stdout)?;
    }
    writeln!(stdout, "diagnostics:")?;
    writeln!(stdout)?;
    write!(stdout, "{}", result.diagnostics)?;
    // Non-concise diagnostic content ends with a trailing blank line of its
    // own; concise mode needs an explicit one to match.
    if is_concise {
        writeln!(stdout)?;
    }
    Ok(())
}

fn write_durations_block(
    stdout: &mut Stdout,
    test_durations: &HashMap<String, Duration>,
    durations: Option<usize>,
    needs_leading_blank: bool,
) -> Result<()> {
    let Some(n) = durations else {
        return Ok(());
    };
    if n == 0 || test_durations.is_empty() {
        return Ok(());
    }

    if needs_leading_blank && stdout.is_enabled() {
        writeln!(stdout)?;
    }

    let mut sorted: Vec<_> = test_durations.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    let count = n.min(sorted.len());

    writeln!(stdout, "{count} slowest tests:")?;
    for (name, duration) in sorted.into_iter().take(n) {
        writeln!(
            stdout,
            "  {} ({})",
            name,
            karva_logging::time::format_duration(*duration)
        )?;
    }
    // Trailing blank so the summary divider doesn't bump up against the
    // last duration line.
    writeln!(stdout)?;
    Ok(())
}
