use std::collections::{HashMap, HashSet};

/// A partition of tests assigned to a single worker.
#[derive(Debug)]
pub struct Partition {
    tests: Vec<String>,
}

impl Partition {
    fn new() -> Self {
        Self { tests: Vec::new() }
    }

    fn len(&self) -> usize {
        self.tests.len()
    }

    pub(crate) fn tests(&self) -> &[String] {
        &self.tests
    }
}

/// Partition collected tests into `num_workers` groups.
///
/// Tests are grouped by module and modules are sorted by test count
/// (descending). Each module is then assigned whole to the lightest
/// partition, so a worker shares module-level imports and fixture setup.
/// A module larger than the per-worker fair share (`total / num_workers`)
/// would strand other workers if kept atomic, so those are split test-by-test
/// across the lightest partitions instead.
pub fn partition_collected_tests(
    package: &karva_collector::CollectedPackage,
    num_workers: usize,
    last_failed: &HashSet<String>,
) -> Vec<Partition> {
    let mut module_groups: HashMap<String, Vec<String>> = HashMap::new();
    collect_module_tests(package, &mut module_groups, last_failed);

    let mut modules: Vec<Vec<String>> = module_groups.into_values().collect();
    modules.sort_by_key(|tests| std::cmp::Reverse(tests.len()));

    let num_workers = num_workers.max(1);
    let total_tests: usize = modules.iter().map(Vec::len).sum();
    let split_threshold = total_tests / num_workers;

    let mut partitions: Vec<Partition> = (0..num_workers).map(|_| Partition::new()).collect();

    for tests in modules {
        if tests.len() > split_threshold {
            for test in tests {
                let idx = lightest_partition(&partitions);
                partitions[idx].tests.push(test);
            }
        } else {
            let idx = lightest_partition(&partitions);
            partitions[idx].tests.extend(tests);
        }
    }

    partitions
}

/// Finds the index of the partition with the fewest tests.
fn lightest_partition(partitions: &[Partition]) -> usize {
    partitions
        .iter()
        .enumerate()
        .min_by_key(|(_, partition)| partition.len())
        .map_or(0, |(idx, _)| idx)
}

/// Walk the package tree and group test paths by their containing module.
fn collect_module_tests(
    package: &karva_collector::CollectedPackage,
    module_groups: &mut HashMap<String, Vec<String>>,
    last_failed: &HashSet<String>,
) {
    for module in package.modules.values() {
        let module_name = module.path.module_name();
        for test_fn_def in &module.test_function_defs {
            let qualified_name = format!("{module_name}::{}", test_fn_def.name);
            if !last_failed.is_empty() && !last_failed.contains(&qualified_name) {
                continue;
            }
            let path = format!("{}::{}", module.path.path(), test_fn_def.name);
            module_groups
                .entry(module_name.to_string())
                .or_default()
                .push(path);
        }
    }

    for subpackage in package.packages.values() {
        collect_module_tests(subpackage, module_groups, last_failed);
    }
}
