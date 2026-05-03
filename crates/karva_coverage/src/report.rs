//! Combine per-worker JSON files and produce a terminal report.
//!
//! Pure Rust — runs in the main process, never touches Python. Reads each
//! per-worker JSON file written by the [`tracer`](crate::tracer), unions the
//! per-file line sets, and prints a `Name / Stmts / Miss / Cover` table
//! sorted alphabetically with a `TOTAL` row.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;

use anyhow::{Context, Result};
use camino::Utf8Path;
use colored::Colorize;

use crate::data::WorkerFile;

/// Combine the per-worker data files in `files` and print a terminal report
/// to stdout. No-ops if there is no data to report.
///
/// `files` is the list of per-worker `coverage.json` paths to merge. The
/// caller (typically [`karva_cache::RunCache::coverage_files`]) is responsible
/// for resolving the paths; this function only reads them.
///
/// When `show_missing` is true, the report includes a final `Missing` column
/// listing the uncovered line numbers per file (consecutive lines collapsed
/// into `a-b` ranges).
///
/// Returns the total coverage percentage (`0.0..=100.0`) shown in the
/// `TOTAL` row, or `None` if there was no data to report. Files with zero
/// executable lines do not contribute to the total.
pub fn combine_and_report(
    cwd: &Utf8Path,
    files: &[impl AsRef<Utf8Path>],
    show_missing: bool,
) -> Result<Option<f64>> {
    let combined = combine(files)?;
    if combined.is_empty() {
        return Ok(None);
    }
    let total = print_report(cwd, &combined, show_missing, &mut std::io::stdout().lock())?;
    Ok(Some(total))
}

#[derive(Debug, Default)]
struct CombinedFile {
    executable: BTreeSet<u32>,
    executed: BTreeSet<u32>,
}

fn combine(files: &[impl AsRef<Utf8Path>]) -> Result<BTreeMap<String, CombinedFile>> {
    let mut combined: BTreeMap<String, CombinedFile> = BTreeMap::new();

    for path in files {
        let path = path.as_ref();
        let bytes = std::fs::read(path.as_std_path())
            .with_context(|| format!("failed to read coverage file {path}"))?;
        let parsed: WorkerFile = serde_json::from_slice(&bytes)
            .with_context(|| format!("failed to parse coverage file {path}"))?;

        for (filename, file_entry) in parsed.files {
            let bucket = combined.entry(filename).or_default();
            bucket.executable.extend(file_entry.executable);
            bucket.executed.extend(file_entry.executed);
        }
    }

    Ok(combined)
}

struct Row<'a> {
    name: &'a str,
    stmts: &'a str,
    miss: &'a str,
    cover: &'a str,
    missing: &'a str,
}

struct FileRow {
    name: String,
    stmts: u32,
    miss: u32,
    missing: String,
}

fn print_report(
    cwd: &Utf8Path,
    combined: &BTreeMap<String, CombinedFile>,
    show_missing: bool,
    out: &mut dyn Write,
) -> Result<f64> {
    let cwd_real = std::fs::canonicalize(cwd.as_std_path()).unwrap_or_else(|_| cwd.into());

    let rows: Vec<FileRow> = combined
        .iter()
        .map(|(filename, data)| {
            let stmts = u32::try_from(data.executable.len()).unwrap_or(u32::MAX);
            let hit = u32::try_from(data.executed.len()).unwrap_or(u32::MAX);
            let miss = stmts.saturating_sub(hit);
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
            FileRow {
                name: display_path(filename, &cwd_real),
                stmts,
                miss,
                missing,
            }
        })
        .collect();

    let name_width = rows
        .iter()
        .map(|row| row.name.len())
        .max()
        .unwrap_or(0)
        .max("Name".len())
        .max("TOTAL".len());

    let header = format_row(
        name_width,
        show_missing,
        &Row {
            name: "Name",
            stmts: "Stmts",
            miss: "Miss",
            cover: "Cover",
            missing: "Missing",
        },
    );
    let rule_len = header.chars().count();
    let rule = "-".repeat(rule_len);

    writeln!(out)?;
    writeln!(out, "{}", header.bold())?;
    writeln!(out, "{rule}")?;

    let mut total_stmts: u32 = 0;
    let mut total_miss: u32 = 0;

    for row in &rows {
        let cover = format_percent(row.stmts, row.miss);
        let stmts_str = row.stmts.to_string();
        let miss_str = row.miss.to_string();
        writeln!(
            out,
            "{}",
            format_row(
                name_width,
                show_missing,
                &Row {
                    name: &row.name,
                    stmts: &stmts_str,
                    miss: &miss_str,
                    cover: &cover,
                    missing: &row.missing,
                },
            )
        )?;
        total_stmts = total_stmts.saturating_add(row.stmts);
        total_miss = total_miss.saturating_add(row.miss);
    }

    writeln!(out, "{rule}")?;
    let total_pct = percent(total_stmts, total_miss);
    let total_cover = format_percent(total_stmts, total_miss);
    let total_stmts_str = total_stmts.to_string();
    let total_miss_str = total_miss.to_string();
    writeln!(
        out,
        "{}",
        format_row(
            name_width,
            show_missing,
            &Row {
                name: "TOTAL",
                stmts: &total_stmts_str,
                miss: &total_miss_str,
                cover: &total_cover,
                missing: "",
            },
        )
    )?;

    Ok(total_pct)
}

fn format_row(name_width: usize, show_missing: bool, row: &Row<'_>) -> String {
    let base = format!(
        "{name:<name_width$}   {stmts:>stmts_w$}   {miss:>miss_w$}   {cover:>cover_w$}",
        name = row.name,
        stmts = row.stmts,
        miss = row.miss,
        cover = row.cover,
        stmts_w = "Stmts".len(),
        miss_w = "Miss".len(),
        cover_w = "Cover".len(),
    );
    if show_missing && !row.missing.is_empty() {
        format!("{base}   {missing}", missing = row.missing)
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

fn percent(total: u32, miss: u32) -> f64 {
    if total == 0 {
        return 100.0;
    }
    let hit = total - miss.min(total);
    f64::from(hit) / f64::from(total) * 100.0
}

fn format_percent(total: u32, miss: u32) -> String {
    let pct = percent(total, miss);
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
        let total = print_report(Utf8Path::new("/proj"), &data, false, &mut buf).unwrap();
        let out = String::from_utf8(buf).unwrap();

        assert!(out.contains("a.py"));
        assert!(out.contains("b.py"));
        assert!(out.contains("TOTAL"));
        assert!(out.contains("67%"));
        assert!(!out.contains("Missing"));
        // 4/6 hit lines ≈ 66.67%; displayed as a rounded `67%` but the
        // returned float is preserved for threshold checks.
        assert!(total > 66.0 && total < 67.0);
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
