use std::io::StdoutLock;

use crate::status_level::{FinalStatusLevel, StatusLevel};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Printer {
    status_level: StatusLevel,
    final_status_level: FinalStatusLevel,
}

impl Printer {
    pub fn new(status_level: StatusLevel, final_status_level: FinalStatusLevel) -> Self {
        Self {
            status_level,
            final_status_level,
        }
    }

    pub fn status_level(self) -> StatusLevel {
        self.status_level
    }

    pub fn final_status_level(self) -> FinalStatusLevel {
        self.final_status_level
    }

    /// Stream for the "Starting N tests" header and per-test result lines.
    ///
    /// The reporter additionally filters individual results by [`StatusLevel`].
    pub fn stream_for_test_result(self) -> Stdout {
        if matches!(self.status_level, StatusLevel::None) {
            Stdout::disabled()
        } else {
            Stdout::enabled()
        }
    }

    /// Stream for the end-of-run summary line.
    ///
    /// `success` is true when no tests failed. `had_retries` is true when at
    /// least one test was retried; it elevates `final-status-level=retry` (or
    /// higher) to show the summary even when all tests eventually passed.
    pub fn stream_for_summary(self, success: bool, had_retries: bool) -> Stdout {
        match self.final_status_level {
            FinalStatusLevel::None => Stdout::disabled(),
            FinalStatusLevel::Fail if success => Stdout::disabled(),
            FinalStatusLevel::Retry | FinalStatusLevel::Slow if success && !had_retries => {
                Stdout::disabled()
            }
            FinalStatusLevel::Fail
            | FinalStatusLevel::Retry
            | FinalStatusLevel::Slow
            | FinalStatusLevel::Pass
            | FinalStatusLevel::Skip
            | FinalStatusLevel::All => Stdout::enabled(),
        }
    }

    /// Stream for the diagnostic block (tracebacks, durations) at the end of the run.
    pub fn stream_for_details(self) -> Stdout {
        match self.final_status_level {
            FinalStatusLevel::None => Stdout::disabled(),
            FinalStatusLevel::Fail
            | FinalStatusLevel::Retry
            | FinalStatusLevel::Slow
            | FinalStatusLevel::Pass
            | FinalStatusLevel::Skip
            | FinalStatusLevel::All => Stdout::enabled(),
        }
    }

    /// Stream for messages explicitly requested by the user, such as
    /// `warning: no tests to run`. Suppressed only when both status levels are `none`.
    pub fn stream_for_message(self) -> Stdout {
        if matches!(self.status_level, StatusLevel::None)
            && matches!(self.final_status_level, FinalStatusLevel::None)
        {
            Stdout::disabled()
        } else {
            Stdout::enabled()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamStatus {
    Enabled,
    Disabled,
}

#[derive(Debug)]
pub struct Stdout {
    status: StreamStatus,
    lock: Option<StdoutLock<'static>>,
}

impl Stdout {
    fn enabled() -> Self {
        Self {
            status: StreamStatus::Enabled,
            lock: None,
        }
    }

    fn disabled() -> Self {
        Self {
            status: StreamStatus::Disabled,
            lock: None,
        }
    }

    #[must_use]
    pub fn lock(mut self) -> Self {
        match self.status {
            StreamStatus::Enabled => {
                self.lock.take();
                self.lock = Some(std::io::stdout().lock());
            }
            StreamStatus::Disabled => self.lock = None,
        }
        self
    }

    fn handle(&mut self) -> Box<dyn std::io::Write + '_> {
        match self.lock.as_mut() {
            Some(lock) => Box::new(lock),
            None => Box::new(std::io::stdout()),
        }
    }

    pub fn is_enabled(&self) -> bool {
        matches!(self.status, StreamStatus::Enabled)
    }
}

impl std::fmt::Write for Stdout {
    fn write_str(&mut self, s: &str) -> std::fmt::Result {
        match self.status {
            StreamStatus::Enabled => {
                let _ = write!(self.handle(), "{s}");
                Ok(())
            }
            StreamStatus::Disabled => Ok(()),
        }
    }
}

impl From<Stdout> for std::process::Stdio {
    fn from(val: Stdout) -> Self {
        match val.status {
            StreamStatus::Enabled => Self::inherit(),
            StreamStatus::Disabled => Self::null(),
        }
    }
}
