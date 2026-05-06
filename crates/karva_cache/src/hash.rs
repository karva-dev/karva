use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

use uuid::Uuid;

use crate::RUN_PREFIX;

/// A unique identifier for a test run.
///
/// Combines a millisecond timestamp (for chronological ordering of cache
/// directories) with a UUID v4 (for uniqueness across dense CI matrices and
/// for correlating logs across worker processes). Serialized as
/// `<ms>-<uuid>`; the cache directory adds the `run-` prefix.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunHash {
    timestamp: u128,
    uuid: Uuid,
}

impl RunHash {
    /// Creates a new identifier for the current invocation.
    pub fn current_time() -> Self {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("System time is before UNIX epoch")
            .as_millis();

        Self {
            timestamp,
            uuid: Uuid::new_v4(),
        }
    }

    /// Parses a hash from an existing run directory name (e.g.
    /// `run-1234-<uuid>`) or its bare `<ms>-<uuid>` form.
    ///
    /// Falls back to a zero timestamp and nil UUID if the input cannot be
    /// parsed; this keeps callers from having to handle malformed legacy
    /// directories that may exist on disk.
    pub fn from_existing(hash: &str) -> Self {
        let inner = hash.strip_prefix(RUN_PREFIX).unwrap_or(hash);
        let (ts_str, uuid_str) = inner.split_once('-').unwrap_or((inner, ""));
        let timestamp = ts_str.parse().unwrap_or(0);
        let uuid = Uuid::parse_str(uuid_str).unwrap_or(Uuid::nil());
        Self { timestamp, uuid }
    }

    /// Returns the bare `<ms>-<uuid>` form. This is the value exposed to
    /// tests as `KARVA_RUN_ID` and passed between processes.
    pub fn inner(&self) -> String {
        format!("{}-{}", self.timestamp, self.uuid)
    }

    /// Returns the directory name used in the cache (`run-<ms>-<uuid>`).
    pub fn dir_name(&self) -> String {
        format!("{RUN_PREFIX}{}", self.inner())
    }

    /// Returns the underlying timestamp, used for ordering runs chronologically.
    pub fn sort_key(&self) -> u128 {
        self.timestamp
    }
}

impl fmt::Display for RunHash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.dir_name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn current_time_produces_valid_hash() {
        let hash = RunHash::current_time();
        let dir = hash.dir_name();
        assert!(dir.starts_with("run-"));
        assert!(hash.sort_key() > 0);
        assert!(dir.contains('-'));
    }

    #[test]
    fn from_existing_roundtrips_with_dir_name() {
        let original = RunHash::current_time();
        let restored = RunHash::from_existing(&original.dir_name());
        assert_eq!(original, restored);
    }

    #[test]
    fn from_existing_roundtrips_with_inner() {
        let original = RunHash::current_time();
        let restored = RunHash::from_existing(&original.inner());
        assert_eq!(original, restored);
    }

    #[test]
    fn from_existing_handles_missing_prefix() {
        let hash = RunHash::from_existing("not-a-number");
        assert_eq!(hash.sort_key(), 0);
    }

    #[test]
    fn from_existing_handles_invalid_input() {
        let hash = RunHash::from_existing("run-abc");
        assert_eq!(hash.sort_key(), 0);
    }

    #[test]
    fn from_existing_handles_legacy_timestamp_only_dir() {
        let hash = RunHash::from_existing("run-42");
        assert_eq!(hash.sort_key(), 42);
    }

    #[test]
    fn sort_key_reflects_timestamp_ordering() {
        let earlier = RunHash::from_existing("run-100-00000000-0000-4000-8000-000000000000");
        let later = RunHash::from_existing("run-200-00000000-0000-4000-8000-000000000000");
        assert!(earlier.sort_key() < later.sort_key());
    }

    #[test]
    fn display_matches_dir_name() {
        let hash = RunHash::current_time();
        assert_eq!(hash.to_string(), hash.dir_name());
    }

    #[test]
    fn two_invocations_produce_distinct_hashes_even_at_same_ms() {
        let a = RunHash::current_time();
        let b = RunHash::current_time();
        assert_ne!(a, b);
    }
}
