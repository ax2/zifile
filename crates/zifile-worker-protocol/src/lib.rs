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
    ArchiveEntryInfo, ArchiveFormat, ArchiveRename, ConflictPolicy, OperationSummary,
    ProgressSnapshot, SafetyLimits,
};

pub const PROTOCOL_VERSION: u16 = 3;

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
    Update {
        archive: PathBuf,
        additions: Vec<PathBuf>,
        compression_level: u8,
        password: Option<String>,
        limits: SafetyLimits,
        /// Optional archive-relative files or directories to remove. The
        /// default keeps older update requests structurally compatible.
        #[serde(default)]
        remove_paths: Vec<PathBuf>,
    },
    Rename {
        archive: PathBuf,
        renames: Vec<ArchiveRename>,
        compression_level: u8,
        password: Option<String>,
        limits: SafetyLimits,
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
    fn archive_entry_optional_metadata_is_backward_compatible() {
        let legacy = r#"{
            "version": 2,
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
        assert_eq!(entry.checksum, None);
        let encoded =
            serde_json::to_string(&Envelope::new(WorkerEvent::ArchiveEntry { entry })).unwrap();
        assert!(!encoded.contains("modified"));
        assert!(!encoded.contains("checksum"));

        let current = ArchiveEntryInfo {
            path: PathBuf::from("current.txt"),
            size: 3,
            compressed_size: 3,
            is_directory: false,
            encrypted: false,
            checksum: Some(
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            ),
            modified: None,
        };
        let encoded =
            serde_json::to_string(&Envelope::new(WorkerEvent::ArchiveEntry { entry: current }))
                .unwrap();
        assert!(encoded.contains("checksum"));
    }

    #[test]
    fn update_request_without_removals_is_backward_compatible() {
        let legacy = r#"{
            "version": 2,
            "payload": {
                "operation": "update",
                "archive": "sample.zip",
                "additions": ["new.txt"],
                "compression_level": 6,
                "password": null,
                "limits": {
                    "max_entries": 100,
                    "max_path_depth": 8,
                    "max_path_bytes": 4096,
                    "max_expanded_bytes": 1048576,
                    "max_expansion_ratio": 1000,
                    "max_compression_ratio": 100
                }
            }
        }"#;
        let decoded: Envelope<WorkerRequest> = serde_json::from_str(legacy).unwrap();
        let Envelope {
            payload:
                WorkerRequest::Update {
                    remove_paths,
                    additions,
                    ..
                },
            ..
        } = decoded
        else {
            panic!("legacy request decoded as the wrong variant");
        };
        assert!(remove_paths.is_empty());
        assert_eq!(additions, [PathBuf::from("new.txt")]);
    }

    #[test]
    fn rename_request_round_trips_archive_relative_mappings() {
        let envelope = Envelope::new(WorkerRequest::Rename {
            archive: PathBuf::from("sample.zip"),
            renames: vec![ArchiveRename {
                from: PathBuf::from("old.txt"),
                to: PathBuf::from("new.txt"),
            }],
            compression_level: 6,
            password: None,
            limits: SafetyLimits::default(),
        });
        let encoded = serde_json::to_string(&envelope).unwrap();
        let decoded: Envelope<WorkerRequest> = serde_json::from_str(&encoded).unwrap();
        let WorkerRequest::Rename { renames, .. } = decoded.payload else {
            panic!("rename request decoded as the wrong variant");
        };
        assert_eq!(renames[0].from, PathBuf::from("old.txt"));
        assert_eq!(renames[0].to, PathBuf::from("new.txt"));
    }
}
