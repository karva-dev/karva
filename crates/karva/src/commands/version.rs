use std::fmt::Write;

use anyhow::Result;
use karva_logging::Printer;

pub fn version() -> Result<()> {
    let mut stdout = Printer::default().stream_for_requested_summary().lock();
    if let Some(version_info) = crate::version::version() {
        writeln!(stdout, "karva {}", &version_info)?;
    } else {
        writeln!(stdout, "Failed to get karva version")?;
    }

    Ok(())
}
