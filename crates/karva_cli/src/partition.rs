use std::fmt;
use std::str::FromStr;

/// Selection of a single partition (slice) from the collected tests.
///
/// Used by `--partition slice:M/N` to run only the tests assigned to slice
/// `M` of `N`. Slice indices are 1-indexed: `slice:1/3`, `slice:2/3`,
/// `slice:3/3` together cover every collected test exactly once.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PartitionSelection {
    pub index: u32,
    pub total: u32,
}

impl PartitionSelection {
    /// Returns true if the test at `position` (0-indexed, in the deterministic
    /// post-filter ordering) belongs to this slice.
    #[must_use]
    pub fn contains(self, position: usize) -> bool {
        // 1-indexed input -> 0-indexed modulo target.
        let target = (self.index - 1) as usize;
        let total = self.total as usize;
        position % total == target
    }
}

impl fmt::Display for PartitionSelection {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "slice:{}/{}", self.index, self.total)
    }
}

impl FromStr for PartitionSelection {
    type Err = String;

    fn from_str(raw: &str) -> Result<Self, Self::Err> {
        let (kind, body) = raw.split_once(':').ok_or_else(|| {
            format!("expected `<strategy>:<M>/<N>` (e.g. `slice:1/3`), got `{raw}`")
        })?;

        if kind != "slice" {
            return Err(format!(
                "unknown partition strategy `{kind}`; supported strategies: `slice`"
            ));
        }

        let (m, n) = body
            .split_once('/')
            .ok_or_else(|| format!("expected `slice:<M>/<N>`, got `slice:{body}`"))?;

        let index: u32 = m
            .parse()
            .map_err(|err| format!("`{m}` is not a valid partition index: {err}"))?;
        let total: u32 = n
            .parse()
            .map_err(|err| format!("`{n}` is not a valid partition count: {err}"))?;

        if total == 0 {
            return Err("partition count `N` must be at least 1".to_string());
        }
        if index == 0 {
            return Err("partition index `M` must be at least 1".to_string());
        }
        if index > total {
            return Err(format!(
                "partition index `M` ({index}) must not exceed partition count `N` ({total})"
            ));
        }

        Ok(Self { index, total })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_slice() {
        assert_eq!(
            "slice:1/3".parse::<PartitionSelection>().unwrap(),
            PartitionSelection { index: 1, total: 3 },
        );
        assert_eq!(
            "slice:3/3".parse::<PartitionSelection>().unwrap(),
            PartitionSelection { index: 3, total: 3 },
        );
        assert_eq!(
            "slice:1/1".parse::<PartitionSelection>().unwrap(),
            PartitionSelection { index: 1, total: 1 },
        );
    }

    #[test]
    fn rejects_zero_total() {
        assert!("slice:1/0".parse::<PartitionSelection>().is_err());
    }

    #[test]
    fn rejects_zero_index() {
        assert!("slice:0/3".parse::<PartitionSelection>().is_err());
    }

    #[test]
    fn rejects_index_above_total() {
        assert!("slice:4/3".parse::<PartitionSelection>().is_err());
    }

    #[test]
    fn rejects_unknown_strategy() {
        assert!("hash:1/3".parse::<PartitionSelection>().is_err());
    }

    #[test]
    fn rejects_missing_separators() {
        assert!("slice".parse::<PartitionSelection>().is_err());
        assert!("slice:13".parse::<PartitionSelection>().is_err());
        assert!("1/3".parse::<PartitionSelection>().is_err());
    }

    #[test]
    fn contains_round_robin() {
        let p = PartitionSelection { index: 1, total: 3 };
        assert!(p.contains(0));
        assert!(!p.contains(1));
        assert!(!p.contains(2));
        assert!(p.contains(3));

        let q = PartitionSelection { index: 3, total: 3 };
        assert!(!q.contains(0));
        assert!(!q.contains(1));
        assert!(q.contains(2));
        assert!(q.contains(5));
    }

    #[test]
    fn display_round_trip() {
        let p = PartitionSelection { index: 2, total: 5 };
        assert_eq!(p.to_string(), "slice:2/5");
        assert_eq!(p.to_string().parse::<PartitionSelection>().unwrap(), p);
    }
}
