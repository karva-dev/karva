use std::fmt;
use std::num::NonZeroU32;
use std::str::FromStr;

use karva_combine::Combine;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Controls how many tests may fail before karva stops scheduling new ones.
///
/// Modelled on nextest's `--max-fail` flag. `All` is the karva default — it
/// lets every test run regardless of failures. `Count(n)` stops scheduling
/// once `n` tests have failed, which generalizes the legacy `--fail-fast`
/// flag (equivalent to `Count(1)`).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MaxFail {
    /// Never stop; run every test regardless of how many fail.
    #[default]
    All,

    /// Stop after this many tests have failed.
    Count(NonZeroU32),
}

impl MaxFail {
    /// Returns a `MaxFail` that stops after the given number of failures,
    /// treating `0` as `All` (never stop).
    pub fn from_count(count: u32) -> Self {
        NonZeroU32::new(count).map_or(Self::All, Self::Count)
    }

    /// The legacy `fail-fast` boolean: `true` maps to stopping after a single
    /// failure, `false` maps to never stopping.
    pub fn from_fail_fast(fail_fast: bool) -> Self {
        if fail_fast {
            Self::from_count(1)
        } else {
            Self::All
        }
    }

    /// Returns `true` when the given number of failures meets or exceeds the
    /// configured budget.
    pub fn is_exceeded_by(self, failures: u32) -> bool {
        match self {
            Self::All => false,
            Self::Count(limit) => failures >= limit.get(),
        }
    }

    /// Returns `true` when this configuration imposes any limit at all.
    ///
    /// Equivalent to "not `MaxFail::All`".
    pub fn has_limit(self) -> bool {
        matches!(self, Self::Count(_))
    }

    /// Returns `true` when the configuration would stop after a single failure.
    ///
    /// This is how the legacy `--fail-fast` boolean is surfaced internally.
    pub fn is_fail_fast(self) -> bool {
        matches!(self, Self::Count(limit) if limit.get() == 1)
    }
}

impl fmt::Display for MaxFail {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::All => f.write_str("all"),
            Self::Count(n) => write!(f, "{n}"),
        }
    }
}

#[derive(Debug, thiserror::Error)]
#[error("invalid `max-fail` value `{input}`: expected a positive integer or `all`")]
pub struct MaxFailParseError {
    input: String,
}

impl FromStr for MaxFail {
    type Err = MaxFailParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let trimmed = s.trim();
        if trimmed.eq_ignore_ascii_case("all") {
            return Ok(Self::All);
        }

        if let Ok(n) = trimmed.parse::<u32>()
            && let Some(nz) = NonZeroU32::new(n)
        {
            return Ok(Self::Count(nz));
        }

        Err(MaxFailParseError {
            input: s.to_string(),
        })
    }
}

impl Serialize for MaxFail {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            Self::All => serializer.serialize_str("all"),
            Self::Count(n) => serializer.serialize_u32(n.get()),
        }
    }
}

impl<'de> Deserialize<'de> for MaxFail {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct Visitor;

        impl serde::de::Visitor<'_> for Visitor {
            type Value = MaxFail;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a positive integer or the string \"all\"")
            }

            fn visit_u64<E: serde::de::Error>(self, value: u64) -> Result<MaxFail, E> {
                u32::try_from(value)
                    .ok()
                    .and_then(NonZeroU32::new)
                    .map(MaxFail::Count)
                    .ok_or_else(|| E::custom("max-fail integer must be a positive u32"))
            }

            fn visit_i64<E: serde::de::Error>(self, value: i64) -> Result<MaxFail, E> {
                u32::try_from(value)
                    .ok()
                    .and_then(NonZeroU32::new)
                    .map(MaxFail::Count)
                    .ok_or_else(|| E::custom("max-fail integer must be a positive u32"))
            }

            fn visit_str<E: serde::de::Error>(self, value: &str) -> Result<MaxFail, E> {
                MaxFail::from_str(value).map_err(E::custom)
            }
        }

        deserializer.deserialize_any(Visitor)
    }
}

impl Combine for MaxFail {
    #[inline(always)]
    fn combine_with(&mut self, _other: Self) {}

    #[inline]
    fn combine(self, _other: Self) -> Self {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_all() {
        assert_eq!(MaxFail::from_str("all").unwrap(), MaxFail::All);
        assert_eq!(MaxFail::from_str("ALL").unwrap(), MaxFail::All);
    }

    #[test]
    fn parse_integer() {
        assert_eq!(MaxFail::from_str("1").unwrap(), MaxFail::from_count(1));
        assert_eq!(MaxFail::from_str("42").unwrap(), MaxFail::from_count(42));
    }

    #[test]
    fn parse_invalid() {
        assert!(MaxFail::from_str("0").is_err());
        assert!(MaxFail::from_str("-1").is_err());
        assert!(MaxFail::from_str("maybe").is_err());
        assert!(MaxFail::from_str("").is_err());
    }

    #[test]
    fn exceeds_budget() {
        let two = MaxFail::from_count(2);
        assert!(!two.is_exceeded_by(0));
        assert!(!two.is_exceeded_by(1));
        assert!(two.is_exceeded_by(2));
        assert!(two.is_exceeded_by(3));

        assert!(!MaxFail::All.is_exceeded_by(u32::MAX));
    }

    #[test]
    fn from_fail_fast() {
        assert_eq!(MaxFail::from_fail_fast(true), MaxFail::from_count(1));
        assert_eq!(MaxFail::from_fail_fast(false), MaxFail::All);
    }
}
