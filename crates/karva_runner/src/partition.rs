use std::collections::{HashMap, HashSet};

/// Test metadata used for partitioning decisions
#[derive(Debug, Clone)]
struct TestInfo {
    module_name: String,
    /// The qualified name of the test (e.g., `test_a::test_1`), used for last-failed filtering.
    qualified_name: String,
    path: String,
}

/// A partition of tests assigned to a single worker
#[derive(Debug)]
pub struct Partition {
    tests: Vec<String>,
}

impl Partition {
    fn new() -> Self {
        Self { tests: Vec::new() }
    }

    fn add_test(&mut self, test: TestInfo) {
        self.tests.push(test.path);
    }

    fn len(&self) -> usize {
        self.tests.len()
    }

    pub(crate) fn tests(&self) -> &[String] {
        &self.tests
    }
}

/// Partition collected tests into N groups using module-aware greedy bin-packing.
///
/// Tests from the same module stay together in one partition when the module is
/// small, so each worker imports fewer unique modules and shares fixture setup.
/// Large modules (with more tests than `total / workers / 2`) are split across
/// workers to keep partition sizes balanced.
///
/// Every test is weighted equally — the runner previously persisted per-test
/// durations to weight this smarter, but the added complexity wasn't paying off.
pub fn partition_collected_tests(
    package: &karva_collector::CollectedPackage,
    num_workers: usize,
    last_failed: &HashSet<String>,
) -> Vec<Partition> {
    let mut test_infos = Vec::new();
    collect_test_paths_recursive(package, &mut test_infos);

    if !last_failed.is_empty() {
        test_infos.retain(|info| last_failed.contains(&info.qualified_name));
    }

    shuffle_tests(&mut test_infos);

    let mut module_groups: HashMap<String, Vec<TestInfo>> = HashMap::new();
    for test_info in test_infos {
        module_groups
            .entry(test_info.module_name.clone())
            .or_default()
            .push(test_info);
    }

    let total_tests: usize = module_groups.values().map(Vec::len).sum();
    let split_threshold = total_tests / num_workers.max(1) / 2;

    let mut small_modules: Vec<Vec<TestInfo>> = Vec::new();
    let mut large_modules: Vec<Vec<TestInfo>> = Vec::new();

    for (_, tests) in module_groups {
        if tests.len() < split_threshold {
            small_modules.push(tests);
        } else {
            large_modules.push(tests);
        }
    }

    // Pack heaviest small modules first for a better bin-packing fit.
    small_modules.sort_by_key(|tests| std::cmp::Reverse(tests.len()));

    let mut partitions: Vec<Partition> = (0..num_workers).map(|_| Partition::new()).collect();

    for tests in small_modules {
        let idx = find_lightest_partition(&partitions);
        for test_info in tests {
            partitions[idx].add_test(test_info);
        }
    }

    for tests in large_modules {
        for test_info in tests {
            let idx = find_lightest_partition(&partitions);
            partitions[idx].add_test(test_info);
        }
    }

    partitions
}

/// Finds the index of the partition with the fewest tests.
fn find_lightest_partition(partitions: &[Partition]) -> usize {
    partitions
        .iter()
        .enumerate()
        .min_by_key(|(_, partition)| partition.len())
        .map_or(0, |(idx, _)| idx)
}

/// Shuffles tests so they distribute randomly across partitions rather than
/// always landing in discovery order.
fn shuffle_tests(test_infos: &mut [TestInfo]) {
    for i in (1..test_infos.len()).rev() {
        let j = fastrand::usize(..=i);
        test_infos.swap(i, j);
    }
}

/// Recursively collects test information from a package and all its subpackages.
fn collect_test_paths_recursive(
    package: &karva_collector::CollectedPackage,
    test_infos: &mut Vec<TestInfo>,
) {
    for module in package.modules.values() {
        for test_fn_def in &module.test_function_defs {
            test_infos.push(TestInfo {
                module_name: module.path.module_name().to_string(),
                qualified_name: format!("{}::{}", module.path.module_name(), test_fn_def.name),
                path: format!("{}::{}", module.path.path(), test_fn_def.name),
            });
        }
    }

    for subpackage in package.packages.values() {
        collect_test_paths_recursive(subpackage, test_infos);
    }
}
