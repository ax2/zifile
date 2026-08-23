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
    ArchiveEntryInfo, ArchiveInfo, CancellationToken, ConflictPolicy, CreateOptions,
    ExtractOptions, OperationProgress, OperationSummary, ProgressSnapshot, create_archive,
    extract_archive, list_archive, test_archive,
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
}

impl ArchiveFormat {
    /// Stable display order used by both CLI and desktop UI.
    pub const ALL: [Self; 14] = [
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
    ];

    pub const fn capabilities(self) -> FormatCapabilities {
        match self {
            Self::Rar => FormatCapabilities::unsupported(ReleaseStage::PostV1),
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
        }
    }
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
    const fn read_write(encryption: bool, stage: ReleaseStage) -> Self {
        Self {
            list: true,
            extract: true,
            create: true,
            encryption,
            stage,
        }
    }

    const fn unsupported(stage: ReleaseStage) -> Self {
        Self {
            list: false,
            extract: false,
            create: false,
            encryption: false,
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
    let bytes = &header[..count];
    let extension_hint = detect_format_from_path(path);

    let detected = if bytes.starts_with(b"PK\x03\x04")
        || bytes.starts_with(b"PK\x05\x06")
        || bytes.starts_with(b"PK\x07\x08")
    {
        Some(ArchiveFormat::Zip)
    } else if bytes.starts_with(b"7z\xBC\xAF\x27\x1C") {
        Some(ArchiveFormat::SevenZip)
    } else if bytes.starts_with(b"Rar!\x1A\x07") {
        Some(ArchiveFormat::Rar)
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
    } else if bytes.len() >= 262 && &bytes[257..262] == b"ustar" {
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
        "zip" | "cbz" | "epub" => Some(ArchiveFormat::Zip),
        "7z" | "cb7" => Some(ArchiveFormat::SevenZip),
        "tar" | "cbt" => Some(ArchiveFormat::Tar),
        "gz" => Some(ArchiveFormat::Gzip),
        "zst" => Some(ArchiveFormat::Zstandard),
        "xz" | "lzma" => Some(ArchiveFormat::Xz),
        "bz" | "bz2" => Some(ArchiveFormat::Bzip2),
        "lz4" => Some(ArchiveFormat::Lz4),
        "br" => Some(ArchiveFormat::Brotli),
        "rar" | "cbr" => Some(ArchiveFormat::Rar),
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
            detect_format_from_path("comic.cb7"),
            Some(ArchiveFormat::SevenZip)
        );
    }

    #[test]
    fn unknown_or_missing_extensions_are_not_guessed() {
        assert_eq!(detect_format_from_path("archive"), None);
        assert_eq!(detect_format_from_path("archive.unknown"), None);
    }

    #[test]
    fn rar_is_explicitly_unavailable_and_post_v1() {
        let capabilities = ArchiveFormat::Rar.capabilities();
        assert!(!capabilities.list);
        assert!(!capabilities.extract);
        assert!(!capabilities.create);
        assert_eq!(capabilities.stage, ReleaseStage::PostV1);
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
