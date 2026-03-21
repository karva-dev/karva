mod watch;

use std::collections::HashMap;
use std::fmt::Write;
use std::time::{Duration, Instant};

use anyhow::Result;
use karva_cache::AggregatedResults;
use karva_cli::{OutputFormat, TestCommand};
use karva_collector::CollectedPackage;
use karva_logging::{Printer, Stdout, set_colored_override, setup_tracing};
use karva_metadata::filter::{NameFilterSet, TagFilterSet};
use karva_metadata::{ProjectMetadata, ProjectOptionsOverrides};
use karva_project::Project;
use karva_project::path::absolute;
use karva_python_semantic::current_python_version;

use crate::ExitStatus;
use crate::utils::cwd;

pub fn test(args: TestCommand) -> Result<ExitStatus> {
    let verbosity = args.verbosity().level();

    set_colored_override(args.sub_command.color);

    let printer = Printer::new(verbosity, args.sub_command.no_progress.unwrap_or(false));

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

    if args.watch && args.dry_run {
        anyhow::bail!("`--watch` and `--dry-run` cannot be used together");
    }

    let sub_command = args.sub_command.clone();
    let dry_run = args.dry_run;
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

    if dry_run {
        let collected = karva_runner::collect_tests(&project)?;
        print_collected_tests(printer, &collected)?;
        return Ok(ExitStatus::Success);
    }

    TagFilterSet::new(&sub_command.tag_expressions)?;
    NameFilterSet::new(&sub_command.name_patterns)?;

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

    let result = karva_runner::run_parallel_tests(&project, &config, &sub_command)?;

    print_test_output(
        printer,
        start_time,
        &result,
        sub_command.output_format.as_ref(),
        durations,
    )?;

    if result.stats.is_success() && result.discovery_diagnostics.is_empty() {
        Ok(ExitStatus::Success)
    } else {
        Ok(ExitStatus::Failure)
    }
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

    let mut result_stdout = printer.stream_for_failure_summary().lock();
    write!(result_stdout, "{}", result.stats.display(start_time))?;

    Ok(())
}

/// Print discovery diagnostics and regular diagnostics, with concise-mode spacing.
fn print_diagnostics_section(
    stdout: &mut Stdout,
    result: &AggregatedResults,
    is_concise: bool,
) -> Result<()> {
    if !result.discovery_diagnostics.is_empty() {
        writeln!(stdout, "discovery diagnostics:")?;
        writeln!(stdout)?;
        write!(stdout, "{}", result.discovery_diagnostics)?;

        if is_concise {
            writeln!(stdout)?;
        }
    }

    if !result.diagnostics.is_empty() {
        writeln!(stdout, "diagnostics:")?;
        writeln!(stdout)?;
        write!(stdout, "{}", result.diagnostics)?;

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

/// Recursively collect test names from a `CollectedPackage` as `(module_name, function_name)` pairs.
fn collect_test_names(package: &CollectedPackage, tests: &mut Vec<(String, String)>) {
    for module in package.modules.values() {
        let module_name = module.path.module_name().to_string();
        for func in &module.test_function_defs {
            tests.push((module_name.clone(), func.name.to_string()));
        }
    }
    for sub_package in package.packages.values() {
        collect_test_names(sub_package, tests);
    }
}

/// Print collected tests in dry-run mode.
fn print_collected_tests(printer: Printer, collected: &CollectedPackage) -> Result<()> {
    let mut tests = Vec::new();
    collect_test_names(collected, &mut tests);
    tests.sort();

    let mut stdout = printer.stream_for_requested_summary().lock();

    for (module_name, function_name) in &tests {
        writeln!(stdout, "<test> {module_name}::{function_name}")?;
    }

    if !tests.is_empty() {
        writeln!(stdout)?;
    }

    writeln!(stdout, "{} tests collected", tests.len())?;

    Ok(())
}
