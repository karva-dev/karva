//! Per-worker JSON schema. Both the tracer and the report side use these
//! types so the wire format stays in lockstep.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

pub const WORKER_FILE_PREFIX: &str = "karva-coverage.";
pub const WORKER_FILE_SUFFIX: &str = ".json";

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkerFile {
    pub files: BTreeMap<String, FileEntry>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub executable: Vec<u32>,
    pub executed: Vec<u32>,
}
