#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::undocumented_unsafe_blocks
    )
)]

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

    #[test]
    fn archive_entry_modified_time_is_backward_compatible() {
        let legacy = r#"{
            "version": 1,
            "payload": {
                "event": "archive_entry",
                "entry": {
                    "path": "legacy.txt",
                    "size": 1,
                    "compressed_size": 1,
                    "is_directory": false,
                    "encrypted": false
                }
            }
        }"#;
        let decoded: Envelope<WorkerEvent> = serde_json::from_str(legacy).unwrap();
        let Envelope {
            payload: WorkerEvent::ArchiveEntry { entry },
            ..
        } = decoded
        else {
            panic!("legacy event decoded as the wrong variant");
        };
        assert_eq!(entry.modified, None);
        let encoded =
            serde_json::to_string(&Envelope::new(WorkerEvent::ArchiveEntry { entry })).unwrap();
        assert!(!encoded.contains("modified"));
    }
}
