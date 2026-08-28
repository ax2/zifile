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

//! Core domain model for ZiFile.
//!
//! This crate deliberately contains no UI or platform code. Archive backends
//! will implement the provider contract here so the desktop and CLI surfaces
//! share identical capability and safety behavior.

use std::fmt;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use serde::{Deserialize, Serialize};
use thiserror::Error;

mod archive;
mod path_policy;

pub use archive::{
    ArchiveEntryInfo, ArchiveInfo, ArchiveTimestamp, ArchiveTimestampOffset,
    ArchiveTimestampPrecision, CancellationToken, ConflictPolicy, CreateOptions, ExtractOptions,
    ListOptions, OperationProgress, OperationSummary, ProgressSnapshot, TestOptions,
    create_archive, extract_archive, list_archive, list_archive_with_limits,
    list_archive_with_options, test_archive, test_archive_with_limits, test_archive_with_options,
};
pub use path_policy::safe_relative_path;

/// Archive and compression formats in the ZiFile product roadmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ArchiveFormat {
    Zip,
    SevenZip,
    Tar,
    TarGzip,
    TarZstd,
    TarXz,
    TarBzip2,
    Gzip,
    Zstandard,
    Xz,
    Bzip2,
    Lz4,
    Brotli,
    Rar,
    Cab,
}

/// Extensions shown by desktop open dialogs for formats ZiFile can inspect.
/// Existing files are still detected by content signatures before hints.
pub const OPEN_ARCHIVE_EXTENSIONS: &[&str] = &[
    "zip", "zipx", "cbz", "epub", "7z", "cb7", "rar", "cbr", "cab", "tar", "cbt", "gz", "tgz",
    "zst", "tzst", "xz", "txz", "lzma", "bz", "bz2", "tbz", "tbz2", "lz4", "br",
];

impl ArchiveFormat {
    /// Stable display order used by both CLI and desktop UI.
    pub const ALL: [Self; 15] = [
        Self::Zip,
        Self::SevenZip,
        Self::Tar,
        Self::TarGzip,
        Self::TarZstd,
        Self::TarXz,
        Self::TarBzip2,
        Self::Gzip,
        Self::Zstandard,
        Self::Xz,
        Self::Bzip2,
        Self::Lz4,
        Self::Brotli,
        Self::Rar,
        Self::Cab,
    ];

    pub const fn capabilities(self) -> FormatCapabilities {
        match self {
            Self::Rar => FormatCapabilities::read_only(true, ReleaseStage::Beta),
            Self::Cab => FormatCapabilities::read_only(false, ReleaseStage::Beta),
            Self::SevenZip => FormatCapabilities::read_write(true, ReleaseStage::Alpha),
            Self::Zip => FormatCapabilities::read_write(true, ReleaseStage::Alpha),
            Self::Tar | Self::TarGzip | Self::TarZstd | Self::TarXz | Self::TarBzip2 => {
                FormatCapabilities::read_write(false, ReleaseStage::Alpha)
            }
            Self::Gzip | Self::Zstandard | Self::Xz | Self::Bzip2 | Self::Lz4 | Self::Brotli => {
                FormatCapabilities::read_write(false, ReleaseStage::Alpha)
            }
        }
    }

    /// Source shape accepted when creating this format.
    pub const fn create_input(self) -> Option<CreateInputKind> {
        match self {
            Self::Rar | Self::Cab => None,
            Self::Gzip | Self::Zstandard | Self::Xz | Self::Bzip2 | Self::Lz4 | Self::Brotli => {
                Some(CreateInputKind::SingleFile)
            }
            Self::Zip
            | Self::SevenZip
            | Self::Tar
            | Self::TarGzip
            | Self::TarZstd
            | Self::TarXz
            | Self::TarBzip2 => Some(CreateInputKind::FilesAndDirectories),
        }
    }

    /// Inclusive compression-level bounds exposed to callers when the selected
    /// encoder has a user-adjustable level. Formats with fixed compression or
    /// no compression return `None` so UIs do not present a control that has no
    /// effect.
    pub const fn compression_level_range(self) -> Option<(u8, u8)> {
        match self {
            Self::Zip | Self::SevenZip | Self::TarGzip | Self::TarXz | Self::Gzip | Self::Xz => {
                Some((0, 9))
            }
            Self::TarZstd | Self::Zstandard => Some((0, 22)),
            Self::TarBzip2 | Self::Bzip2 => Some((1, 9)),
            Self::Brotli => Some((0, 11)),
            Self::Tar | Self::Lz4 | Self::Rar | Self::Cab => None,
        }
    }

    /// Clamp a persisted or externally supplied level to this format's
    /// supported range. Fixed-level formats preserve the value because their
    /// encoders intentionally ignore it.
    pub const fn clamp_compression_level(self, level: u8) -> u8 {
        match self.compression_level_range() {
            Some((minimum, _)) if level < minimum => minimum,
            Some((_, maximum)) if level > maximum => maximum,
            _ => level,
        }
    }

    pub const fn canonical_extension(self) -> &'static str {
        match self {
            Self::Zip => "zip",
            Self::SevenZip => "7z",
            Self::Tar => "tar",
            Self::TarGzip => "tar.gz",
            Self::TarZstd => "tar.zst",
            Self::TarXz => "tar.xz",
            Self::TarBzip2 => "tar.bz2",
            Self::Gzip => "gz",
            Self::Zstandard => "zst",
            Self::Xz => "xz",
            Self::Bzip2 => "bz2",
            Self::Lz4 => "lz4",
            Self::Brotli => "br",
            Self::Rar => "rar",
            Self::Cab => "cab",
        }
    }
}

/// Source shape supported by a format during archive creation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateInputKind {
    FilesAndDirectories,
    SingleFile,
}

impl fmt::Display for ArchiveFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Zip => "ZIP",
            Self::SevenZip => "7z",
            Self::Tar => "TAR",
            Self::TarGzip => "TAR + gzip",
            Self::TarZstd => "TAR + Zstandard",
            Self::TarXz => "TAR + XZ",
            Self::TarBzip2 => "TAR + Bzip2",
            Self::Gzip => "gzip",
            Self::Zstandard => "Zstandard",
            Self::Xz => "XZ/LZMA",
            Self::Bzip2 => "Bzip2",
            Self::Lz4 => "LZ4",
            Self::Brotli => "Brotli",
            Self::Rar => "RAR",
            Self::Cab => "CAB",
        })
    }
}

/// Planned delivery stage for a format provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReleaseStage {
    Alpha,
    Beta,
    PostV1,
}

impl fmt::Display for ReleaseStage {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Alpha => "Alpha",
            Self::Beta => "Beta",
            Self::PostV1 => "Post-1.0",
        })
    }
}

/// User-visible capabilities for one format provider.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatCapabilities {
    pub list: bool,
    pub extract: bool,
    pub create: bool,
    pub encryption: bool,
    pub stage: ReleaseStage,
}

impl FormatCapabilities {
    const fn read_only(encryption: bool, stage: ReleaseStage) -> Self {
        Self {
            list: true,
            extract: true,
            create: false,
            encryption,
            stage,
        }
    }

    const fn read_write(encryption: bool, stage: ReleaseStage) -> Self {
        Self {
            list: true,
            extract: true,
            create: true,
            encryption,
            stage,
        }
    }
}

/// Conservative defaults applied before any archive entry is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SafetyLimits {
    pub max_entries: u64,
    pub max_expanded_bytes: u64,
    pub max_expansion_ratio: u64,
    pub max_path_depth: u16,
}

impl Default for SafetyLimits {
    fn default() -> Self {
        Self {
            max_entries: 1_000_000,
            max_expanded_bytes: 512 * 1024 * 1024 * 1024,
            max_expansion_ratio: 1_000,
            max_path_depth: 128,
        }
    }
}

#[derive(Debug, Error)]
pub enum ZiFileError {
    #[error("the archive format could not be identified")]
    UnknownFormat,
    #[error("the requested operation is not supported for {0}")]
    UnsupportedOperation(ArchiveFormat),
    #[error("encryption is not supported for {0}")]
    UnsupportedEncryption(ArchiveFormat),
    #[error("a password is required to open this archive")]
    PasswordRequired,
    #[error("the archive contains an unsafe path: {0}")]
    UnsafePath(String),
    #[error("symbolic and hard-link entries are not extracted: {0}")]
    LinkEntry(String),
    #[error("the archive contains an unsupported special entry: {0}")]
    UnsupportedEntry(String),
    #[error("a configured safety limit was exceeded: {0}")]
    LimitExceeded(String),
    #[error("two entries collide on a Windows filesystem: {0}")]
    NameCollision(std::path::PathBuf),
    #[error("the destination already exists: {0}")]
    DestinationExists(std::path::PathBuf),
    #[error("the caller must select a non-interactive conflict policy")]
    ConflictPolicyRequired,
    #[error("invalid input: {0}")]
    InvalidInput(String),
    #[error("archive backend error: {0}")]
    Backend(String),
    #[error("operation cancelled")]
    Cancelled,
    #[error(transparent)]
    Io(#[from] std::io::Error),
    #[error(transparent)]
    Zip(#[from] zip::result::ZipError),
    #[error(transparent)]
    SevenZip(#[from] sevenz_rust2::Error),
}

pub type ZiFileResult<T> = Result<T, ZiFileError>;

/// Detects a format from its signature, using the extension only where a
/// stream format and a TAR composition share the same signature.
pub fn detect_format(path: impl AsRef<Path>) -> ZiFileResult<ArchiveFormat> {
    let path = path.as_ref();
    let mut file = File::open(path)?;
    let mut header = [0_u8; 512];
    let count = file.read(&mut header)?;
    let bytes = header.get(..count).ok_or_else(|| {
        ZiFileError::Backend(
            "reader returned more bytes than the supplied header buffer".to_owned(),
        )
    })?;
    let extension_hint = detect_format_from_path(path);

    let detected = if bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(b"PK\x05\x06")
        || bytes.starts_with(b"PK\x07\x08")
    {
        Some(ArchiveFormat::Zip)
    } else if bytes.starts_with(b"7z\xBC\xAF\x27\x1C") {
        Some(ArchiveFormat::SevenZip)
    } else if bytes.starts_with(b"Rar!\x1A\x07") || bytes.starts_with(b"RE~^") {
        Some(ArchiveFormat::Rar)
    } else if bytes.starts_with(b"MSCF") {
        Some(ArchiveFormat::Cab)
    } else if bytes.starts_with(b"\x1F\x8B") {
        Some(if extension_hint == Some(ArchiveFormat::TarGzip) {
            ArchiveFormat::TarGzip
        } else {
            ArchiveFormat::Gzip
        })
    } else if bytes.starts_with(b"\x28\xB5\x2F\xFD") {
        Some(if extension_hint == Some(ArchiveFormat::TarZstd) {
            ArchiveFormat::TarZstd
        } else {
            ArchiveFormat::Zstandard
        })
    } else if bytes.starts_with(b"\xFD7zXZ\x00") {
        Some(if extension_hint == Some(ArchiveFormat::TarXz) {
            ArchiveFormat::TarXz
        } else {
            ArchiveFormat::Xz
        })
    } else if bytes.starts_with(b"BZh") {
        Some(if extension_hint == Some(ArchiveFormat::TarBzip2) {
            ArchiveFormat::TarBzip2
        } else {
            ArchiveFormat::Bzip2
        })
    } else if bytes.starts_with(b"\x04\x22\x4D\x18") {
        Some(ArchiveFormat::Lz4)
    } else if bytes.get(257..262) == Some(b"ustar") {
        Some(ArchiveFormat::Tar)
    } else if extension_hint == Some(ArchiveFormat::Brotli) {
        // Brotli intentionally has no universal magic bytes.
        Some(ArchiveFormat::Brotli)
    } else {
        None
    };
    detected.ok_or(ZiFileError::UnknownFormat)
}

/// Detect a format from a path without opening the file.
///
/// This extension-only helper is suitable for save dialogs. Opening existing
/// archives should use [`detect_format`] so renamed files are identified by
/// their content whenever the format defines a signature.
pub fn detect_format_from_path(path: impl AsRef<Path>) -> Option<ArchiveFormat> {
    let name = path
        .as_ref()
        .file_name()?
        .to_string_lossy()
        .to_ascii_lowercase();

    let compound = [
        (".tar.gz", ArchiveFormat::TarGzip),
        (".tgz", ArchiveFormat::TarGzip),
        (".tar.zst", ArchiveFormat::TarZstd),
        (".tzst", ArchiveFormat::TarZstd),
        (".tar.xz", ArchiveFormat::TarXz),
        (".txz", ArchiveFormat::TarXz),
        (".tar.bz2", ArchiveFormat::TarBzip2),
        (".tbz2", ArchiveFormat::TarBzip2),
        (".tbz", ArchiveFormat::TarBzip2),
    ];

    if let Some((_, format)) = compound.iter().find(|(suffix, _)| name.ends_with(suffix)) {
        return Some(*format);
    }

    let extension = Path::new(&name).extension()?.to_string_lossy();
    match extension.as_ref() {
        "zip" | "zipx" | "cbz" | "epub" => Some(ArchiveFormat::Zip),
        "7z" | "cb7" => Some(ArchiveFormat::SevenZip),
        "tar" | "cbt" => Some(ArchiveFormat::Tar),
        "gz" => Some(ArchiveFormat::Gzip),
        "zst" => Some(ArchiveFormat::Zstandard),
        "xz" | "lzma" => Some(ArchiveFormat::Xz),
        "bz" | "bz2" => Some(ArchiveFormat::Bzip2),
        "lz4" => Some(ArchiveFormat::Lz4),
        "br" => Some(ArchiveFormat::Brotli),
        "rar" | "cbr" => Some(ArchiveFormat::Rar),
        "cab" => Some(ArchiveFormat::Cab),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_compound_extensions_before_simple_extensions() {
        assert_eq!(
            detect_format_from_path("backup.TAR.GZ"),
            Some(ArchiveFormat::TarGzip)
        );
        assert_eq!(
            detect_format_from_path("backup.tar.zst"),
            Some(ArchiveFormat::TarZstd)
        );
    }

    #[test]
    fn detects_aliases_case_insensitively() {
        assert_eq!(
            detect_format_from_path("comic.CBZ"),
            Some(ArchiveFormat::Zip)
        );
        assert_eq!(
            detect_format_from_path("extended.ZIPX"),
            Some(ArchiveFormat::Zip)
        );
        assert_eq!(
            detect_format_from_path("comic.cb7"),
            Some(ArchiveFormat::SevenZip)
        );
        assert_eq!(
            detect_format_from_path("driver.CAB"),
            Some(ArchiveFormat::Cab)
        );
        for extension in OPEN_ARCHIVE_EXTENSIONS {
            assert!(
                detect_format_from_path(format!("sample.{extension}")).is_some(),
                "desktop extension is not recognized: {extension}"
            );
        }
    }

    #[test]
    fn unknown_or_missing_extensions_are_not_guessed() {
        assert_eq!(detect_format_from_path("archive"), None);
        assert_eq!(detect_format_from_path("archive.unknown"), None);
    }

    #[test]
    fn rar_is_read_only_beta() {
        let capabilities = ArchiveFormat::Rar.capabilities();
        assert!(capabilities.list);
        assert!(capabilities.extract);
        assert!(!capabilities.create);
        assert!(capabilities.encryption);
        assert_eq!(capabilities.stage, ReleaseStage::Beta);
    }

    #[test]
    fn cab_is_unencrypted_read_only_beta() {
        let capabilities = ArchiveFormat::Cab.capabilities();
        assert!(capabilities.list);
        assert!(capabilities.extract);
        assert!(!capabilities.create);
        assert!(!capabilities.encryption);
        assert_eq!(capabilities.stage, ReleaseStage::Beta);
    }

    #[test]
    fn creation_input_shape_distinguishes_archives_from_single_streams() {
        for format in [
            ArchiveFormat::Zip,
            ArchiveFormat::SevenZip,
            ArchiveFormat::Tar,
            ArchiveFormat::TarGzip,
            ArchiveFormat::TarZstd,
            ArchiveFormat::TarXz,
            ArchiveFormat::TarBzip2,
        ] {
            assert_eq!(
                format.create_input(),
                Some(CreateInputKind::FilesAndDirectories)
            );
        }
        for format in [
            ArchiveFormat::Gzip,
            ArchiveFormat::Zstandard,
            ArchiveFormat::Xz,
            ArchiveFormat::Bzip2,
            ArchiveFormat::Lz4,
            ArchiveFormat::Brotli,
        ] {
            assert_eq!(format.create_input(), Some(CreateInputKind::SingleFile));
        }
        assert_eq!(ArchiveFormat::Rar.create_input(), None);
        assert_eq!(ArchiveFormat::Cab.create_input(), None);
    }

    #[test]
    fn compression_level_contract_matches_each_encoder() {
        for format in [
            ArchiveFormat::Zip,
            ArchiveFormat::SevenZip,
            ArchiveFormat::TarGzip,
            ArchiveFormat::TarXz,
            ArchiveFormat::Gzip,
            ArchiveFormat::Xz,
        ] {
            assert_eq!(format.compression_level_range(), Some((0, 9)));
            assert_eq!(format.clamp_compression_level(22), 9);
        }
        for format in [ArchiveFormat::TarZstd, ArchiveFormat::Zstandard] {
            assert_eq!(format.compression_level_range(), Some((0, 22)));
            assert_eq!(format.clamp_compression_level(22), 22);
        }
        for format in [ArchiveFormat::TarBzip2, ArchiveFormat::Bzip2] {
            assert_eq!(format.compression_level_range(), Some((1, 9)));
            assert_eq!(format.clamp_compression_level(0), 1);
        }
        assert_eq!(
            ArchiveFormat::Brotli.compression_level_range(),
            Some((0, 11))
        );
        assert_eq!(ArchiveFormat::Brotli.clamp_compression_level(22), 11);
        for format in [ArchiveFormat::Tar, ArchiveFormat::Lz4] {
            assert_eq!(format.compression_level_range(), None);
        }
    }

    #[test]
    fn safety_limits_are_bounded_by_default() {
        let limits = SafetyLimits::default();
        assert!(limits.max_entries > 0);
        assert!(limits.max_expanded_bytes > 0);
        assert!(limits.max_expansion_ratio > 0);
        assert!(limits.max_path_depth > 0);
    }
}
