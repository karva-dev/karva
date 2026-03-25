use std::time::Duration;

pub fn format_duration(duration: Duration) -> String {
    if duration.as_secs() < 2 {
        format!("{}ms", duration.as_millis())
    } else {
        format!("{:.2}s", duration.as_secs_f64())
    }
}

/// Format a duration in nextest-style bracketed format: `[   0.015s]`.
pub fn format_duration_bracketed(duration: Duration) -> String {
    format!("[{:>8.3}s]", duration.as_secs_f64())
}
