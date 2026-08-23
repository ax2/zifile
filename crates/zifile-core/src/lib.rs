//! Core domain model for ZiFile.
//!
//! This crate deliberately contains no UI or platform code. Archive backends
//! will implement the provider contract here so the desktop and CLI surfaces
//! share identical capability and safety behavior.

use std::fmt;
use std::path::Path;

use thiserror::Error;

/// Archive and compression formats in the ZiFile product roadmap.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ArchiveFormat {
    Zip,
    SevenZip,
    Tar,
    TarGzip,
    TarZstd,
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
    pub const ALL: [Self; 12] = [
        Self::Zip,
        Self::SevenZip,
        Self::Tar,
        Self::TarGzip,
        Self::TarZstd,
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
            Self::Rar => FormatCapabilities::read_only(false, ReleaseStage::PostV1),
            Self::SevenZip => FormatCapabilities::read_write(true, ReleaseStage::Beta),
            Self::Zip => FormatCapabilities::read_write(true, ReleaseStage::Alpha),
            Self::Tar | Self::TarGzip | Self::TarZstd => {
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

    const fn read_only(encryption: bool, stage: ReleaseStage) -> Self {
        Self {
            list: true,
            extract: true,
            create: false,
            encryption,
            stage,
        }
    }
}

/// Conservative defaults applied before any archive entry is written.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ZiFileError {
    #[error("the archive format could not be identified")]
    UnknownFormat,
    #[error("the requested operation is not supported for {0}")]
    UnsupportedOperation(ArchiveFormat),
}

/// Detect a format from a path without opening the file.
///
/// Signature-based detection will be added in Stage 1. This function exists
/// now so UI and CLI behavior can share one tested extension registry.
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
    fn rar_is_explicitly_read_only_and_post_v1() {
        let capabilities = ArchiveFormat::Rar.capabilities();
        assert!(capabilities.list);
        assert!(capabilities.extract);
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
