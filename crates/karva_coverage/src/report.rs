//! Combine per-worker JSON files and produce a terminal report.
//!
//! Pure Rust — runs in the main process, never touches Python. Reads the
//! `karva-coverage.<worker_id>.json` files written by the [`tracer`](crate::tracer),
//! unions the per-file line sets, and prints a `Name / Stmts / Miss / Cover`
//! table sorted alphabetically with a `TOTAL` row.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use anyhow::{Context, Result};
use camino::{Utf8Path, Utf8PathBuf};
use colored::Colorize;

use crate::data::{WORKER_FILE_PREFIX, WORKER_FILE_SUFFIX, WorkerFile};

/// The data file path a worker should write to.
pub fn worker_data_file(data_dir: &Utf8Path, worker_id: usize) -> Utf8PathBuf {
    data_dir.join(format!(
        "{WORKER_FILE_PREFIX}{worker_id}{WORKER_FILE_SUFFIX}"
    ))
}

/// Prepare the coverage data directory by creating it (if missing) and
/// removing any stale per-worker files left from a previous run.
pub fn prepare_data_dir(data_dir: &Utf8Path) -> Result<()> {
    if data_dir.exists() {
        for entry in std::fs::read_dir(data_dir.as_std_path())
            .with_context(|| format!("failed to read coverage dir {data_dir}"))?
        {
            let entry = entry?;
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str())
                && name.starts_with(WORKER_FILE_PREFIX)
                && name.ends_with(WORKER_FILE_SUFFIX)
            {
                let _ = std::fs::remove_file(&path);
            }
        }
    } else {
        std::fs::create_dir_all(data_dir.as_std_path())
            .with_context(|| format!("failed to create coverage dir {data_dir}"))?;
    }
    Ok(())
}

/// Combine per-worker data files in `data_dir` and print a terminal report
/// to stdout. No-ops if there is no data to report.
///
/// When `show_missing` is true, the report includes a final `Missing` column
/// listing the uncovered line numbers per file (consecutive lines collapsed
/// into `a-b` ranges).
pub fn combine_and_report(cwd: &Utf8Path, data_dir: &Utf8Path, show_missing: bool) -> Result<()> {
    let combined = combine(data_dir)?;
    if combined.is_empty() {
        return Ok(());
    }
    print_report(cwd, &combined, show_missing, &mut std::io::stdout().lock())?;
    Ok(())
}

#[derive(Debug, Default)]
struct CombinedFile {
    executable: BTreeSet<u32>,
    executed: BTreeSet<u32>,
}

fn combine(data_dir: &Utf8Path) -> Result<BTreeMap<String, CombinedFile>> {
    let mut combined: BTreeMap<String, CombinedFile> = BTreeMap::new();

    if !data_dir.exists() {
        return Ok(combined);
    }

    for entry in std::fs::read_dir(data_dir.as_std_path())
        .with_context(|| format!("failed to read coverage dir {data_dir}"))?
    {
        let entry = entry?;
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        if !name.starts_with(WORKER_FILE_PREFIX) || !name.ends_with(WORKER_FILE_SUFFIX) {
            continue;
        }

        let bytes = std::fs::read(&path)
            .with_context(|| format!("failed to read coverage file {}", path.display()))?;
        let parsed: WorkerFile = serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse coverage file {}", path.display()))?;

        for (filename, file_entry) in parsed.files {
            let bucket = combined.entry(filename).or_default();
            bucket.executable.extend(file_entry.executable);
            bucket.executed.extend(file_entry.executed);
        }
    }

    Ok(combined)
}

fn print_report(
    cwd: &Utf8Path,
    combined: &BTreeMap<String, CombinedFile>,
    show_missing: bool,
    out: &mut dyn Write,
) -> Result<()> {
    let cwd_real = std::fs::canonicalize(cwd.as_std_path()).unwrap_or_else(|_| cwd.into());

    let rows: Vec<(String, u32, u32, String)> = combined
        .iter()
        .map(|(filename, data)| {
            let display = display_path(filename, &cwd_real);
            let total = u32::try_from(data.executable.len()).unwrap_or(u32::MAX);
            let hit = u32::try_from(data.executed.len()).unwrap_or(u32::MAX);
            let miss = total.saturating_sub(hit);
            let missing = if show_missing {
                let uncovered: BTreeSet<u32> = data
                    .executable
                    .difference(&data.executed)
                    .copied()
                    .collect();
                collapse_ranges(&uncovered)
            } else {
                String::new()
            };
            (display, total, miss, missing)
        })
        .collect();

    let name_width = rows
        .iter()
        .map(|(n, _, _, _)| n.len())
        .max()
        .unwrap_or(0)
        .max("Name".len())
        .max("TOTAL".len());

    let header = format_row(
        name_width,
        "Name",
        "Stmts",
        "Miss",
        "Cover",
        show_missing,
        "Missing",
    );
    let rule_len = header.chars().count();
    let rule = "-".repeat(rule_len);

    writeln!(out)?;
    writeln!(out, "{}", header.bold())?;
    writeln!(out, "{rule}")?;

    let mut total_stmts: u32 = 0;
    let mut total_miss: u32 = 0;

    for (name, stmts, miss, missing) in &rows {
        let cover = format_percent(*stmts, *miss);
        let stmts_str = stmts.to_string();
        let miss_str = miss.to_string();
        writeln!(
            out,
            "{}",
            format_row(
                name_width,
                name,
                &stmts_str,
                &miss_str,
                &cover,
                show_missing,
                missing
            )
        )?;
        total_stmts = total_stmts.saturating_add(*stmts);
        total_miss = total_miss.saturating_add(*miss);
    }

    writeln!(out, "{rule}")?;
    let total_cover = format_percent(total_stmts, total_miss);
    let total_stmts_str = total_stmts.to_string();
    let total_miss_str = total_miss.to_string();
    writeln!(
        out,
        "{}",
        format_row(
            name_width,
            "TOTAL",
            &total_stmts_str,
            &total_miss_str,
            &total_cover,
            show_missing,
            "",
        )
    )?;

    Ok(())
}

fn format_row(
    name_width: usize,
    name: &str,
    stmts: &str,
    miss: &str,
    cover: &str,
    show_missing: bool,
    missing: &str,
) -> String {
    let base = format!(
        "{name:<name_width$}   {stmts:>stmts_w$}   {miss:>miss_w$}   {cover:>cover_w$}",
        stmts_w = "Stmts".len(),
        miss_w = "Miss".len(),
        cover_w = "Cover".len(),
    );
    if show_missing && !missing.is_empty() {
        format!("{base}   {missing}")
    } else {
        base
    }
}

fn collapse_ranges(lines: &BTreeSet<u32>) -> String {
    let mut parts: Vec<String> = Vec::new();
    let mut iter = lines.iter().copied();
    let Some(mut start) = iter.next() else {
        return String::new();
    };
    let mut end = start;
    for line in iter {
        if line != end + 1 {
            parts.push(format_range(start, end));
            start = line;
        }
        end = line;
    }
    parts.push(format_range(start, end));
    parts.join(", ")
}

fn format_range(start: u32, end: u32) -> String {
    if start == end {
        start.to_string()
    } else {
        format!("{start}-{end}")
    }
}

fn format_percent(total: u32, miss: u32) -> String {
    if total == 0 {
        return "100%".to_string();
    }
    let hit = total - miss.min(total);
    let pct = f64::from(hit) / f64::from(total) * 100.0;
    format!("{pct:.0}%")
}

fn display_path(absolute: &str, cwd: &std::path::Path) -> String {
    if let Ok(rel) = std::path::Path::new(absolute).strip_prefix(cwd) {
        rel.to_string_lossy().into_owned()
    } else {
        absolute.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cf(executable: &[u32], executed: &[u32]) -> CombinedFile {
        CombinedFile {
            executable: executable.iter().copied().collect(),
            executed: executed.iter().copied().collect(),
        }
    }

    #[test]
    fn percent_full_coverage() {
        assert_eq!(format_percent(10, 0), "100%");
    }

    #[test]
    fn percent_partial() {
        assert_eq!(format_percent(10, 3), "70%");
    }

    #[test]
    fn percent_zero_stmts() {
        assert_eq!(format_percent(0, 0), "100%");
    }

    #[test]
    fn report_contains_total_row() {
        let mut data = BTreeMap::new();
        data.insert("/proj/a.py".to_string(), cf(&[1, 2, 3, 4], &[1, 2]));
        data.insert("/proj/b.py".to_string(), cf(&[1, 2], &[1, 2]));

        let mut buf: Vec<u8> = Vec::new();
        print_report(Utf8Path::new("/proj"), &data, false, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();

        assert!(out.contains("a.py"));
        assert!(out.contains("b.py"));
        assert!(out.contains("TOTAL"));
        assert!(out.contains("67%"));
        assert!(!out.contains("Missing"));
    }

    #[test]
    fn report_with_missing_shows_uncovered_lines() {
        let mut data = BTreeMap::new();
        data.insert(
            "/proj/a.py".to_string(),
            cf(&[1, 2, 3, 4, 5, 6, 7, 8, 9], &[1, 5, 9]),
        );

        let mut buf: Vec<u8> = Vec::new();
        print_report(Utf8Path::new("/proj"), &data, true, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();

        assert!(out.contains("Missing"));
        assert!(out.contains("2-4, 6-8"));
    }

    #[test]
    fn collapse_empty() {
        let set: BTreeSet<u32> = BTreeSet::new();
        assert_eq!(collapse_ranges(&set), "");
    }

    #[test]
    fn collapse_singletons() {
        let set: BTreeSet<u32> = [3, 7, 12].into_iter().collect();
        assert_eq!(collapse_ranges(&set), "3, 7, 12");
    }

    #[test]
    fn collapse_mixed_ranges() {
        let set: BTreeSet<u32> = [26, 87, 94, 95, 119, 120, 121, 157].into_iter().collect();
        assert_eq!(collapse_ranges(&set), "26, 87, 94-95, 119-121, 157");
    }

    #[test]
    fn collapse_single_contiguous_range() {
        let set: BTreeSet<u32> = [10, 11, 12, 13].into_iter().collect();
        assert_eq!(collapse_ranges(&set), "10-13");
    }
}
