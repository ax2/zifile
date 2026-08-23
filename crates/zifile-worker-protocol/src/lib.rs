use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use zifile_core::{
    ArchiveEntryInfo, ArchiveFormat, ConflictPolicy, OperationSummary, ProgressSnapshot,
    SafetyLimits,
};

pub const PROTOCOL_VERSION: u16 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Envelope<T> {
    pub version: u16,
    pub payload: T,
}

impl<T> Envelope<T> {
    pub const fn new(payload: T) -> Self {
        Self {
            version: PROTOCOL_VERSION,
            payload,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "operation", rename_all = "snake_case")]
pub enum WorkerRequest {
    List {
        archive: PathBuf,
        password: Option<String>,
    },
    Test {
        archive: PathBuf,
        password: Option<String>,
    },
    Extract {
        archive: PathBuf,
        destination: PathBuf,
        conflict: ConflictPolicy,
        limits: SafetyLimits,
        password: Option<String>,
        selected_paths: Option<Vec<PathBuf>>,
    },
    Create {
        sources: Vec<PathBuf>,
        destination: PathBuf,
        format: ArchiveFormat,
        compression_level: u8,
        password: Option<String>,
    },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(tag = "control", rename_all = "snake_case")]
pub enum WorkerControl {
    Cancel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "event", rename_all = "snake_case")]
pub enum WorkerEvent {
    ArchiveStart {
        path: PathBuf,
        format: ArchiveFormat,
        total_size: u64,
        compressed_size: u64,
    },
    ArchiveEntry {
        entry: ArchiveEntryInfo,
    },
    ArchiveEnd,
    Progress {
        snapshot: ProgressSnapshot,
    },
    Summary {
        summary: OperationSummary,
    },
    Error {
        message: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn protocol_envelope_has_an_explicit_version() {
        let envelope = Envelope::new(WorkerRequest::List {
            archive: PathBuf::from("sample.zip"),
            password: None,
        });
        assert_eq!(envelope.version, PROTOCOL_VERSION);
    }
}
