//! Wall-time benchmark for karva.
//!
//! Clones a fixed snapshot of <https://github.com/MatthewMckee4/karva-benchmark-1>
//! into `target/benchmark_cache/`, installs its dependencies via uv, and runs
//! `karva test` against it. The snapshot is pinned to a specific commit so
//! results stay stable across runs.

use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use camino::Utf8PathBuf;
use divan::Bencher;
use karva_cli::{OutputFormat, SubTestCommand};
use karva_logging::{FinalStatusLevel, Printer, StatusLevel};
use karva_metadata::ProjectMetadata;
use karva_project::Project;
use ruff_python_ast::PythonVersion;

const PROJECT_NAME: &str = "karva-benchmark-1";
const REPOSITORY: &str = "https://github.com/MatthewMckee4/karva-benchmark-1";
const COMMIT: &str = "89791b99d8b13a1e104af7a0b55b3741e315268a";
const DEPENDENCIES: &[&str] = &["pytest"];
const MAX_DEP_DATE: &str = "2026-12-01";
const PYTHON_VERSION: PythonVersion = PythonVersion::PY313;

/// Clone (or update) the benchmark project, install its dependencies, and
/// return a `Project` ready to be benchmarked.
pub fn setup_project() -> Project {
    let project_root = ensure_checkout().expect("Failed to checkout benchmark project");
    install_dependencies(&project_root).expect("Failed to install dependencies");

    Project::from_metadata(ProjectMetadata::new(project_root, PYTHON_VERSION))
}

/// Run karva tests against the prepared project once.
pub fn run_karva(project: &Project) {
    let config = karva_runner::ParallelTestConfig {
        num_workers: 2,
        no_cache: false,
        create_ctrlc_handler: false,
        last_failed: false,
    };

    let args = SubTestCommand {
        no_ignore: Some(true),
        output_format: Some(OutputFormat::Concise),
        status_level: Some(StatusLevel::None),
        final_status_level: Some(FinalStatusLevel::None),
        ..SubTestCommand::default()
    };

    let printer = Printer::new(StatusLevel::None, FinalStatusLevel::None);
    let output = karva_runner::run_parallel_tests(project, &config, &args, printer).unwrap();

    assert!(output.results.stats.total() > 0);
}

/// Divan bencher entry point: re-creates the project for each iteration so the
/// measurement covers running tests, not project construction.
pub fn bench(bencher: Bencher) {
    bencher
        .with_inputs(setup_project)
        .bench_local_refs(|project| run_karva(project));
}

fn ensure_checkout() -> Result<Utf8PathBuf> {
    let project_root = project_cache_dir()?;
    if !project_root.exists() {
        clone_repository(&project_root)?;
    }
    fetch_and_checkout(&project_root)?;
    Ok(project_root)
}

fn project_cache_dir() -> Result<Utf8PathBuf> {
    let target_dir = cargo_target_directory()
        .cloned()
        .unwrap_or_else(|| PathBuf::from("target"));
    let target_dir =
        std::path::absolute(target_dir).context("Failed to construct an absolute path")?;
    let cache_dir = target_dir.join("benchmark_cache").join(PROJECT_NAME);

    if let Some(parent) = cache_dir.parent() {
        std::fs::create_dir_all(parent).context("Failed to create cache directory")?;
    }

    Ok(Utf8PathBuf::from_path_buf(cache_dir).unwrap())
}

fn clone_repository(target_dir: &Utf8PathBuf) -> Result<()> {
    if let Some(parent) = target_dir.parent() {
        std::fs::create_dir_all(parent).context("Failed to create parent directory for clone")?;
    }

    let output = Command::new("git")
        .args([
            "clone",
            "--filter=blob:none",
            "--no-checkout",
            REPOSITORY,
            target_dir.as_ref(),
        ])
        .output()
        .context("Failed to execute git clone command")?;

    anyhow::ensure!(
        output.status.success(),
        "Git clone failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

fn fetch_and_checkout(project_root: &Utf8PathBuf) -> Result<()> {
    let output = Command::new("git")
        .args(["fetch", "origin", COMMIT])
        .current_dir(project_root)
        .output()
        .context("Failed to execute git fetch command")?;

    anyhow::ensure!(
        output.status.success(),
        "Git fetch of commit {COMMIT} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new("git")
        .args(["checkout", COMMIT])
        .current_dir(project_root)
        .output()
        .context("Failed to execute git checkout command")?;

    anyhow::ensure!(
        output.status.success(),
        "Git checkout of commit {COMMIT} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

fn install_dependencies(project_root: &Utf8PathBuf) -> Result<()> {
    let uv_check = Command::new("uv")
        .arg("--version")
        .output()
        .context("Failed to execute uv version check.")?;

    anyhow::ensure!(
        uv_check.status.success(),
        "uv is not installed or not found in PATH. \
         If you need to install it, follow the instructions at \
         https://docs.astral.sh/uv/getting-started/installation/",
    );

    let venv_path = global_venv_path();
    let python_version_str = PYTHON_VERSION.to_string();

    let output = Command::new("uv")
        .args(["venv", "--python", &python_version_str, "--allow-existing"])
        .arg(&venv_path)
        .output()
        .context("Failed to execute uv venv command")?;

    anyhow::ensure!(
        output.status.success(),
        "Failed to create virtual environment: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let output = Command::new("uv")
        .args([
            "pip",
            "install",
            "--python",
            venv_path.to_str().unwrap(),
            "--exclude-newer",
            MAX_DEP_DATE,
            "--no-build",
        ])
        .args(DEPENDENCIES)
        .output()
        .context("Failed to execute uv pip install command")?;

    anyhow::ensure!(
        output.status.success(),
        "Dependency installation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    if let Ok(karva_wheel) = karva_project::find_karva_wheel() {
        let output = Command::new("uv")
            .args(["pip", "install", "--python", venv_path.to_str().unwrap()])
            .arg(karva_wheel)
            .output()
            .context("Failed to execute uv pip install command")?;

        anyhow::ensure!(
            output.status.success(),
            "Karva wheel installation failed: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let output = Command::new("uv")
        .args(["pip", "install", "--python", venv_path.to_str().unwrap()])
        .arg(project_root)
        .output()
        .context("Failed to execute uv pip install command")?;

    anyhow::ensure!(
        output.status.success(),
        "Project installation failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    Ok(())
}

fn global_venv_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join(".venv")
}

static CARGO_TARGET_DIR: std::sync::OnceLock<Option<PathBuf>> = std::sync::OnceLock::new();

fn cargo_target_directory() -> Option<&'static PathBuf> {
    CARGO_TARGET_DIR
        .get_or_init(|| {
            #[derive(serde::Deserialize)]
            struct Metadata {
                target_directory: PathBuf,
            }

            std::env::var_os("CARGO_TARGET_DIR")
                .map(PathBuf::from)
                .or_else(|| {
                    let output = Command::new(std::env::var_os("CARGO")?)
                        .args(["metadata", "--format-version", "1"])
                        .output()
                        .ok()?;
                    let metadata: Metadata = serde_json::from_slice(&output.stdout).ok()?;
                    Some(metadata.target_directory)
                })
        })
        .as_ref()
}
