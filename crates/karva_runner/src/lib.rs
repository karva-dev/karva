mod collection;
pub mod coverage;
mod orchestration;
mod partition;
mod shutdown;

pub use orchestration::{ParallelTestConfig, coverage_data_dir, run_parallel_tests};
pub use shutdown::shutdown_receiver;
