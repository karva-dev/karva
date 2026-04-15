use std::collections::HashMap;
use std::fs;
use std::time::Duration;

use anyhow::Result;
use camino::{Utf8Path, Utf8PathBuf};
use karva_diagnostic::{TestResultStats, TestRunResult};
use ruff_db::diagnostic::{DisplayDiagnosticConfig, DisplayDiagnostics, FileResolver};
use serde::de::DeserializeOwned;

use crate::{
    DIAGNOSTICS_FILE, DISCOVER_DIAGNOSTICS_FILE, DURATIONS_FILE, FAIL_FAST_SIGNAL_FILE,
    FAILED_TESTS_FILE, LAST_FAILED_FILE, RunHash, STATS_FILE, worker_folder,
};

/// Aggregated test results collected from all worker processes.
#[derive(Default)]
pub struct AggregatedResults {
    pub stats: TestResultStats,
    pub diagnostics: String,
    pub discovery_diagnostics: String,
    pub failed_tests: Vec<String>,
    pub durations: HashMap<String, Duration>,
}

/// Reads and writes test results in the cache directory for a specific run.
pub struct Cache {
    run_dir: Utf8PathBuf,
}

impl Cache {
    /// Constructs a cache handle for a specific run within the cache directory.
    pub fn new(cache_dir: &Utf8PathBuf, run_hash: &RunHash) -> Self {
        let run_dir = cache_dir.join(run_hash.to_string());
        Self { run_dir }
    }

    /// Writes a fail-fast signal file to indicate a worker encountered a test failure.
    pub fn write_fail_fast_signal(&self) -> Result<()> {
        fs::create_dir_all(&self.run_dir)?;
        let signal_path = self.run_dir.join(FAIL_FAST_SIGNAL_FILE);
        fs::write(signal_path, "")?;
        Ok(())
    }

    /// Checks whether any worker has written a fail-fast signal file.
    pub fn has_fail_fast_signal(&self) -> bool {
        self.run_dir.join(FAIL_FAST_SIGNAL_FILE).exists()
    }

    /// Reads and merges test results from all worker directories for this run.
    pub fn aggregate_results(&self) -> Result<AggregatedResults> {
        let mut results = AggregatedResults::default();

        if self.run_dir.exists() {
            let mut worker_dirs: Vec<Utf8PathBuf> = fs::read_dir(&self.run_dir)?
                .filter_map(|entry| {
                    let entry = entry.ok()?;
                    let path = Utf8PathBuf::try_from(entry.path()).ok()?;
                    if path.is_dir()
                        && path
                            .file_name()
                            .is_some_and(|name| name.starts_with("worker-"))
                    {
                        Some(path)
                    } else {
                        None
                    }
                })
                .collect();
            worker_dirs.sort();

            for worker_dir in &worker_dirs {
                read_worker_results(worker_dir, &mut results)?;
            }
        }

        Ok(results)
    }

    /// Persists a test run result (stats, diagnostics, durations, and failed tests) to disk.
    pub fn write_result(
        &self,
        worker_id: usize,
        result: &TestRunResult,
        resolver: &dyn FileResolver,
        config: &DisplayDiagnosticConfig,
    ) -> Result<()> {
        let worker_dir = self.run_dir.join(worker_folder(worker_id));
        fs::create_dir_all(&worker_dir)?;

        write_diagnostics(&worker_dir, result, resolver, config)?;
        write_stats(&worker_dir, result.stats())?;
        write_durations(&worker_dir, result.durations())?;
        write_failed_tests(&worker_dir, result.failed_tests())?;

        Ok(())
    }
}

/// Formats and writes test diagnostics and discovery diagnostics to files.
fn write_diagnostics(
    worker_dir: &Utf8Path,
    result: &TestRunResult,
    resolver: &dyn FileResolver,
    config: &DisplayDiagnosticConfig,
) -> Result<()> {
    if !result.diagnostics().is_empty() {
        let output = DisplayDiagnostics::new(resolver, config, result.diagnostics());
        fs::write(worker_dir.join(DIAGNOSTICS_FILE), output.to_string())?;
    }

    if !result.discovery_diagnostics().is_empty() {
        let output = DisplayDiagnostics::new(resolver, config, result.discovery_diagnostics());
        fs::write(
            worker_dir.join(DISCOVER_DIAGNOSTICS_FILE),
            output.to_string(),
        )?;
    }

    Ok(())
}

/// Writes test result stats as JSON.
fn write_stats(worker_dir: &Utf8Path, stats: &TestResultStats) -> Result<()> {
    let json = serde_json::to_string_pretty(stats)?;
    fs::write(worker_dir.join(STATS_FILE), json)?;
    Ok(())
}

/// Writes test durations as JSON.
fn write_durations<K: serde::Serialize, V: serde::Serialize>(
    worker_dir: &Utf8Path,
    durations: &HashMap<K, V>,
) -> Result<()> {
    let json = serde_json::to_string_pretty(durations)?;
    fs::write(worker_dir.join(DURATIONS_FILE), json)?;
    Ok(())
}

/// Writes the list of failed test names as JSON, skipping if empty.
fn write_failed_tests(worker_dir: &Utf8Path, failed_tests: &[impl ToString]) -> Result<()> {
    if !failed_tests.is_empty() {
        let names: Vec<String> = failed_tests.iter().map(ToString::to_string).collect();
        let json = serde_json::to_string_pretty(&names)?;
        fs::write(worker_dir.join(FAILED_TESTS_FILE), json)?;
    }
    Ok(())
}

/// Reads a JSON file from a directory and deserializes it, returning `None` if the file
/// does not exist.
fn read_and_parse<T: DeserializeOwned>(dir: &Utf8Path, filename: &str) -> Result<Option<T>> {
    let path = dir.join(filename);
    if !path.exists() {
        return Ok(None);
    }
    let content = fs::read_to_string(&path)?;
    let value = serde_json::from_str(&content)?;
    Ok(Some(value))
}

/// Reads a text file from a directory, returning `None` if the file does not exist.
fn read_text(dir: &Utf8Path, filename: &str) -> Result<Option<String>> {
    let path = dir.join(filename);
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(fs::read_to_string(&path)?))
}

/// Read results from a single worker directory into the accumulator.
fn read_worker_results(worker_dir: &Utf8Path, results: &mut AggregatedResults) -> Result<()> {
    if let Some(stats) = read_and_parse::<TestResultStats>(worker_dir, STATS_FILE)? {
        results.stats.merge(&stats);
    }

    if let Some(content) = read_text(worker_dir, DIAGNOSTICS_FILE)? {
        results.diagnostics.push_str(&content);
    }

    if let Some(content) = read_text(worker_dir, DISCOVER_DIAGNOSTICS_FILE)? {
        results.discovery_diagnostics.push_str(&content);
    }

    if let Some(failed) = read_and_parse::<Vec<String>>(worker_dir, FAILED_TESTS_FILE)? {
        results.failed_tests.extend(failed);
    }

    if let Some(durations) =
        read_and_parse::<HashMap<String, Duration>>(worker_dir, DURATIONS_FILE)?
    {
        results.durations.extend(durations);
    }

    Ok(())
}

/// Writes the list of failed tests to the cache directory root.
///
/// This overwrites any previous last-failed list.
pub fn write_last_failed(cache_dir: &Utf8Path, failed_tests: &[String]) -> Result<()> {
    fs::create_dir_all(cache_dir)?;
    let path = cache_dir.join(LAST_FAILED_FILE);
    let json = serde_json::to_string_pretty(failed_tests)?;
    fs::write(path, json)?;
    Ok(())
}

/// Reads the list of previously failed tests from the cache directory root.
///
/// Returns an empty list if the file does not exist.
pub fn read_last_failed(cache_dir: &Utf8Path) -> Result<Vec<String>> {
    let path = cache_dir.join(LAST_FAILED_FILE);
    if !path.exists() {
        return Ok(Vec::new());
    }
    let content = fs::read_to_string(&path)?;
    let failed_tests: Vec<String> = serde_json::from_str(&content)?;
    Ok(failed_tests)
}

/// Collects sorted `run-*` directory names from the cache directory.
fn collect_run_dirs(cache_dir: &Utf8Path) -> Result<Vec<String>> {
    let mut run_dirs = Vec::new();

    for entry in fs::read_dir(cache_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if let Some(dir_name) = path.file_name().and_then(|n| n.to_str()) {
                if dir_name.starts_with("run-") {
                    run_dirs.push(dir_name.to_string());
                }
            }
        }
    }

    run_dirs.sort_by_key(|hash| RunHash::from_existing(hash).sort_key());
    Ok(run_dirs)
}

/// Result of a cache prune operation.
pub struct PruneResult {
    /// Names of the removed run directories.
    pub removed: Vec<String>,
}

/// Removes all but the most recent `run-*` directory from the cache.
pub fn prune_cache(cache_dir: &Utf8Path) -> Result<PruneResult> {
    if !cache_dir.exists() {
        return Ok(PruneResult {
            removed: Vec::new(),
        });
    }

    let mut run_dirs = collect_run_dirs(cache_dir)?;

    let to_remove = run_dirs.len().saturating_sub(1);
    let mut removed = Vec::with_capacity(to_remove);

    for dir_name in run_dirs.drain(..to_remove) {
        let path = cache_dir.join(&dir_name);
        fs::remove_dir_all(&path)?;
        removed.push(dir_name);
    }

    Ok(PruneResult { removed })
}

/// Removes the entire cache directory.
///
/// Returns `true` if the directory existed and was removed.
pub fn clean_cache(cache_dir: &Utf8Path) -> Result<bool> {
    if cache_dir.exists() {
        fs::remove_dir_all(cache_dir)?;
        Ok(true)
    } else {
        Ok(false)
    }
}

#[cfg(test)]
mod tests {
    use std::fs;

    use camino::Utf8PathBuf;
    use insta::assert_debug_snapshot;

    use super::*;

    fn create_cache_with_stats(
        dir: &std::path::Path,
        run_name: &str,
        worker_id: usize,
        stats_json: &str,
    ) {
        let worker_dir = dir.join(run_name).join(format!("worker-{worker_id}"));
        fs::create_dir_all(&worker_dir).unwrap();
        fs::write(worker_dir.join(STATS_FILE), stats_json).unwrap();
    }

    #[test]
    fn aggregate_results_merges_stats_from_multiple_workers() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();

        let run_hash = RunHash::from_existing("run-500");

        create_cache_with_stats(tmp.path(), "run-500", 0, r#"{"passed": 3, "failed": 1}"#);
        create_cache_with_stats(tmp.path(), "run-500", 1, r#"{"passed": 2, "skipped": 1}"#);

        let cache = Cache::new(&cache_dir, &run_hash);
        let results = cache.aggregate_results().unwrap();

        assert_eq!(results.stats.passed(), 5);
        assert_eq!(results.stats.failed(), 1);
        assert_eq!(results.stats.skipped(), 1);
    }

    #[test]
    fn aggregate_results_handles_missing_worker_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();

        let run_hash = RunHash::from_existing("run-600");
        let run_dir = tmp.path().join("run-600");
        fs::create_dir_all(&run_dir).unwrap();

        let cache = Cache::new(&cache_dir, &run_hash);
        let results = cache.aggregate_results().unwrap();

        assert_eq!(results.stats.total(), 0);
        assert!(results.diagnostics.is_empty());
    }

    #[test]
    fn write_last_failed_roundtrips_with_read() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();

        let failed = vec!["mod::test_a".to_string(), "mod::test_b".to_string()];
        write_last_failed(&cache_dir, &failed).unwrap();

        assert_debug_snapshot!(read_last_failed(&cache_dir).unwrap(), @r#"
        [
            "mod::test_a",
            "mod::test_b",
        ]
        "#);
    }

    #[test]
    fn read_last_failed_missing_file_returns_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();

        let read = read_last_failed(&cache_dir).unwrap();
        assert!(read.is_empty());
    }

    #[test]
    fn write_last_failed_overwrites_previous_list() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();

        write_last_failed(&cache_dir, &["old".to_string()]).unwrap();
        write_last_failed(&cache_dir, &["new".to_string()]).unwrap();

        assert_debug_snapshot!(read_last_failed(&cache_dir).unwrap(), @r#"
        [
            "new",
        ]
        "#);
    }

    #[test]
    fn write_last_failed_creates_cache_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().join("nested").join("cache")).unwrap();
        assert!(!cache_dir.exists());

        write_last_failed(&cache_dir, &["x".to_string()]).unwrap();

        assert!(cache_dir.exists());
        assert_debug_snapshot!(read_last_failed(&cache_dir).unwrap(), @r#"
        [
            "x",
        ]
        "#);
    }

    #[test]
    fn read_last_failed_empty_json_list_parses() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();

        write_last_failed(&cache_dir, &[]).unwrap();
        assert!(read_last_failed(&cache_dir).unwrap().is_empty());
    }

    #[test]
    fn prune_cache_keeps_most_recent_run_only() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();

        for ts in ["run-100", "run-200", "run-300"] {
            fs::create_dir_all(tmp.path().join(ts)).unwrap();
        }

        let mut removed = prune_cache(&cache_dir).unwrap().removed;
        removed.sort();
        assert_debug_snapshot!(removed, @r#"
        [
            "run-100",
            "run-200",
        ]
        "#);
        assert!(cache_dir.join("run-300").exists());
        assert!(!cache_dir.join("run-100").exists());
        assert!(!cache_dir.join("run-200").exists());
    }

    #[test]
    fn prune_cache_handles_missing_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().join("nope")).unwrap();

        let result = prune_cache(&cache_dir).unwrap();
        assert!(result.removed.is_empty());
    }

    #[test]
    fn prune_cache_ignores_non_run_directories() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();

        fs::create_dir_all(tmp.path().join("run-10")).unwrap();
        fs::create_dir_all(tmp.path().join("run-20")).unwrap();
        fs::create_dir_all(tmp.path().join("not-a-run")).unwrap();
        fs::write(tmp.path().join("last-failed.json"), "[]").unwrap();

        prune_cache(&cache_dir).unwrap();

        assert!(cache_dir.join("not-a-run").exists());
        assert!(cache_dir.join("last-failed.json").exists());
        assert!(cache_dir.join("run-20").exists());
        assert!(!cache_dir.join("run-10").exists());
    }

    #[test]
    fn prune_cache_keeps_newest_even_when_names_are_lexicographically_out_of_order() {
        // `run-9` lexicographically sorts AFTER `run-100` but numerically it is
        // older; pruning must use the numeric `sort_key` or it would delete the
        // newest run directory. This test guards against a regression to naive
        // string sorting.
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();

        fs::create_dir_all(tmp.path().join("run-9")).unwrap();
        fs::create_dir_all(tmp.path().join("run-100")).unwrap();

        prune_cache(&cache_dir).unwrap();

        assert!(cache_dir.join("run-100").exists());
        assert!(!cache_dir.join("run-9").exists());
    }

    #[test]
    fn clean_cache_removes_dir_and_returns_true() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();
        fs::create_dir_all(tmp.path().join("run-1")).unwrap();

        assert!(clean_cache(&cache_dir).unwrap());
        assert!(!cache_dir.exists());
    }

    #[test]
    fn clean_cache_missing_dir_returns_false() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().join("nope")).unwrap();
        assert!(!clean_cache(&cache_dir).unwrap());
    }

    #[test]
    fn aggregate_results_merges_failed_tests_and_durations_across_workers() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();
        let run_hash = RunHash::from_existing("run-700");

        let run_dir = tmp.path().join("run-700");
        let worker0 = run_dir.join("worker-0");
        let worker1 = run_dir.join("worker-1");
        fs::create_dir_all(&worker0).unwrap();
        fs::create_dir_all(&worker1).unwrap();

        fs::write(worker0.join(FAILED_TESTS_FILE), r#"["mod::test_a"]"#).unwrap();
        fs::write(worker1.join(FAILED_TESTS_FILE), r#"["mod::test_b"]"#).unwrap();

        let mut d0 = HashMap::new();
        d0.insert("mod::test_a".to_string(), Duration::from_millis(10));
        let mut d1 = HashMap::new();
        d1.insert("mod::test_b".to_string(), Duration::from_millis(20));
        fs::write(
            worker0.join(DURATIONS_FILE),
            serde_json::to_string(&d0).unwrap(),
        )
        .unwrap();
        fs::write(
            worker1.join(DURATIONS_FILE),
            serde_json::to_string(&d1).unwrap(),
        )
        .unwrap();

        let cache = Cache::new(&cache_dir, &run_hash);
        let results = cache.aggregate_results().unwrap();

        let mut failed = results.failed_tests.clone();
        failed.sort();
        assert_debug_snapshot!(failed, @r#"
        [
            "mod::test_a",
            "mod::test_b",
        ]
        "#);

        let mut durations: Vec<(String, Duration)> = results.durations.into_iter().collect();
        durations.sort();
        assert_debug_snapshot!(durations, @r#"
        [
            (
                "mod::test_a",
                10ms,
            ),
            (
                "mod::test_b",
                20ms,
            ),
        ]
        "#);
    }

    #[test]
    fn fail_fast_signal_round_trip() {
        let tmp = tempfile::tempdir().unwrap();
        let cache_dir = Utf8PathBuf::try_from(tmp.path().to_path_buf()).unwrap();
        let run_hash = RunHash::from_existing("run-800");
        let cache = Cache::new(&cache_dir, &run_hash);

        assert!(!cache.has_fail_fast_signal());
        cache.write_fail_fast_signal().unwrap();
        assert!(cache.has_fail_fast_signal());
    }
}
