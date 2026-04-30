mod watch;

use std::collections::HashMap;
use std::fmt::Write;
use std::time::{Duration, Instant};

use anyhow::{Context as _, Result};
use karva_cache::AggregatedResults;
use karva_cli::{OutputFormat, TestCommand};
use karva_logging::{Printer, Stdout, set_colored_override, setup_tracing};
use karva_metadata::filter::FiltersetSet;
use karva_metadata::{NoTestsMode, ProjectMetadata, ProjectOptionsOverrides};
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
    let num_workers = if args.no_parallel.unwrap_or(false) {
        1
    } else {
        args.num_workers
            .unwrap_or_else(|| karva_static::max_parallelism().get())
    };

    let project_options_overrides = ProjectOptionsOverrides::new(config_file, args.into_options());
    project_metadata.apply_overrides(&project_options_overrides);

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

    let result = karva_runner::run_parallel_tests(&project, &config, &sub_command, printer)?;

    print_test_output(
        printer,
        start_time,
        &result,
        sub_command.output_format.as_ref(),
        durations,
    )?;

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

    if result.stats.is_success() && result.discovery_diagnostics.is_empty() {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::Failure)
    }
}

fn no_tests_collected(result: &AggregatedResults) -> bool {
    result.stats.total() == 0
        && result.discovery_diagnostics.is_empty()
        && result.diagnostics.is_empty()
}

/// Print test output: diagnostics, durations, and result summary.
pub fn print_test_output(
    printer: Printer,
    start_time: Instant,
    result: &AggregatedResults,
    output_format: Option<&OutputFormat>,
    durations: Option<usize>,
) -> Result<()> {
    let mut stdout = printer.stream_for_details().lock();
    let is_concise = matches!(output_format, Some(OutputFormat::Concise));

    let has_diagnostics =
        !result.diagnostics.is_empty() || !result.discovery_diagnostics.is_empty();

    if has_diagnostics && result.stats.total() > 0 && stdout.is_enabled() {
        writeln!(stdout)?;
    }

    print_diagnostics_section(&mut stdout, result, is_concise)?;

    let durations_printed = print_durations_section(&mut stdout, &result.durations, durations)?;

    if !has_diagnostics && !durations_printed && result.stats.total() > 0 && stdout.is_enabled() {
        writeln!(stdout)?;
    }

    let mut result_stdout = printer.stream_for_summary(result.stats.is_success()).lock();
    write!(result_stdout, "{}", result.stats.display(start_time))?;

    Ok(())
}

/// Print all diagnostics (collection errors and test failures), with concise-mode spacing.
fn print_diagnostics_section(
    stdout: &mut Stdout,
    result: &AggregatedResults,
    is_concise: bool,
) -> Result<()> {
    let has_any = !result.discovery_diagnostics.is_empty() || !result.diagnostics.is_empty();

    if has_any {
        writeln!(stdout, "diagnostics:")?;
        writeln!(stdout)?;

        if !result.discovery_diagnostics.is_empty() {
            write!(stdout, "{}", result.discovery_diagnostics)?;
        }

        if !result.diagnostics.is_empty() {
            write!(stdout, "{}", result.diagnostics)?;
        }

        if is_concise {
            writeln!(stdout)?;
        }
    }

    Ok(())
}

/// Print the N slowest test durations. Returns whether anything was printed.
fn print_durations_section(
    stdout: &mut Stdout,
    test_durations: &HashMap<String, Duration>,
    durations: Option<usize>,
) -> Result<bool> {
    let Some(n) = durations else {
        return Ok(false);
    };
    if n == 0 || test_durations.is_empty() {
        return Ok(false);
    }

    let mut sorted: Vec<_> = test_durations.iter().collect();
    sorted.sort_by(|a, b| b.1.cmp(a.1));
    let count = n.min(sorted.len());

    writeln!(stdout)?;
    writeln!(stdout, "{count} slowest tests:")?;
    for (name, duration) in sorted.into_iter().take(n) {
        writeln!(
            stdout,
            "  {} ({})",
            name,
            karva_logging::time::format_duration(*duration)
        )?;
    }
    writeln!(stdout)?;

    Ok(true)
}
