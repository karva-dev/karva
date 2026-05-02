//! Generate a Markdown reference for environment variables consumed and
//! produced by Karva.

use std::fmt::Write;
use std::path::PathBuf;

use anyhow::bail;
use karva_static::{EnvVarDoc, EnvVars, WorkerEnvVars};
use pretty_assertions::StrComparison;

use crate::{Mode, REGENERATE_ALL_COMMAND, ROOT_DIR};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Write the generated reference to stdout (rather than to `docs/env-vars.md`).
    #[arg(long, default_value_t, value_enum)]
    pub(crate) mode: Mode,
}

const FILE_NAME: &str = "docs/env-vars.md";

const HEADER: &str = "<!-- WARNING: This file is auto-generated (cargo run -p karva_dev generate-all). Update the doc comments on the env-var structs in 'crates/karva_static/src/lib.rs' if you want to change anything here. -->\n\n# Environment Variables\n\nThis page lists every environment variable that Karva reads from the \
environment, plus the variables the worker exposes to running tests.\n\n";

pub(crate) fn main(args: &Args) -> anyhow::Result<()> {
    let output = generate();
    let markdown_path = PathBuf::from(ROOT_DIR).join(FILE_NAME);

    match args.mode {
        Mode::DryRun => {
            println!("{output}");
        }
        Mode::Check => {
            let current = std::fs::read_to_string(&markdown_path)?;
            if output == current {
                println!("Up-to-date: {FILE_NAME}");
            } else {
                let comparison = StrComparison::new(&current, &output);
                bail!("{FILE_NAME} changed, please run `{REGENERATE_ALL_COMMAND}`:\n{comparison}");
            }
        }
        Mode::Write => {
            let current = std::fs::read_to_string(&markdown_path).unwrap_or_default();
            if current == output {
                println!("Up-to-date: {FILE_NAME}");
            } else {
                println!("Updating: {FILE_NAME}");
                std::fs::write(markdown_path, output.as_bytes())?;
            }
        }
    }

    Ok(())
}

fn generate() -> String {
    let mut output = String::new();
    output.push_str(HEADER);

    emit_section(
        &mut output,
        "Read by Karva",
        "Variables Karva reads from the environment to influence its own behavior.",
        EnvVars::METADATA,
    );

    emit_section(
        &mut output,
        "Set by the worker on tests",
        "Variables the Karva worker writes into the test process before each \
         attempt, so running test code can introspect its own retry state.",
        WorkerEnvVars::METADATA,
    );

    output
}

fn emit_section(output: &mut String, title: &str, description: &str, vars: &[EnvVarDoc]) {
    let _ = writeln!(output, "## {title}\n");
    output.push_str(description);
    output.push_str("\n\n");

    for var in vars {
        let _ = writeln!(output, "### `{}`\n", var.name);
        for line in var.doc_lines {
            let trimmed = line.strip_prefix(' ').unwrap_or(line);
            let _ = writeln!(output, "{trimmed}");
        }
        output.push('\n');
    }
}

#[cfg(test)]
mod tests {
    use anyhow::Result;

    use super::{Args, main};
    use crate::Mode;

    #[test]
    #[cfg(unix)]
    fn env_vars_markdown_up_to_date() -> Result<()> {
        main(&Args { mode: Mode::Check })
    }
}
