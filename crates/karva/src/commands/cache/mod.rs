mod clean;
mod prune;

use anyhow::Result;
use karva_cli::{CacheAction, CacheCommand};
use karva_logging::Printer;

use crate::ExitStatus;
use crate::utils::cwd;

pub fn cache(args: &CacheCommand) -> Result<ExitStatus> {
    let cwd = cwd()?;

    let printer = Printer::default();
    let mut stdout = printer.stream_for_message().lock();

    match args.action {
        CacheAction::Prune => prune::prune(&cwd, &mut stdout),
        CacheAction::Clean => clean::clean(&cwd, &mut stdout),
    }
}
