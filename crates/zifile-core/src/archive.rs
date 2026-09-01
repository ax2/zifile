use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bzip2::read::BzDecoder;
use bzip2::write::BzEncoder;
use cab::{Cabinet, CabinetBuilder, CompressionType as CabCompressionType};
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use lzma_rust2::{LzmaOptions, LzmaReader, LzmaWriter};
use rars::{
    Archive as RarArchive, ArchiveMemberDetail as RarMemberDetail,
    ArchiveReadOptions as RarReadOptions, ArchiveReader as RarReader,
    ArchiveVersion as RarArchiveVersion, Builder as RarBuilder, EntrySource as RarEntrySource,
    WriteOperation as RarWriteOperation, WriteProgress as RarWriteProgress,
    WriteProgressEvent as RarWriteProgressEvent, WriterResources as RarWriterResources,
};
use serde::{Deserialize, Serialize};
use sevenz_rust2::{ArchiveReader as SevenZReader, ArchiveWriter as SevenZWriter, Password};
use sha2::{Digest, Sha256};
use tempfile::NamedTempFile;
use walkdir::WalkDir;
use xz2::read::XzDecoder;
use xz2::write::XzEncoder;
use zip::read::ZipArchive;
use zip::write::SimpleFileOptions;
use zip::{AesMode, CompressionMethod, ZipWriter};

use crate::path_policy::safe_relative_path;
use crate::{ArchiveFormat, SafetyLimits, ZiFileError, ZiFileResult, detect_format};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveEntryInfo {
    pub path: PathBuf,
    pub size: u64,
    pub compressed_size: u64,
    pub is_directory: bool,
    pub encrypted: bool,
    /// Lowercase hexadecimal SHA-256 of the decoded file content. Listing
    /// leaves this empty; integrity testing populates it for regular files.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub checksum: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modified: Option<ArchiveTimestamp>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveTimestampOffset {
    Utc,
    Unspecified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveTimestampPrecision {
    TwoSeconds,
    Second,
    Subsecond,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ArchiveTimestamp {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
    pub nanosecond: u32,
    pub offset: ArchiveTimestampOffset,
    pub precision: ArchiveTimestampPrecision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveInfo {
    pub path: PathBuf,
    pub format: ArchiveFormat,
    pub entries: Vec<ArchiveEntryInfo>,
    pub total_size: u64,
    pub compressed_size: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum ConflictPolicy {
    #[default]
    Ask,
    Overwrite,
    Skip,
    Rename,
    Error,
}

#[derive(Debug, Clone, Default)]
pub struct ListOptions {
    pub limits: SafetyLimits,
    pub password: Option<String>,
    pub cancellation: CancellationToken,
    pub progress: OperationProgress,
}

#[derive(Debug, Clone, Default)]
pub struct TestOptions {
    pub limits: SafetyLimits,
    pub password: Option<String>,
    pub cancellation: CancellationToken,
    pub progress: OperationProgress,
}

#[derive(Debug, Clone)]
pub struct ExtractOptions {
    pub conflict: ConflictPolicy,
    pub limits: SafetyLimits,
    pub password: Option<String>,
    /// Optional exact archive-relative paths to extract. Parent directories
    /// are created as needed and do not need to be included.
    pub selected_paths: Option<HashSet<PathBuf>>,
    pub cancellation: CancellationToken,
    pub progress: OperationProgress,
}

impl Default for ExtractOptions {
    fn default() -> Self {
        Self {
            conflict: ConflictPolicy::Error,
            limits: SafetyLimits::default(),
            password: None,
            selected_paths: None,
            cancellation: CancellationToken::default(),
            progress: OperationProgress::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CreateOptions {
    pub compression_level: u8,
    pub password: Option<String>,
    pub cancellation: CancellationToken,
    pub progress: OperationProgress,
}

impl Default for CreateOptions {
    fn default() -> Self {
        Self {
            compression_level: 6,
            password: None,
            cancellation: CancellationToken::default(),
            progress: OperationProgress::default(),
        }
    }
}

/// Options for rebuilding an existing multi-entry archive after merging local
/// files or directories and/or removing archive-relative paths. The original
/// archive is only replaced after the complete staged rebuild succeeds.
#[derive(Debug, Clone)]
pub struct UpdateOptions {
    pub compression_level: u8,
    pub password: Option<String>,
    pub limits: SafetyLimits,
    /// Archive-relative files or directories to remove after extraction.
    /// Paths are normalized and checked with the same policy used for archive
    /// entries before anything is removed from the staging tree.
    pub remove_paths: Vec<PathBuf>,
    pub cancellation: CancellationToken,
    pub progress: OperationProgress,
}

/// One archive-relative rename applied while rebuilding a multi-entry archive.
///
/// Both paths are validated with the archive path policy before any staging
/// content is changed. Directory renames move the complete subtree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchiveRename {
    pub from: PathBuf,
    pub to: PathBuf,
}

impl Default for UpdateOptions {
    fn default() -> Self {
        Self {
            compression_level: 6,
            password: None,
            limits: SafetyLimits::default(),
            remove_paths: Vec::new(),
            cancellation: CancellationToken::default(),
            progress: OperationProgress::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationSummary {
    pub files: u64,
    pub directories: u64,
    pub bytes: u64,
    pub skipped: u64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgressSnapshot {
    pub processed_entries: u64,
    pub total_entries: u64,
    pub processed_bytes: u64,
    pub total_bytes: u64,
}

impl ProgressSnapshot {
    pub fn fraction(self) -> f32 {
        if self.total_bytes > 0 {
            (self.processed_bytes.min(self.total_bytes) as f64 / self.total_bytes as f64) as f32
        } else if self.total_entries > 0 {
            (self.processed_entries.min(self.total_entries) as f64 / self.total_entries as f64)
                as f32
        } else {
            0.0
        }
    }
}

#[derive(Debug, Default)]
struct ProgressState {
    processed_entries: AtomicU64,
    total_entries: AtomicU64,
    processed_bytes: AtomicU64,
    total_bytes: AtomicU64,
}

#[derive(Debug, Clone, Default)]
pub struct OperationProgress(Arc<ProgressState>);

impl OperationProgress {
    pub fn snapshot(&self) -> ProgressSnapshot {
        ProgressSnapshot {
            processed_entries: self.0.processed_entries.load(Ordering::Acquire),
            total_entries: self.0.total_entries.load(Ordering::Acquire),
            processed_bytes: self.0.processed_bytes.load(Ordering::Acquire),
            total_bytes: self.0.total_bytes.load(Ordering::Acquire),
        }
    }

    /// Replaces the observable snapshot for a trusted local worker client.
    pub fn update(&self, snapshot: ProgressSnapshot) {
        self.0
            .processed_entries
            .store(snapshot.processed_entries, Ordering::Release);
        self.0
            .total_entries
            .store(snapshot.total_entries, Ordering::Release);
        self.0
            .processed_bytes
            .store(snapshot.processed_bytes, Ordering::Release);
        self.0
            .total_bytes
            .store(snapshot.total_bytes, Ordering::Release);
    }

    fn set_totals(&self, entries: u64, bytes: u64) {
        self.0.total_entries.store(entries, Ordering::Release);
        self.0.total_bytes.store(bytes, Ordering::Release);
    }

    fn reset(&self) {
        self.0.processed_entries.store(0, Ordering::Release);
        self.0.total_entries.store(0, Ordering::Release);
        self.0.processed_bytes.store(0, Ordering::Release);
        self.0.total_bytes.store(0, Ordering::Release);
    }

    fn advance_entry(&self) {
        self.0.processed_entries.fetch_add(1, Ordering::AcqRel);
    }

    fn advance_bytes(&self, bytes: u64) {
        self.0.processed_bytes.fetch_add(bytes, Ordering::AcqRel);
    }
}

#[derive(Debug, Clone, Default)]
pub struct CancellationToken(Arc<AtomicBool>);

impl CancellationToken {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Release);
    }

    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::Acquire)
    }

    fn check(&self) -> ZiFileResult<()> {
        if self.is_cancelled() {
            Err(ZiFileError::Cancelled)
        } else {
            Ok(())
        }
    }
}

pub fn list_archive(path: impl AsRef<Path>, password: Option<&str>) -> ZiFileResult<ArchiveInfo> {
    list_archive_with_options(
        path,
        &ListOptions {
            password: password.map(str::to_owned),
            ..ListOptions::default()
        },
    )
}

pub fn list_archive_with_limits(
    path: impl AsRef<Path>,
    password: Option<&str>,
    limits: SafetyLimits,
) -> ZiFileResult<ArchiveInfo> {
    list_archive_with_options(
        path,
        &ListOptions {
            limits,
            password: password.map(str::to_owned),
            ..ListOptions::default()
        },
    )
}

pub fn list_archive_with_options(
    path: impl AsRef<Path>,
    options: &ListOptions,
) -> ZiFileResult<ArchiveInfo> {
    options.cancellation.check()?;
    let path = path.as_ref();
    let format = detect_format(path)?;
    let entries = match format {
        ArchiveFormat::Zip => list_zip(path, options)?,
        ArchiveFormat::SevenZip => {
            guard_archive_backend(format, "listing", || list_seven_zip(path, options))?
        }
        ArchiveFormat::Tar
        | ArchiveFormat::TarGzip
        | ArchiveFormat::TarZstd
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarLzma
        | ArchiveFormat::TarBzip2
        | ArchiveFormat::TarLz4 => list_tar(path, format, options)?,
        ArchiveFormat::Gzip
        | ArchiveFormat::Zstandard
        | ArchiveFormat::Xz
        | ArchiveFormat::Lzma
        | ArchiveFormat::Bzip2
        | ArchiveFormat::Lz4
        | ArchiveFormat::Brotli => list_stream(path, format, options)?,
        ArchiveFormat::Rar => guard_archive_backend(format, "listing", || list_rar(path, options))?,
        ArchiveFormat::Cab => guard_archive_backend(format, "listing", || list_cab(path, options))?,
    };
    options.cancellation.check()?;
    let scanned_bytes = options.progress.snapshot().processed_bytes;
    options
        .progress
        .set_totals(entries.len() as u64, scanned_bytes);

    let total_size = entries.iter().try_fold(0_u64, |total, entry| {
        total
            .checked_add(entry.size)
            .ok_or_else(|| ZiFileError::LimitExceeded("archive size overflow".to_owned()))
    })?;
    let compressed_size = fs::metadata(path)?.len();
    let info = ArchiveInfo {
        path: path.to_path_buf(),
        format,
        entries,
        total_size,
        compressed_size,
    };
    validate_declared_limits(&info, options.limits)?;
    Ok(info)
}

pub fn test_archive(path: impl AsRef<Path>, password: Option<&str>) -> ZiFileResult<ArchiveInfo> {
    test_archive_with_options(
        path,
        &TestOptions {
            password: password.map(str::to_owned),
            ..TestOptions::default()
        },
    )
}

pub fn test_archive_with_limits(
    path: impl AsRef<Path>,
    password: Option<&str>,
    limits: SafetyLimits,
) -> ZiFileResult<ArchiveInfo> {
    test_archive_with_options(
        path,
        &TestOptions {
            limits,
            password: password.map(str::to_owned),
            ..TestOptions::default()
        },
    )
}

pub fn test_archive_with_options(
    path: impl AsRef<Path>,
    options: &TestOptions,
) -> ZiFileResult<ArchiveInfo> {
    options.cancellation.check()?;
    let path = path.as_ref();
    let mut info = list_archive_with_options(
        path,
        &ListOptions {
            limits: options.limits,
            password: options.password.clone(),
            cancellation: options.cancellation.clone(),
            progress: options.progress.clone(),
        },
    )?;
    validate_declared_limits(&info, options.limits)?;
    let total_entries = info
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .count() as u64;
    options.progress.reset();
    options.progress.set_totals(total_entries, info.total_size);
    let mut checksums = match info.format {
        ArchiveFormat::Zip => test_zip(path, options)?,
        ArchiveFormat::SevenZip => {
            guard_archive_backend(info.format, "testing", || test_seven_zip(path, options))?
        }
        ArchiveFormat::Tar
        | ArchiveFormat::TarGzip
        | ArchiveFormat::TarZstd
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarLzma
        | ArchiveFormat::TarBzip2
        | ArchiveFormat::TarLz4 => test_tar(path, info.format, options)?,
        ArchiveFormat::Gzip
        | ArchiveFormat::Zstandard
        | ArchiveFormat::Xz
        | ArchiveFormat::Lzma
        | ArchiveFormat::Bzip2
        | ArchiveFormat::Lz4
        | ArchiveFormat::Brotli => test_stream(path, info.format, options)?,
        ArchiveFormat::Rar => {
            guard_archive_backend(info.format, "testing", || test_rar(path, options))?
        }
        ArchiveFormat::Cab => {
            guard_archive_backend(info.format, "testing", || test_cab(path, options))?
        }
    };
    for entry in &mut info.entries {
        if !entry.is_directory {
            entry.checksum = Some(checksums.remove(&entry.path).ok_or_else(|| {
                ZiFileError::Backend(format!(
                    "checksum was not produced for archive entry {}",
                    entry.path.display()
                ))
            })?);
        }
    }
    Ok(info)
}

pub fn extract_archive(
    archive: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: &ExtractOptions,
) -> ZiFileResult<OperationSummary> {
    options.cancellation.check()?;
    if options.conflict == ConflictPolicy::Ask {
        return Err(ZiFileError::ConflictPolicyRequired);
    }
    let archive = archive.as_ref();
    let destination = destination.as_ref();
    reject_symlink_components(destination)?;
    if destination.exists() && !destination.is_dir() {
        return Err(ZiFileError::DestinationExists(destination.to_path_buf()));
    }
    let info = list_archive_with_options(
        archive,
        &ListOptions {
            limits: options.limits,
            password: options.password.clone(),
            cancellation: options.cancellation.clone(),
            progress: options.progress.clone(),
        },
    )?;
    validate_declared_limits(&info, options.limits)?;
    let selected = info.entries.iter().filter(|entry| {
        !entry.is_directory
            && options
                .selected_paths
                .as_ref()
                .is_none_or(|paths| paths.contains(&entry.path))
    });
    let (entry_count, total_bytes) = selected.fold((0_u64, 0_u64), |(count, bytes), entry| {
        (count + 1, bytes.saturating_add(entry.size))
    });
    options.progress.reset();
    options.progress.set_totals(entry_count, total_bytes);
    options.cancellation.check()?;
    fs::create_dir_all(destination)?;
    reject_symlink_components(destination)?;

    match info.format {
        ArchiveFormat::Zip => extract_zip(archive, destination, options),
        ArchiveFormat::SevenZip => guard_archive_backend(info.format, "extracting", || {
            extract_seven_zip(archive, destination, options)
        }),
        ArchiveFormat::Tar
        | ArchiveFormat::TarGzip
        | ArchiveFormat::TarZstd
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarLzma
        | ArchiveFormat::TarBzip2
        | ArchiveFormat::TarLz4 => extract_tar(archive, destination, info.format, options),
        ArchiveFormat::Gzip
        | ArchiveFormat::Zstandard
        | ArchiveFormat::Xz
        | ArchiveFormat::Lzma
        | ArchiveFormat::Bzip2
        | ArchiveFormat::Lz4
        | ArchiveFormat::Brotli => extract_stream(archive, destination, info.format, options),
        ArchiveFormat::Rar => guard_archive_backend(info.format, "extracting", || {
            extract_rar(archive, destination, options, &info)
        }),
        ArchiveFormat::Cab => guard_archive_backend(info.format, "extracting", || {
            extract_cab(archive, destination, options)
        }),
    }
}

pub fn create_archive(
    sources: &[PathBuf],
    destination: impl AsRef<Path>,
    format: ArchiveFormat,
    options: &CreateOptions,
) -> ZiFileResult<OperationSummary> {
    create_archive_inner(sources, destination, format, options, false)
}

fn create_archive_inner(
    sources: &[PathBuf],
    destination: impl AsRef<Path>,
    format: ArchiveFormat,
    options: &CreateOptions,
    allow_empty: bool,
) -> ZiFileResult<OperationSummary> {
    options.cancellation.check()?;
    if sources.is_empty() && !allow_empty {
        return Err(ZiFileError::InvalidInput(
            "at least one source is required".to_owned(),
        ));
    }
    if !format.capabilities().create {
        return Err(ZiFileError::UnsupportedOperation(format));
    }
    let destination = destination.as_ref();
    let canonical_destination = canonicalize_with_missing(destination)?;
    for source in sources.iter().filter(|source| source.exists()) {
        let canonical_source = fs::canonicalize(source)?;
        if path_is_same_or_descendant(&canonical_destination, &canonical_source) {
            return Err(ZiFileError::InvalidInput(
                "destination cannot be inside a source directory".to_owned(),
            ));
        }
    }
    // Reject an existing output before traversing or encoding the sources.
    // The commit helpers repeat this check to cover a race where another
    // process creates the destination after this preflight.
    if destination.exists() {
        return Err(ZiFileError::DestinationExists(destination.to_path_buf()));
    }
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    match format {
        ArchiveFormat::Zip => create_zip(sources, destination, options),
        ArchiveFormat::SevenZip => create_seven_zip(sources, destination, options),
        ArchiveFormat::Tar
        | ArchiveFormat::TarGzip
        | ArchiveFormat::TarZstd
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarLzma
        | ArchiveFormat::TarBzip2
        | ArchiveFormat::TarLz4 => create_tar(sources, destination, format, options),
        ArchiveFormat::Gzip
        | ArchiveFormat::Zstandard
        | ArchiveFormat::Xz
        | ArchiveFormat::Lzma
        | ArchiveFormat::Bzip2
        | ArchiveFormat::Lz4
        | ArchiveFormat::Brotli => create_stream(sources, destination, format, options),
        ArchiveFormat::Rar => create_rar(sources, destination, options),
        ArchiveFormat::Cab => create_cab(sources, destination, options),
    }
}

/// Adds local files or directories to an existing multi-entry archive.
///
/// The archive is extracted into a private sibling staging directory, the
/// additions are merged by archive-relative root name, and a fresh archive is
/// created before the original is atomically replaced. A source file with an
/// existing archive-relative name is updated; directory/file type collisions
/// are rejected. RAR, CAB, and single-file stream formats remain read-only for
/// this operation; CAB can be created as a new fixed-layout archive but is not
/// updated in place.
pub fn update_archive(
    archive: impl AsRef<Path>,
    additions: &[PathBuf],
    options: &UpdateOptions,
) -> ZiFileResult<OperationSummary> {
    options.cancellation.check()?;
    if additions.is_empty() && options.remove_paths.is_empty() {
        return Err(ZiFileError::InvalidInput(
            "at least one addition or removal is required".to_owned(),
        ));
    }

    let archive = archive.as_ref();
    let format = detect_format(archive)?;
    if !format.supports_update() {
        return Err(ZiFileError::UnsupportedOperation(format));
    }
    let canonical_archive = fs::canonicalize(archive)?;
    for source in additions {
        let canonical_source = fs::canonicalize(source).map_err(|error| {
            ZiFileError::InvalidInput(format!(
                "addition does not exist or cannot be resolved: {} ({error})",
                source.display()
            ))
        })?;
        if path_is_same_or_descendant(&canonical_archive, &canonical_source) {
            return Err(ZiFileError::InvalidInput(
                "an archive cannot be added to itself or to one of its parent sources".to_owned(),
            ));
        }
    }

    let parent = archive.parent().unwrap_or_else(|| Path::new("."));
    let staging = tempfile::tempdir_in(parent)?;
    let contents = staging.path().join("contents");
    fs::create_dir(&contents)?;
    let remove_paths = normalized_update_removals(&options.remove_paths, options.limits)?;
    let extract_options = ExtractOptions {
        conflict: ConflictPolicy::Error,
        limits: options.limits,
        password: options.password.clone(),
        selected_paths: None,
        cancellation: options.cancellation.clone(),
        progress: options.progress.clone(),
    };
    extract_archive(archive, &contents, &extract_options)?;
    for path in &remove_paths {
        remove_update_path(&contents, path, options)?;
    }
    for source in additions {
        merge_update_source(source, &contents, options)?;
    }

    let sources = top_level_sources(&contents)?;
    let output = staging
        .path()
        .join(format!("updated.{}", format.canonical_extension()));
    let source_entries = collect_sources(&sources, &output)?;
    validate_update_entries(&source_entries, options.limits)?;
    options.cancellation.check()?;
    let create_options = CreateOptions {
        compression_level: format.clamp_compression_level(options.compression_level),
        password: options.password.clone(),
        cancellation: options.cancellation.clone(),
        progress: options.progress.clone(),
    };
    let summary = create_archive_inner(&sources, &output, format, &create_options, true)?;
    options.cancellation.check()?;
    replace_existing_file(&output, archive)?;
    Ok(summary)
}

/// Renames files or directories inside an existing multi-entry archive.
///
/// The archive is extracted into a private staging directory, all rename
/// mappings are validated, and the rebuilt archive replaces the original only
/// after encoding succeeds. ZIP, 7z, and TAR-family archives support this
/// operation; CAB creation is supported, but its fixed container layout is not
/// safely updated in place. Single-file streams and read-only providers do not.
pub fn rename_archive(
    archive: impl AsRef<Path>,
    renames: &[ArchiveRename],
    options: &UpdateOptions,
) -> ZiFileResult<OperationSummary> {
    options.cancellation.check()?;
    if renames.is_empty() {
        return Err(ZiFileError::InvalidInput(
            "at least one rename is required".to_owned(),
        ));
    }

    let archive = archive.as_ref();
    let format = detect_format(archive)?;
    if !format.supports_update() {
        return Err(ZiFileError::UnsupportedOperation(format));
    }

    let staging = tempfile::tempdir_in(archive.parent().unwrap_or_else(|| Path::new(".")))?;
    let contents = staging.path().join("contents");
    fs::create_dir(&contents)?;
    let extract_options = ExtractOptions {
        conflict: ConflictPolicy::Error,
        limits: options.limits,
        password: options.password.clone(),
        selected_paths: None,
        cancellation: options.cancellation.clone(),
        progress: options.progress.clone(),
    };
    extract_archive(archive, &contents, &extract_options)?;
    let renames = normalized_archive_renames(renames, options.limits)?;
    apply_archive_renames(&contents, &renames, options)?;

    let sources = top_level_sources(&contents)?;
    let output = staging
        .path()
        .join(format!("renamed.{}", format.canonical_extension()));
    let source_entries = collect_sources(&sources, &output)?;
    validate_update_entries(&source_entries, options.limits)?;
    options.cancellation.check()?;
    let create_options = CreateOptions {
        compression_level: format.clamp_compression_level(options.compression_level),
        password: options.password.clone(),
        cancellation: options.cancellation.clone(),
        progress: options.progress.clone(),
    };
    let summary = create_archive_inner(&sources, &output, format, &create_options, true)?;
    options.cancellation.check()?;
    replace_existing_file(&output, archive)?;
    Ok(summary)
}

fn normalized_archive_renames(
    renames: &[ArchiveRename],
    limits: SafetyLimits,
) -> ZiFileResult<Vec<ArchiveRename>> {
    let mut normalized = Vec::with_capacity(renames.len());
    let mut source_keys = HashSet::with_capacity(renames.len());
    let mut destination_keys = HashSet::with_capacity(renames.len());
    for rename in renames {
        let from = safe_relative_path(&rename.from.to_string_lossy(), limits.max_path_depth)?;
        let to = safe_relative_path(&rename.to.to_string_lossy(), limits.max_path_depth)?;
        if from == to {
            return Err(ZiFileError::InvalidInput(format!(
                "rename source and destination are identical: {}",
                from.display()
            )));
        }
        if !source_keys.insert(archive_collision_key(&from)) {
            return Err(ZiFileError::InvalidInput(format!(
                "rename source collides with another source: {}",
                from.display()
            )));
        }
        if !destination_keys.insert(archive_collision_key(&to)) {
            return Err(ZiFileError::InvalidInput(format!(
                "rename destination collides with another destination: {}",
                to.display()
            )));
        }
        if archive_collision_key(&from) == archive_collision_key(&to) {
            return Err(ZiFileError::InvalidInput(format!(
                "rename source and destination collide: {} -> {}",
                from.display(),
                to.display()
            )));
        }
        if normalized
            .iter()
            .any(|item: &ArchiveRename| item.from == from)
        {
            return Err(ZiFileError::InvalidInput(format!(
                "rename source is specified more than once: {}",
                from.display()
            )));
        }
        if normalized.iter().any(|item: &ArchiveRename| item.to == to) {
            return Err(ZiFileError::InvalidInput(format!(
                "rename destination is specified more than once: {}",
                to.display()
            )));
        }
        normalized.push(ArchiveRename { from, to });
    }

    for (index, current) in normalized.iter().enumerate() {
        for (other_index, other) in normalized.iter().enumerate() {
            if index == other_index {
                continue;
            }
            if current.from.starts_with(&other.from) || other.from.starts_with(&current.from) {
                return Err(ZiFileError::InvalidInput(
                    "rename sources cannot overlap".to_owned(),
                ));
            }
        }
        if normalized
            .iter()
            .any(|other| current.to != other.from && current.to.starts_with(&other.from))
        {
            return Err(ZiFileError::InvalidInput(format!(
                "rename destination is inside a renamed source: {}",
                current.to.display()
            )));
        }
        if normalized.iter().any(|other| {
            current.to != other.to
                && (current.to.starts_with(&other.to) || other.to.starts_with(&current.to))
        }) {
            return Err(ZiFileError::InvalidInput(
                "rename destinations cannot overlap".to_owned(),
            ));
        }
    }
    Ok(normalized)
}

fn apply_archive_renames(
    contents: &Path,
    renames: &[ArchiveRename],
    options: &UpdateOptions,
) -> ZiFileResult<()> {
    let mut source_kinds = HashMap::with_capacity(renames.len());
    for rename in renames {
        options.cancellation.check()?;
        let source = contents.join(&rename.from);
        reject_symlink_components(&source)?;
        let metadata = fs::symlink_metadata(&source).map_err(|error| {
            if error.kind() == io::ErrorKind::NotFound {
                ZiFileError::InvalidInput(format!(
                    "archive entry to rename was not found: {}",
                    rename.from.display()
                ))
            } else {
                ZiFileError::Io(error)
            }
        })?;
        if metadata_is_link_like(&metadata) {
            return Err(ZiFileError::LinkEntry(rename.from.display().to_string()));
        }
        if !metadata.is_dir() && !metadata.is_file() {
            return Err(ZiFileError::UnsupportedEntry(
                rename.from.display().to_string(),
            ));
        }
        source_kinds.insert(rename.from.clone(), metadata.is_dir());
    }

    for rename in renames {
        options.cancellation.check()?;
        let destination = contents.join(&rename.to);
        reject_symlink_components(&destination)?;
        if let Some(source_is_directory) = source_kinds.get(&rename.to) {
            let source_is_directory = *source_is_directory;
            let current_is_directory =
                source_kinds.get(&rename.from).copied().ok_or_else(|| {
                    ZiFileError::InvalidInput("rename source validation lost an entry".to_owned())
                })?;
            if source_is_directory != current_is_directory {
                return Err(ZiFileError::NameCollision(rename.to.clone()));
            }
        } else if fs::symlink_metadata(&destination).is_ok() {
            return Err(ZiFileError::NameCollision(rename.to.clone()));
        }
    }

    // Move every source out of the way first. This also makes swaps such as
    // a.txt -> b.txt and b.txt -> a.txt deterministic on Windows.
    let temporary = tempfile::tempdir_in(contents.parent().unwrap_or(contents))?;
    for (index, rename) in renames.iter().enumerate() {
        options.cancellation.check()?;
        fs::rename(
            contents.join(&rename.from),
            temporary.path().join(index.to_string()),
        )?;
    }
    for (index, rename) in renames.iter().enumerate() {
        options.cancellation.check()?;
        let destination = contents.join(&rename.to);
        if let Some(parent) = destination.parent() {
            reject_symlink_components(parent)?;
            fs::create_dir_all(parent)?;
        }
        fs::rename(temporary.path().join(index.to_string()), destination)?;
    }
    options.cancellation.check()
}

fn normalized_update_removals(
    paths: &[PathBuf],
    limits: SafetyLimits,
) -> ZiFileResult<Vec<PathBuf>> {
    let mut normalized = paths
        .iter()
        .map(|path| safe_relative_path(&path.to_string_lossy(), limits.max_path_depth))
        .collect::<ZiFileResult<Vec<_>>>()?;
    normalized.sort_by_key(|path| path.components().count());
    normalized.dedup();

    let mut roots = Vec::with_capacity(normalized.len());
    for path in normalized {
        if !roots.iter().any(|root: &PathBuf| path.starts_with(root)) {
            roots.push(path);
        }
    }
    Ok(roots)
}

fn remove_update_path(contents: &Path, path: &Path, options: &UpdateOptions) -> ZiFileResult<()> {
    options.cancellation.check()?;
    let target = contents.join(path);
    reject_symlink_components(&target)?;
    let metadata = fs::symlink_metadata(&target).map_err(|error| {
        if error.kind() == io::ErrorKind::NotFound {
            ZiFileError::InvalidInput(format!(
                "archive entry to remove was not found: {}",
                path.display()
            ))
        } else {
            ZiFileError::Io(error)
        }
    })?;
    if metadata_is_link_like(&metadata) {
        return Err(ZiFileError::LinkEntry(path.display().to_string()));
    }
    if metadata.is_dir() {
        fs::remove_dir_all(&target)?;
    } else if metadata.is_file() {
        fs::remove_file(&target)?;
    } else {
        return Err(ZiFileError::UnsupportedEntry(path.display().to_string()));
    }
    options.cancellation.check()
}

fn top_level_sources(contents: &Path) -> ZiFileResult<Vec<PathBuf>> {
    let mut sources = fs::read_dir(contents)?
        .map(|entry| entry.map(|entry| entry.path()).map_err(ZiFileError::Io))
        .collect::<ZiFileResult<Vec<_>>>()?;
    sources.sort();
    Ok(sources)
}

fn validate_update_entries(entries: &[SourceEntry], limits: SafetyLimits) -> ZiFileResult<()> {
    if entries.len() as u64 > limits.max_entries {
        return Err(ZiFileError::LimitExceeded(format!(
            "entry count exceeds {}",
            limits.max_entries
        )));
    }
    let total_size = entries.iter().try_fold(0_u64, |total, entry| {
        total
            .checked_add(entry.size)
            .ok_or_else(|| ZiFileError::LimitExceeded("archive size overflow".to_owned()))
    })?;
    if total_size > limits.max_expanded_bytes {
        return Err(ZiFileError::LimitExceeded(format!(
            "expanded data exceeds {} bytes",
            limits.max_expanded_bytes
        )));
    }
    Ok(())
}

fn merge_update_source(
    source: &Path,
    contents: &Path,
    options: &UpdateOptions,
) -> ZiFileResult<()> {
    let metadata = fs::symlink_metadata(source)?;
    if metadata_is_link_like(&metadata) {
        return Err(ZiFileError::LinkEntry(source.display().to_string()));
    }
    let root_name = source.file_name().ok_or_else(|| {
        ZiFileError::InvalidInput(format!("addition has no file name: {}", source.display()))
    })?;
    let root = safe_relative_path(&root_name.to_string_lossy(), options.limits.max_path_depth)?;
    for item in WalkDir::new(source).follow_links(false) {
        options.cancellation.check()?;
        let item = item.map_err(|error| ZiFileError::Backend(error.to_string()))?;
        let item_metadata = fs::symlink_metadata(item.path())?;
        if metadata_is_link_like(&item_metadata) {
            return Err(ZiFileError::LinkEntry(item.path().display().to_string()));
        }
        let relative = item.path().strip_prefix(source).map_err(|error| {
            ZiFileError::InvalidInput(format!("cannot relativize addition: {error}"))
        })?;
        let relative_archive_path = root.join(relative);
        let safe = safe_relative_path(
            &relative_archive_path.to_string_lossy(),
            options.limits.max_path_depth,
        )?;
        let target = contents.join(&safe);
        reject_symlink_components(&target)?;
        if item_metadata.is_dir() {
            if target.exists() && !target.is_dir() {
                return Err(ZiFileError::NameCollision(safe));
            }
            fs::create_dir_all(&target)?;
            set_modified_time_if_present(&target, item_metadata.modified().ok())?;
        } else if item_metadata.is_file() {
            if target.exists() && target.is_dir() {
                return Err(ZiFileError::NameCollision(safe));
            }
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent)?;
            }
            let mut input = BufReader::new(File::open(item.path())?);
            let mut output = BufWriter::new(File::create(&target)?);
            let mut copied = 0_u64;
            copy_limited(
                &mut input,
                &mut output,
                &mut copied,
                options.limits.max_expanded_bytes,
                Some(&options.cancellation),
                Some(&options.progress),
            )?;
            output.flush()?;
            output.get_ref().sync_all()?;
            set_modified_time_if_present(&target, item_metadata.modified().ok())?;
        } else {
            return Err(ZiFileError::UnsupportedEntry(
                item.path().display().to_string(),
            ));
        }
    }
    Ok(())
}

fn replace_existing_file(source: &Path, destination: &Path) -> ZiFileResult<()> {
    if destination.is_dir() {
        return Err(ZiFileError::DestinationExists(destination.to_path_buf()));
    }
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::{
            MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
        };

        let source: Vec<u16> = source
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let destination: Vec<u16> = destination
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        // SAFETY: both vectors are NUL-terminated UTF-16 paths that remain
        // alive for the duration of the synchronous Win32 call.
        let moved = unsafe {
            MoveFileExW(
                source.as_ptr(),
                destination.as_ptr(),
                MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
            )
        };
        if moved == 0 {
            return Err(ZiFileError::Io(io::Error::last_os_error()));
        }
    }
    #[cfg(not(windows))]
    {
        fs::rename(source, destination)?;
    }
    Ok(())
}

fn open_cab(path: &Path) -> ZiFileResult<Cabinet<BufReader<File>>> {
    const CABINET_CHAIN_FLAGS: u16 = 0x0003;
    const CABINET_FLAGS_OFFSET: usize = 30;

    let mut file = File::open(path)?;
    let mut header = [0_u8; CABINET_FLAGS_OFFSET + 2];
    file.read_exact(&mut header)?;
    let flags = u16::from_le_bytes([
        header[CABINET_FLAGS_OFFSET],
        header[CABINET_FLAGS_OFFSET + 1],
    ]);
    if flags & CABINET_CHAIN_FLAGS != 0 {
        return Err(ZiFileError::InvalidInput(
            "multi-cabinet sets are not supported".to_owned(),
        ));
    }
    file.seek(SeekFrom::Start(0))?;
    let cabinet = Cabinet::new(BufReader::new(file)).map_err(ZiFileError::Io)?;
    if cabinet.cabinet_set_index() != 0 {
        return Err(ZiFileError::InvalidInput(
            "multi-cabinet sets are not supported".to_owned(),
        ));
    }
    Ok(cabinet)
}

fn cab_file_names(
    cabinet: &Cabinet<BufReader<File>>,
    max_entries: u64,
    cancellation: &CancellationToken,
) -> ZiFileResult<Vec<String>> {
    let mut names = Vec::new();
    for folder in cabinet.folder_entries() {
        for entry in folder.file_entries() {
            cancellation.check()?;
            if names.len() as u64 >= max_entries {
                return Err(ZiFileError::LimitExceeded(format!(
                    "entry count exceeds {max_entries}"
                )));
            }
            names.push(entry.name().to_owned());
        }
    }
    Ok(names)
}

fn list_cab(path: &Path, options: &ListOptions) -> ZiFileResult<Vec<ArchiveEntryInfo>> {
    options.cancellation.check()?;
    let cabinet = open_cab(path)?;
    let mut entries = Vec::new();
    for folder in cabinet.folder_entries() {
        for entry in folder.file_entries() {
            options.cancellation.check()?;
            if entries.len() as u64 >= options.limits.max_entries {
                return Err(ZiFileError::LimitExceeded(format!(
                    "entry count exceeds {}",
                    options.limits.max_entries
                )));
            }
            let safe = safe_relative_path(entry.name(), options.limits.max_path_depth)?;
            entries.push(ArchiveEntryInfo {
                path: safe,
                size: u64::from(entry.uncompressed_size()),
                compressed_size: 0,
                is_directory: false,
                encrypted: false,
                checksum: None,
                modified: entry.datetime().map(|value| {
                    timestamp_from_primitive(
                        value,
                        ArchiveTimestampOffset::Unspecified,
                        ArchiveTimestampPrecision::TwoSeconds,
                    )
                }),
            });
            options.progress.advance_entry();
        }
    }
    validate_entry_names(&entries, options.limits)?;
    Ok(entries)
}

fn test_cab(path: &Path, options: &TestOptions) -> ZiFileResult<HashMap<PathBuf, String>> {
    let mut cabinet = open_cab(path)?;
    let names = cab_file_names(&cabinet, options.limits.max_entries, &options.cancellation)?;
    let mut total = 0_u64;
    let mut checksums = HashMap::new();
    for name in names {
        options.cancellation.check()?;
        let mut reader = cabinet.read_file(&name).map_err(ZiFileError::Io)?;
        let checksum = checksum_reader(
            &mut reader,
            &mut total,
            options.limits.max_expanded_bytes,
            Some(&options.cancellation),
            Some(&options.progress),
        )?;
        let relative = safe_relative_path(&name, options.limits.max_path_depth)?;
        checksums.insert(relative, checksum);
        options.progress.advance_entry();
    }
    Ok(checksums)
}

fn extract_cab(
    path: &Path,
    destination: &Path,
    options: &ExtractOptions,
) -> ZiFileResult<OperationSummary> {
    let mut cabinet = open_cab(path)?;
    let names = cab_file_names(&cabinet, options.limits.max_entries, &options.cancellation)?;
    let mut summary = OperationSummary::default();
    let mut claimed = HashSet::new();
    for name in names {
        options.cancellation.check()?;
        let modified = cabinet
            .get_file_entry(&name)
            .and_then(|entry| entry.datetime())
            .map(primitive_datetime_to_system_time);
        let relative = safe_relative_path(&name, options.limits.max_path_depth)?;
        if !is_selected(&relative, options) {
            continue;
        }
        let Some(output) = prepare_output(
            destination,
            &relative,
            false,
            options.conflict,
            &mut claimed,
        )?
        else {
            summary.skipped += 1;
            continue;
        };
        let mut reader = cabinet.read_file(&name).map_err(ZiFileError::Io)?;
        write_atomic(&output, |writer| {
            copy_limited(
                &mut reader,
                writer,
                &mut summary.bytes,
                options.limits.max_expanded_bytes,
                Some(&options.cancellation),
                Some(&options.progress),
            )?;
            Ok(())
        })?;
        set_modified_time_if_present(&output, modified)?;
        summary.files += 1;
        options.progress.advance_entry();
    }
    Ok(summary)
}

const RAR_BUFFERED_DECODE_LIMIT: u64 = 256 * 1024 * 1024;
const WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT: u64 = 0x400;

fn rar_read_options<'a>(password: Option<&'a str>, limits: SafetyLimits) -> RarReadOptions<'a> {
    RarReadOptions::with_optional_password(password.map(str::as_bytes))
        .with_rar50_buffered_decode_limit(limits.max_expanded_bytes.min(RAR_BUFFERED_DECODE_LIMIT))
}

fn read_rar(path: &Path, password: Option<&str>, limits: SafetyLimits) -> ZiFileResult<RarArchive> {
    RarReader::read_path_with_options(path, rar_read_options(password, limits))
        .map_err(map_rar_error)
}

fn map_rar_error(error: rars::Error) -> ZiFileError {
    if rar_error_needs_password(&error) {
        ZiFileError::PasswordRequired
    } else if rar_error_is_limit(&error) {
        ZiFileError::LimitExceeded(format!("RAR decoder resource limit: {error}"))
    } else {
        ZiFileError::Backend(format!("RAR backend: {error}"))
    }
}

fn rar_error_needs_password(error: &rars::Error) -> bool {
    match error {
        rars::Error::NeedPassword => true,
        rars::Error::AtArchiveOffset { source, .. }
        | rars::Error::AtEntry { source, .. }
        | rars::Error::InVolume { source, .. } => rar_error_needs_password(source),
        _ => false,
    }
}

fn rar_error_is_limit(error: &rars::Error) -> bool {
    match error {
        rars::Error::Rar50BufferedDecodeLimitExceeded { .. }
        | rars::Error::MemoryLimitExceeded { .. } => true,
        rars::Error::AtArchiveOffset { source, .. }
        | rars::Error::AtEntry { source, .. }
        | rars::Error::InVolume { source, .. } => rar_error_is_limit(source),
        _ => false,
    }
}

fn list_rar(path: &Path, options: &ListOptions) -> ZiFileResult<Vec<ArchiveEntryInfo>> {
    options.cancellation.check()?;
    let archive = read_rar(path, options.password.as_deref(), options.limits)?;
    options.cancellation.check()?;
    reject_rar_special_entries(&archive)?;
    let mut entries = Vec::new();
    for member in archive.members() {
        options.cancellation.check()?;
        if entries.len() as u64 >= options.limits.max_entries {
            return Err(ZiFileError::LimitExceeded(format!(
                "entry count exceeds {}",
                options.limits.max_entries
            )));
        }
        reject_rar_member_link(&member)?;
        let name = member.meta.name_lossy();
        let safe = safe_relative_path(&name, options.limits.max_path_depth)?;
        entries.push(ArchiveEntryInfo {
            path: safe,
            size: member.meta.unpacked_size,
            compressed_size: member.meta.packed_size,
            is_directory: member.meta.is_directory,
            encrypted: member.meta.is_encrypted,
            checksum: None,
            modified: member.meta.file_time.and_then(rar_archive_timestamp),
        });
        options.progress.advance_entry();
    }
    validate_entry_names(&entries, options.limits)?;
    Ok(entries)
}

fn reject_rar_special_entries(archive: &RarArchive) -> ZiFileResult<()> {
    if let RarArchive::Rar50Plus(archive) = archive {
        for block in &archive.blocks {
            if let rars::rar50::Block::File(file) = block
                && file.redirection.is_some()
            {
                return Err(ZiFileError::LinkEntry(
                    String::from_utf8_lossy(&file.name).into_owned(),
                ));
            }
        }
    }
    Ok(())
}

fn reject_rar_member_link(member: &rars::ArchiveMember) -> ZiFileResult<()> {
    let host = member.meta.host_os;
    let attributes = member.meta.file_attr;
    let (unix_link, windows_reparse) = match &member.detail {
        RarMemberDetail::Rar13 { .. } => (
            false,
            attributes & WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT != 0,
        ),
        RarMemberDetail::Rar15To40 { .. } => (
            matches!(host, Some(3 | 5)) && attributes & 0o170000 == 0o120000,
            matches!(host, Some(0 | 1 | 2 | 4))
                && attributes & WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT != 0,
        ),
        RarMemberDetail::Rar50Plus { .. } => (
            host == Some(1) && attributes & 0o170000 == 0o120000,
            host == Some(0) && attributes & WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT != 0,
        ),
        _ => (false, false),
    };
    if unix_link || windows_reparse {
        return Err(ZiFileError::LinkEntry(member.meta.name_lossy()));
    }
    Ok(())
}

fn test_rar(path: &Path, options: &TestOptions) -> ZiFileResult<HashMap<PathBuf, String>> {
    let archive = read_rar(path, options.password.as_deref(), options.limits)?;
    reject_rar_special_entries(&archive)?;
    for member in archive.members() {
        reject_rar_member_link(&member)?;
    }
    let total = Arc::new(AtomicU64::new(0));
    let limit_exceeded = Arc::new(AtomicBool::new(false));
    let checksums = Arc::new(Mutex::new(HashMap::new()));
    let result = archive.extract_to_with_options(
        rar_read_options(options.password.as_deref(), options.limits),
        |meta| {
            let relative = safe_relative_path(
                &String::from_utf8_lossy(&meta.name),
                options.limits.max_path_depth,
            )
            .map_err(|error| rars::Error::from(io::Error::other(error.to_string())))?;
            let writer = RarGuardedWriter::discard(
                Arc::clone(&total),
                options.limits.max_expanded_bytes,
                Arc::clone(&limit_exceeded),
                Some(options.cancellation.clone()),
                Some(options.progress.clone()),
            );
            if meta.is_directory {
                Ok(Box::new(writer) as Box<dyn Write>)
            } else {
                Ok(
                    Box::new(RarHashWriter::new(writer, relative, Arc::clone(&checksums)))
                        as Box<dyn Write>,
                )
            }
        },
    );
    options.cancellation.check()?;
    if limit_exceeded.load(Ordering::Acquire) {
        return Err(ZiFileError::LimitExceeded(format!(
            "expanded data exceeds {} bytes",
            options.limits.max_expanded_bytes
        )));
    }
    result.map_err(map_rar_error)?;
    for member in archive.members().filter(|member| !member.meta.is_directory) {
        let _ = member;
        options.progress.advance_entry();
    }
    let checksums = Arc::try_unwrap(checksums)
        .map_err(|_| ZiFileError::Backend("RAR checksum writers remained active".to_owned()))?
        .into_inner()
        .map_err(|_| ZiFileError::Backend("RAR checksum map lock poisoned".to_owned()))?;
    Ok(checksums)
}

struct PendingRarFile {
    temporary: Arc<Mutex<Option<NamedTempFile>>>,
    destination: PathBuf,
    modified: Option<SystemTime>,
}

struct RarGuardedWriter {
    temporary: Option<Arc<Mutex<Option<NamedTempFile>>>>,
    decoded_total: Arc<AtomicU64>,
    selected_total: Option<Arc<AtomicU64>>,
    maximum: u64,
    limit_exceeded: Arc<AtomicBool>,
    cancellation: Option<CancellationToken>,
    progress: Option<OperationProgress>,
}

impl RarGuardedWriter {
    fn discard(
        decoded_total: Arc<AtomicU64>,
        maximum: u64,
        limit_exceeded: Arc<AtomicBool>,
        cancellation: Option<CancellationToken>,
        progress: Option<OperationProgress>,
    ) -> Self {
        Self {
            temporary: None,
            decoded_total,
            selected_total: None,
            maximum,
            limit_exceeded,
            cancellation,
            progress,
        }
    }

    fn file(
        temporary: Arc<Mutex<Option<NamedTempFile>>>,
        decoded_total: Arc<AtomicU64>,
        selected_total: Arc<AtomicU64>,
        maximum: u64,
        limit_exceeded: Arc<AtomicBool>,
        cancellation: CancellationToken,
        progress: OperationProgress,
    ) -> Self {
        Self {
            temporary: Some(temporary),
            decoded_total,
            selected_total: Some(selected_total),
            maximum,
            limit_exceeded,
            cancellation: Some(cancellation),
            progress: Some(progress),
        }
    }

    fn should_write(&self, length: usize) -> bool {
        if self
            .cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled)
        {
            return false;
        }
        let length = length as u64;
        let current = self.decoded_total.load(Ordering::Acquire);
        if current
            .checked_add(length)
            .is_none_or(|next| next > self.maximum)
        {
            self.limit_exceeded.store(true, Ordering::Release);
            return false;
        }
        true
    }
}

impl Write for RarGuardedWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        // rars 0.9.3 can leave its solid-decoder worker waiting when a
        // destination writer returns an error. Consume but do not persist data
        // after cancellation/limit detection, then surface the recorded ZiFile
        // error before any temporary file is committed.
        if !self.should_write(buffer.len()) {
            return Ok(buffer.len());
        }
        let written = if let Some(temporary) = &self.temporary {
            let mut guard = temporary
                .lock()
                .map_err(|_| io::Error::other("RAR temporary writer lock poisoned"))?;
            guard
                .as_mut()
                .ok_or_else(|| io::Error::other("RAR temporary writer already committed"))?
                .write(buffer)?
        } else {
            buffer.len()
        };
        let written = written as u64;
        self.decoded_total.fetch_add(written, Ordering::AcqRel);
        if let Some(selected_total) = &self.selected_total {
            selected_total.fetch_add(written, Ordering::AcqRel);
        }
        if let Some(progress) = &self.progress {
            progress.advance_bytes(written);
        }
        Ok(written as usize)
    }

    fn flush(&mut self) -> io::Result<()> {
        if let Some(temporary) = &self.temporary {
            let mut guard = temporary
                .lock()
                .map_err(|_| io::Error::other("RAR temporary writer lock poisoned"))?;
            guard
                .as_mut()
                .ok_or_else(|| io::Error::other("RAR temporary writer already committed"))?
                .flush()
        } else {
            Ok(())
        }
    }
}

struct RarHashWriter {
    inner: RarGuardedWriter,
    path: PathBuf,
    hasher: Sha256,
    checksums: Arc<Mutex<HashMap<PathBuf, String>>>,
}

impl RarHashWriter {
    fn new(
        inner: RarGuardedWriter,
        path: PathBuf,
        checksums: Arc<Mutex<HashMap<PathBuf, String>>>,
    ) -> Self {
        Self {
            inner,
            path,
            hasher: Sha256::new(),
            checksums,
        }
    }
}

impl Write for RarHashWriter {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        let written = self.inner.write(buffer)?;
        if let Some(buffer) = buffer.get(..written) {
            self.hasher.update(buffer);
        }
        Ok(written)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}

impl Drop for RarHashWriter {
    fn drop(&mut self) {
        let digest = std::mem::replace(&mut self.hasher, Sha256::new()).finalize();
        if let Ok(mut checksums) = self.checksums.lock() {
            checksums.insert(self.path.clone(), hex_digest(&digest));
        }
    }
}

fn extract_rar(
    path: &Path,
    destination: &Path,
    options: &ExtractOptions,
    info: &ArchiveInfo,
) -> ZiFileResult<OperationSummary> {
    let archive = read_rar(path, options.password.as_deref(), options.limits)?;
    reject_rar_special_entries(&archive)?;
    for member in archive.members() {
        reject_rar_member_link(&member)?;
    }

    let mut claimed = HashSet::new();
    let mut outputs = HashMap::<String, Option<PathBuf>>::new();
    let mut directories = Vec::new();
    let mut summary = OperationSummary::default();
    for entry in &info.entries {
        if !is_selected(&entry.path, options) {
            continue;
        }
        let output = prepare_output(
            destination,
            &entry.path,
            entry.is_directory,
            options.conflict,
            &mut claimed,
        )?;
        if output.is_none() {
            summary.skipped += 1;
        } else if entry.is_directory
            && let Some(directory) = output.as_ref()
        {
            directories.push((archive_collision_key(&entry.path), directory.clone()));
        }
        outputs.insert(archive_collision_key(&entry.path), output);
    }

    let decoded_total = Arc::new(AtomicU64::new(0));
    let selected_total = Arc::new(AtomicU64::new(0));
    let limit_exceeded = Arc::new(AtomicBool::new(false));
    let mut pending = Vec::<PendingRarFile>::new();
    let mut directory_times = HashMap::<String, SystemTime>::new();
    let mut setup_error = None;
    let result = archive.extract_to_with_options(
        rar_read_options(options.password.as_deref(), options.limits),
        |meta| {
            let relative = match safe_relative_path(
                &String::from_utf8_lossy(&meta.name),
                options.limits.max_path_depth,
            ) {
                Ok(relative) => relative,
                Err(error) => {
                    setup_error = Some(error);
                    return Err(rars::Error::from(io::Error::other(
                        "RAR output path failed safety validation",
                    )));
                }
            };
            let key = archive_collision_key(&relative);
            let Some(Some(output)) = outputs.get(&key) else {
                return Ok(Box::new(RarGuardedWriter::discard(
                    Arc::clone(&decoded_total),
                    options.limits.max_expanded_bytes,
                    Arc::clone(&limit_exceeded),
                    Some(options.cancellation.clone()),
                    None,
                )) as Box<dyn Write>);
            };
            if meta.is_directory {
                if let Some(modified) = rar_modified_time(meta.file_time, meta.mtime_refinement) {
                    directory_times.insert(key, modified);
                }
                return Ok(Box::new(io::sink()) as Box<dyn Write>);
            }

            let parent = output.parent().unwrap_or_else(|| Path::new("."));
            let temporary =
                match fs::create_dir_all(parent).and_then(|()| NamedTempFile::new_in(parent)) {
                    Ok(temporary) => Arc::new(Mutex::new(Some(temporary))),
                    Err(error) => {
                        setup_error = Some(ZiFileError::Io(error));
                        return Err(rars::Error::from(io::Error::other(
                            "RAR temporary output creation failed",
                        )));
                    }
                };
            pending.push(PendingRarFile {
                temporary: Arc::clone(&temporary),
                destination: output.clone(),
                modified: rar_modified_time(meta.file_time, meta.mtime_refinement),
            });
            Ok(Box::new(RarGuardedWriter::file(
                temporary,
                Arc::clone(&decoded_total),
                Arc::clone(&selected_total),
                options.limits.max_expanded_bytes,
                Arc::clone(&limit_exceeded),
                options.cancellation.clone(),
                options.progress.clone(),
            )) as Box<dyn Write>)
        },
    );

    if let Some(error) = setup_error {
        return Err(error);
    }
    if options.cancellation.is_cancelled() {
        return Err(ZiFileError::Cancelled);
    }
    if limit_exceeded.load(Ordering::Acquire) {
        return Err(ZiFileError::LimitExceeded(format!(
            "expanded data exceeds {} bytes",
            options.limits.max_expanded_bytes
        )));
    }
    result.map_err(map_rar_error)?;

    for (_, directory) in &directories {
        fs::create_dir_all(directory)?;
    }
    summary.directories = directories.len() as u64;
    summary.files = pending.len() as u64;
    summary.bytes = selected_total.load(Ordering::Acquire);
    for pending_file in pending {
        let destination = pending_file.destination.clone();
        let modified = pending_file.modified;
        persist_rar_output(pending_file)?;
        set_modified_time_if_present(&destination, modified)?;
        options.progress.advance_entry();
    }
    let directory_times = directories
        .into_iter()
        .filter_map(|(key, path)| directory_times.remove(&key).map(|time| (path, time)))
        .collect();
    restore_directory_times(directory_times)?;
    Ok(summary)
}

fn persist_rar_output(pending: PendingRarFile) -> ZiFileResult<()> {
    let mutex = Arc::try_unwrap(pending.temporary)
        .map_err(|_| ZiFileError::Backend("RAR output writer remained active".to_owned()))?;
    let mut temporary = mutex
        .into_inner()
        .map_err(|_| ZiFileError::Backend("RAR output writer lock poisoned".to_owned()))?
        .ok_or_else(|| ZiFileError::Backend("RAR output was already committed".to_owned()))?;
    temporary.as_file_mut().sync_all()?;
    if pending.destination.is_dir() {
        return Err(ZiFileError::DestinationExists(pending.destination));
    }
    temporary
        .persist(&pending.destination)
        .map_err(|error| ZiFileError::Io(error.error))?;
    Ok(())
}

fn list_zip(path: &Path, options: &ListOptions) -> ZiFileResult<Vec<ArchiveEntryInfo>> {
    options.cancellation.check()?;
    let mut archive = ZipArchive::new(BufReader::new(File::open(path)?))?;
    if archive.len() as u64 > options.limits.max_entries {
        return Err(ZiFileError::LimitExceeded(format!(
            "entry count exceeds {}",
            options.limits.max_entries
        )));
    }
    options.progress.set_totals(archive.len() as u64, 0);
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
        options.cancellation.check()?;
        let entry = archive.by_index_raw(index)?;
        let safe = entry
            .enclosed_name()
            .ok_or_else(|| ZiFileError::UnsafePath(entry.name().to_owned()))?;
        reject_zip_link(entry.unix_mode(), entry.name())?;
        entries.push(ArchiveEntryInfo {
            path: safe,
            size: entry.size(),
            compressed_size: entry.compressed_size(),
            is_directory: entry.is_dir(),
            encrypted: entry.encrypted(),
            checksum: None,
            modified: entry.last_modified().map(timestamp_from_zip_datetime),
        });
        options.progress.advance_entry();
    }
    validate_entry_names(&entries, options.limits)?;
    Ok(entries)
}

fn test_zip(path: &Path, options: &TestOptions) -> ZiFileResult<HashMap<PathBuf, String>> {
    let mut archive = ZipArchive::new(BufReader::new(File::open(path)?))?;
    let mut total = 0;
    let mut checksums = HashMap::new();
    for index in 0..archive.len() {
        options.cancellation.check()?;
        let mut entry = open_zip_entry(&mut archive, index, options.password.as_deref())?;
        reject_zip_link(entry.unix_mode(), entry.name())?;
        if !entry.is_dir() {
            let checksum = checksum_reader(
                &mut entry,
                &mut total,
                options.limits.max_expanded_bytes,
                Some(&options.cancellation),
                Some(&options.progress),
            )?;
            let relative = entry
                .enclosed_name()
                .ok_or_else(|| ZiFileError::UnsafePath(entry.name().to_owned()))?;
            checksums.insert(relative, checksum);
            options.progress.advance_entry();
        }
    }
    Ok(checksums)
}

fn extract_zip(
    path: &Path,
    destination: &Path,
    options: &ExtractOptions,
) -> ZiFileResult<OperationSummary> {
    let mut archive = ZipArchive::new(BufReader::new(File::open(path)?))?;
    let mut summary = OperationSummary::default();
    let mut claimed = HashSet::new();
    let mut directory_times = Vec::new();
    for index in 0..archive.len() {
        options.cancellation.check()?;
        let mut entry = open_zip_entry(&mut archive, index, options.password.as_deref())?;
        let modified = entry.last_modified().and_then(zip_datetime_to_system_time);
        reject_zip_link(entry.unix_mode(), entry.name())?;
        let relative = safe_relative_path(entry.name(), options.limits.max_path_depth)?;
        if !is_selected(&relative, options) {
            continue;
        }
        let Some(output) = prepare_output(
            destination,
            &relative,
            entry.is_dir(),
            options.conflict,
            &mut claimed,
        )?
        else {
            summary.skipped += 1;
            continue;
        };
        if entry.is_dir() {
            fs::create_dir_all(&output)?;
            if let Some(modified) = modified {
                directory_times.push((output.clone(), modified));
            }
            summary.directories += 1;
        } else {
            write_atomic(&output, |writer| {
                copy_limited(
                    &mut entry,
                    writer,
                    &mut summary.bytes,
                    options.limits.max_expanded_bytes,
                    Some(&options.cancellation),
                    Some(&options.progress),
                )?;
                Ok(())
            })?;
            set_modified_time_if_present(&output, modified)?;
            summary.files += 1;
            options.progress.advance_entry();
        }
    }
    restore_directory_times(directory_times)?;
    Ok(summary)
}

fn open_zip_entry<'a, R: Read + Seek>(
    archive: &'a mut ZipArchive<R>,
    index: usize,
    password: Option<&str>,
) -> ZiFileResult<zip::read::ZipFile<'a, R>> {
    let encrypted = archive.by_index_raw(index)?.encrypted();
    if encrypted {
        let password = password.ok_or(ZiFileError::PasswordRequired)?;
        Ok(archive.by_index_decrypt(index, password.as_bytes())?)
    } else {
        Ok(archive.by_index(index)?)
    }
}

fn reject_zip_link(mode: Option<u32>, name: &str) -> ZiFileResult<()> {
    if mode.is_some_and(|mode| mode & 0o170000 == 0o120000) {
        return Err(ZiFileError::LinkEntry(name.to_owned()));
    }
    Ok(())
}

fn create_zip(
    sources: &[PathBuf],
    destination: &Path,
    options: &CreateOptions,
) -> ZiFileResult<OperationSummary> {
    let entries = collect_sources(sources, destination)?;
    set_source_totals(&entries, &options.progress);
    let mut temporary = temporary_archive(destination)?;
    let mut writer = ZipWriter::new(temporary.as_file_mut());
    let mut summary = OperationSummary::default();
    for source in entries {
        options.cancellation.check()?;
        let name = archive_name(&source.archive_path);
        let base = SimpleFileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(i64::from(options.compression_level.min(9))))
            .large_file(source.size >= u64::from(u32::MAX));
        let base = source
            .modified
            .and_then(system_time_to_zip_datetime)
            .map_or(base, |modified| base.last_modified_time(modified));
        if source.is_directory {
            writer.add_directory(format!("{name}/"), base)?;
            summary.directories += 1;
        } else {
            if let Some(password) = options
                .password
                .as_deref()
                .filter(|value| !value.is_empty())
            {
                writer.start_file(name, base.with_aes_encryption(AesMode::Aes256, password))?;
            } else {
                writer.start_file(name, base)?;
            }
            let mut input = BufReader::new(File::open(&source.disk_path)?);
            summary.bytes += copy_cancellable(
                &mut input,
                &mut writer,
                &options.cancellation,
                &options.progress,
            )?;
            summary.files += 1;
            options.progress.advance_entry();
        }
    }
    writer.finish()?;
    persist_archive(temporary, destination)?;
    Ok(summary)
}

fn list_tar(
    path: &Path,
    format: ArchiveFormat,
    options: &ListOptions,
) -> ZiFileResult<Vec<ArchiveEntryInfo>> {
    options.cancellation.check()?;
    let mut archive = tar::Archive::new(open_tar_reader(path, format, options.limits)?);
    let compressed_size = fs::metadata(path)?.len();
    let maximum_expanded = options
        .limits
        .max_expanded_bytes
        .min(compressed_size.saturating_mul(options.limits.max_expansion_ratio));
    let mut total_size = 0_u64;
    let mut result = Vec::new();
    for entry in archive.entries()? {
        options.cancellation.check()?;
        if result.len() as u64 >= options.limits.max_entries {
            return Err(ZiFileError::LimitExceeded(format!(
                "entry count exceeds {}",
                options.limits.max_entries
            )));
        }
        let entry = entry?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(ZiFileError::LinkEntry(
                entry.path()?.to_string_lossy().into_owned(),
            ));
        }
        total_size = total_size
            .checked_add(entry.size())
            .ok_or_else(|| ZiFileError::LimitExceeded("archive size overflow".to_owned()))?;
        if total_size > maximum_expanded {
            return Err(ZiFileError::LimitExceeded(format!(
                "expanded data exceeds {maximum_expanded} bytes"
            )));
        }
        let path = entry.path()?.into_owned();
        let safe = safe_relative_path(&path.to_string_lossy(), options.limits.max_path_depth)?;
        result.push(ArchiveEntryInfo {
            path: safe,
            size: entry.size(),
            compressed_size: 0,
            is_directory: entry_type.is_dir(),
            encrypted: false,
            checksum: None,
            modified: entry
                .header()
                .mtime()
                .ok()
                .and_then(timestamp_from_unix_seconds),
        });
        options.progress.advance_entry();
    }
    validate_entry_names(&result, options.limits)?;
    Ok(result)
}

fn test_tar(
    path: &Path,
    format: ArchiveFormat,
    options: &TestOptions,
) -> ZiFileResult<HashMap<PathBuf, String>> {
    let mut archive = tar::Archive::new(open_tar_reader(path, format, options.limits)?);
    let mut total = 0;
    let mut checksums = HashMap::new();
    for entry in archive.entries()? {
        options.cancellation.check()?;
        let mut entry = entry?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(ZiFileError::LinkEntry(
                entry.path()?.to_string_lossy().into_owned(),
            ));
        }
        if entry_type.is_file() {
            let checksum = checksum_reader(
                &mut entry,
                &mut total,
                options.limits.max_expanded_bytes,
                Some(&options.cancellation),
                Some(&options.progress),
            )?;
            let path = entry.path()?.into_owned();
            let relative =
                safe_relative_path(&path.to_string_lossy(), options.limits.max_path_depth)?;
            checksums.insert(relative, checksum);
            options.progress.advance_entry();
        }
    }
    Ok(checksums)
}

fn extract_tar(
    path: &Path,
    destination: &Path,
    format: ArchiveFormat,
    options: &ExtractOptions,
) -> ZiFileResult<OperationSummary> {
    let mut archive = tar::Archive::new(open_tar_reader(path, format, options.limits)?);
    let mut summary = OperationSummary::default();
    let mut claimed = HashSet::new();
    let mut directory_times = Vec::new();
    for entry in archive.entries()? {
        options.cancellation.check()?;
        let mut entry = entry?;
        let entry_type = entry.header().entry_type();
        let modified = entry
            .header()
            .mtime()
            .ok()
            .and_then(|seconds| UNIX_EPOCH.checked_add(Duration::from_secs(seconds)));
        let original = entry.path()?.to_string_lossy().into_owned();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(ZiFileError::LinkEntry(original));
        }
        let relative = safe_relative_path(&original, options.limits.max_path_depth)?;
        if !is_selected(&relative, options) {
            continue;
        }
        let Some(output) = prepare_output(
            destination,
            &relative,
            entry_type.is_dir(),
            options.conflict,
            &mut claimed,
        )?
        else {
            summary.skipped += 1;
            continue;
        };
        if entry_type.is_dir() {
            fs::create_dir_all(&output)?;
            if let Some(modified) = modified {
                directory_times.push((output.clone(), modified));
            }
            summary.directories += 1;
        } else if entry_type.is_file() {
            write_atomic(&output, |writer| {
                copy_limited(
                    &mut entry,
                    writer,
                    &mut summary.bytes,
                    options.limits.max_expanded_bytes,
                    Some(&options.cancellation),
                    Some(&options.progress),
                )?;
                Ok(())
            })?;
            set_modified_time_if_present(&output, modified)?;
            summary.files += 1;
            options.progress.advance_entry();
        } else {
            return Err(ZiFileError::UnsupportedEntry(original));
        }
    }
    restore_directory_times(directory_times)?;
    Ok(summary)
}

fn create_tar(
    sources: &[PathBuf],
    destination: &Path,
    format: ArchiveFormat,
    options: &CreateOptions,
) -> ZiFileResult<OperationSummary> {
    if options
        .password
        .as_deref()
        .is_some_and(|value| !value.is_empty())
    {
        return Err(ZiFileError::UnsupportedEncryption(format));
    }
    let entries = collect_sources(sources, destination)?;
    set_source_totals(&entries, &options.progress);
    let mut temporary = temporary_archive(destination)?;
    let summary = match format {
        ArchiveFormat::Tar => write_tar(
            &entries,
            BufWriter::new(temporary.as_file_mut()),
            &options.cancellation,
            &options.progress,
        )?,
        ArchiveFormat::TarGzip => write_tar(
            &entries,
            GzEncoder::new(
                BufWriter::new(temporary.as_file_mut()),
                Compression::new(u32::from(options.compression_level.min(9))),
            ),
            &options.cancellation,
            &options.progress,
        )?,
        ArchiveFormat::TarZstd => write_tar(
            &entries,
            zstd::stream::write::Encoder::new(
                BufWriter::new(temporary.as_file_mut()),
                i32::from(options.compression_level.min(22)),
            )?
            .auto_finish(),
            &options.cancellation,
            &options.progress,
        )?,
        ArchiveFormat::TarXz => write_tar(
            &entries,
            XzEncoder::new(
                BufWriter::new(temporary.as_file_mut()),
                u32::from(options.compression_level.min(9)),
            ),
            &options.cancellation,
            &options.progress,
        )?,
        ArchiveFormat::TarLzma => write_tar_lzma(
            &entries,
            BufWriter::new(temporary.as_file_mut()),
            options.compression_level,
            &options.cancellation,
            &options.progress,
        )?,
        ArchiveFormat::TarBzip2 => write_tar(
            &entries,
            BzEncoder::new(
                BufWriter::new(temporary.as_file_mut()),
                bzip2::Compression::new(u32::from(options.compression_level.clamp(1, 9))),
            ),
            &options.cancellation,
            &options.progress,
        )?,
        ArchiveFormat::TarLz4 => write_tar_lz4(
            &entries,
            BufWriter::new(temporary.as_file_mut()),
            &options.cancellation,
            &options.progress,
        )?,
        _ => return Err(ZiFileError::UnsupportedOperation(format)),
    };
    persist_archive(temporary, destination)?;
    Ok(summary)
}

struct RarCreateProgress {
    cancellation: CancellationToken,
    progress: OperationProgress,
    compression_bytes: Mutex<u64>,
}

impl RarCreateProgress {
    fn new(cancellation: CancellationToken, progress: OperationProgress) -> Self {
        Self {
            cancellation,
            progress,
            compression_bytes: Mutex::new(0),
        }
    }
}

impl RarWriteProgress for RarCreateProgress {
    fn report(&self, event: RarWriteProgressEvent<'_>) {
        match event {
            RarWriteProgressEvent::OperationStarted {
                operation: RarWriteOperation::Compression,
                ..
            } => {
                if let Ok(mut completed) = self.compression_bytes.lock() {
                    *completed = 0;
                }
            }
            RarWriteProgressEvent::Advanced {
                operation: RarWriteOperation::Compression,
                completed_bytes,
                ..
            } => {
                if let Ok(mut previous) = self.compression_bytes.lock() {
                    let delta = completed_bytes.saturating_sub(*previous);
                    *previous = (*previous).max(completed_bytes);
                    self.progress.advance_bytes(delta);
                }
            }
            RarWriteProgressEvent::EntryFinished {
                operation: RarWriteOperation::Compression,
                ..
            } => self.progress.advance_entry(),
            _ => {}
        }
    }

    fn is_cancelled(&self) -> bool {
        self.cancellation.is_cancelled()
    }
}

fn create_rar(
    sources: &[PathBuf],
    destination: &Path,
    options: &CreateOptions,
) -> ZiFileResult<OperationSummary> {
    let entries = collect_sources(sources, destination)?;
    let files: Vec<&SourceEntry> = entries.iter().filter(|entry| !entry.is_directory).collect();
    if files.is_empty() {
        return Err(ZiFileError::InvalidInput(
            "RAR requires at least one file; empty directories cannot be represented".to_owned(),
        ));
    }
    for directory in entries.iter().filter(|entry| entry.is_directory) {
        if !files.iter().any(|file| {
            file.archive_path.starts_with(&directory.archive_path)
                && file.archive_path != directory.archive_path
        }) {
            return Err(ZiFileError::InvalidInput(format!(
                "RAR cannot preserve empty directory: {}",
                archive_name(&directory.archive_path)
            )));
        }
    }

    set_source_totals(&entries, &options.progress);
    let password = options
        .password
        .as_deref()
        .filter(|value| !value.is_empty())
        .map(|value| value.as_bytes().to_vec());
    let encrypt_headers = password.is_some();
    let mut builder = RarBuilder::new(RarArchiveVersion::Rar50)
        .compression_level(Some(options.compression_level.min(5)))
        .password(password)
        .header_encryption(encrypt_headers);
    for source in &files {
        options.cancellation.check()?;
        builder
            .add_source(
                source
                    .archive_path
                    .to_string_lossy()
                    .replace('\\', "/")
                    .into_bytes(),
                RarEntrySource::from_path(&source.disk_path),
                source.modified.and_then(rar_dos_time),
                None,
            )
            .map_err(map_rar_error)?;
    }

    let mut temporary = temporary_archive(destination)?;
    let resources = RarWriterResources::default().with_temp_dir(
        destination
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or_else(|| Path::new(".")),
    );
    let writer_progress =
        RarCreateProgress::new(options.cancellation.clone(), options.progress.clone());
    builder
        .write_to(temporary.as_file_mut(), &resources, Some(&writer_progress))
        .map_err(map_rar_error)?;
    options.cancellation.check()?;
    persist_archive(temporary, destination)?;

    let bytes = files.iter().map(|source| source.size).sum();
    options.progress.update(ProgressSnapshot {
        processed_entries: files.len() as u64,
        total_entries: files.len() as u64,
        processed_bytes: bytes,
        total_bytes: bytes,
    });
    Ok(OperationSummary {
        files: files.len() as u64,
        directories: entries.iter().filter(|entry| entry.is_directory).count() as u64,
        bytes,
        ..OperationSummary::default()
    })
}

fn create_cab(
    sources: &[PathBuf],
    destination: &Path,
    options: &CreateOptions,
) -> ZiFileResult<OperationSummary> {
    if options
        .password
        .as_deref()
        .is_some_and(|value| !value.is_empty())
    {
        return Err(ZiFileError::UnsupportedEncryption(ArchiveFormat::Cab));
    }

    let entries = collect_sources(sources, destination)?;
    let files: Vec<&SourceEntry> = entries.iter().filter(|entry| !entry.is_directory).collect();
    if files.is_empty() {
        return Err(ZiFileError::InvalidInput(
            "CAB requires at least one file; empty directories cannot be represented".to_owned(),
        ));
    }
    if files.len() > usize::from(u16::MAX) {
        return Err(ZiFileError::LimitExceeded(
            "CAB supports at most 65535 files per cabinet".to_owned(),
        ));
    }

    for source in &files {
        let name = archive_name(&source.archive_path);
        if name.len() > 255 {
            return Err(ZiFileError::LimitExceeded(format!(
                "CAB file name exceeds 255 bytes: {name}"
            )));
        }
        if source.size > 0x7fff_8000 {
            return Err(ZiFileError::LimitExceeded(format!(
                "CAB file exceeds the 2147450880-byte per-file limit: {name}"
            )));
        }
    }

    for directory in entries.iter().filter(|entry| entry.is_directory) {
        if !files.iter().any(|file| {
            file.archive_path.starts_with(&directory.archive_path)
                && file.archive_path != directory.archive_path
        }) {
            return Err(ZiFileError::InvalidInput(format!(
                "CAB cannot preserve empty directory: {}",
                archive_name(&directory.archive_path)
            )));
        }
    }

    set_source_totals(&entries, &options.progress);

    // CAB stores files in folders. Split before the folder's 32-bit uncompressed
    // offset would overflow; the backend then compresses each folder with MSZIP.
    let mut groups: Vec<Vec<&SourceEntry>> = Vec::new();
    let mut group = Vec::new();
    let mut group_size = 0_u64;
    for source in files.iter().copied() {
        if !group.is_empty() && group_size.saturating_add(source.size) > u64::from(u32::MAX) {
            groups.push(group);
            group = Vec::new();
            group_size = 0;
        }
        group_size = group_size.saturating_add(source.size);
        group.push(source);
    }
    if !group.is_empty() {
        groups.push(group);
    }

    let mut builder = CabinetBuilder::new();
    for group in &groups {
        let folder = builder.add_folder(CabCompressionType::MsZip);
        for source in group {
            let name = archive_name(&source.archive_path);
            let file = folder.add_file(name);
            if let Some(modified) = source.modified {
                let value = time::OffsetDateTime::from(modified);
                file.set_datetime(time::PrimitiveDateTime::new(value.date(), value.time()));
            }
        }
    }

    let mut temporary = temporary_archive(destination)?;
    {
        let mut writer = builder.build(temporary.as_file_mut())?;
        let mut index = 0_usize;
        while let Some(mut output) = writer.next_file()? {
            let source = files.get(index).ok_or_else(|| {
                ZiFileError::Backend("CAB writer returned more files than planned".to_owned())
            })?;
            let expected_name = archive_name(&source.archive_path);
            if output.file_name() != expected_name {
                return Err(ZiFileError::Backend(format!(
                    "CAB writer file order changed: expected {expected_name}, got {}",
                    output.file_name()
                )));
            }
            let mut input = BufReader::new(File::open(&source.disk_path)?);
            copy_cancellable(
                &mut input,
                &mut output,
                &options.cancellation,
                &options.progress,
            )?;
            index += 1;
        }
        if index != files.len() {
            return Err(ZiFileError::Backend(format!(
                "CAB writer accepted {index} files but {} were planned",
                files.len()
            )));
        }
        writer.finish()?;
    }
    persist_archive(temporary, destination)?;

    Ok(OperationSummary {
        files: files.len() as u64,
        directories: entries.iter().filter(|entry| entry.is_directory).count() as u64,
        bytes: files.iter().map(|entry| entry.size).sum(),
        ..OperationSummary::default()
    })
}

fn write_tar<W: Write>(
    entries: &[SourceEntry],
    output: W,
    cancellation: &CancellationToken,
    progress: &OperationProgress,
) -> ZiFileResult<OperationSummary> {
    let mut archive = tar::Builder::new(output);
    let summary = append_tar_entries(&mut archive, entries, cancellation, progress)?;
    archive.finish()?;
    Ok(summary)
}

fn append_tar_entries<W: Write>(
    archive: &mut tar::Builder<W>,
    entries: &[SourceEntry],
    cancellation: &CancellationToken,
    progress: &OperationProgress,
) -> ZiFileResult<OperationSummary> {
    let mut summary = OperationSummary::default();
    for source in entries {
        cancellation.check()?;
        let name = &source.archive_path;
        if source.is_directory {
            archive.append_dir(name, &source.disk_path)?;
            summary.directories += 1;
        } else {
            archive.append_path_with_name(&source.disk_path, name)?;
            summary.files += 1;
            summary.bytes += source.size;
            progress.advance_bytes(source.size);
            progress.advance_entry();
        }
    }
    Ok(summary)
}

fn write_tar_lzma(
    entries: &[SourceEntry],
    output: BufWriter<&mut File>,
    compression_level: u8,
    cancellation: &CancellationToken,
    progress: &OperationProgress,
) -> ZiFileResult<OperationSummary> {
    let options = LzmaOptions::with_preset(u32::from(compression_level.min(9)));
    let lzma = LzmaWriter::new_use_header(output, &options, None)
        .map_err(|error| ZiFileError::Backend(error.to_string()))?;
    let mut archive = tar::Builder::new(lzma);
    let summary = append_tar_entries(&mut archive, entries, cancellation, progress)?;
    archive.finish()?;
    archive
        .into_inner()
        .map_err(|error| ZiFileError::Backend(error.to_string()))?
        .finish()
        .map_err(|error| ZiFileError::Backend(error.to_string()))?;
    Ok(summary)
}

fn write_tar_lz4(
    entries: &[SourceEntry],
    output: BufWriter<&mut File>,
    cancellation: &CancellationToken,
    progress: &OperationProgress,
) -> ZiFileResult<OperationSummary> {
    let lz4 = FrameEncoder::new(output);
    let mut archive = tar::Builder::new(lz4);
    let summary = append_tar_entries(&mut archive, entries, cancellation, progress)?;
    archive.finish()?;
    archive
        .into_inner()
        .map_err(|error| ZiFileError::Backend(error.to_string()))?
        .finish()
        .map_err(|error| ZiFileError::Backend(error.to_string()))?;
    Ok(summary)
}

fn open_tar_reader(
    path: &Path,
    format: ArchiveFormat,
    limits: SafetyLimits,
) -> ZiFileResult<Box<dyn Read>> {
    let file = BufReader::new(File::open(path)?);
    match format {
        ArchiveFormat::Tar => Ok(Box::new(file)),
        ArchiveFormat::TarGzip => Ok(Box::new(GzDecoder::new(file))),
        ArchiveFormat::TarZstd => Ok(Box::new(zstd::stream::read::Decoder::new(file)?)),
        ArchiveFormat::TarXz => Ok(Box::new(XzDecoder::new(file))),
        ArchiveFormat::TarLzma => {
            LzmaReader::new_mem_limit(file, lzma_memory_limit_kib(limits), None)
                .map(|reader| Box::new(reader) as Box<dyn Read>)
                .map_err(|error| ZiFileError::Backend(error.to_string()))
        }
        ArchiveFormat::TarBzip2 => Ok(Box::new(BzDecoder::new(file))),
        ArchiveFormat::TarLz4 => Ok(Box::new(FrameDecoder::new(file))),
        _ => Err(ZiFileError::UnsupportedOperation(format)),
    }
}

fn guard_archive_backend<T>(
    format: ArchiveFormat,
    operation: &str,
    action: impl FnOnce() -> ZiFileResult<T>,
) -> ZiFileResult<T> {
    // Third-party decoders may panic on impossible sizes in malformed metadata.
    // Keep that failure inside the provider boundary; process-aborting failures
    // such as OOM and sanitizer findings are intentionally not intercepted.
    catch_unwind(AssertUnwindSafe(action)).unwrap_or_else(|_| {
        Err(ZiFileError::Backend(format!(
            "{format} backend rejected malformed metadata while {operation} the archive"
        )))
    })
}

fn list_seven_zip(path: &Path, options: &ListOptions) -> ZiFileResult<Vec<ArchiveEntryInfo>> {
    options.cancellation.check()?;
    let reader = SevenZReader::open(path, seven_password(options.password.as_deref()))?;
    options.cancellation.check()?;
    if reader.archive().files.len() as u64 > options.limits.max_entries {
        return Err(ZiFileError::LimitExceeded(format!(
            "entry count exceeds {}",
            options.limits.max_entries
        )));
    }
    let files = reader.archive().files.clone();
    options.progress.set_totals(files.len() as u64, 0);
    let mut entries = Vec::with_capacity(files.len());
    for entry in &files {
        options.cancellation.check()?;
        let mut methods = Vec::new();
        reader.file_compression_methods(entry.name(), &mut methods)?;
        entries.push(ArchiveEntryInfo {
            path: safe_relative_path(entry.name(), options.limits.max_path_depth)?,
            size: entry.size(),
            compressed_size: entry.compressed_size,
            is_directory: entry.is_directory(),
            encrypted: methods.contains(&sevenz_rust2::EncoderMethod::AES256_SHA256),
            checksum: None,
            modified: entry.has_last_modified_date.then(|| {
                timestamp_from_system_time(
                    SystemTime::from(entry.last_modified_date()),
                    ArchiveTimestampPrecision::Subsecond,
                )
            }),
        });
        options.progress.advance_entry();
    }
    validate_entry_names(&entries, options.limits)?;
    Ok(entries)
}

fn test_seven_zip(path: &Path, options: &TestOptions) -> ZiFileResult<HashMap<PathBuf, String>> {
    let mut reader = SevenZReader::open(path, seven_password(options.password.as_deref()))?;
    let mut total = 0;
    let mut checksums = HashMap::new();
    let mut operation_error = None;
    let result = reader.for_each_entries(|entry, input| {
        let operation = (|| -> ZiFileResult<()> {
            options.cancellation.check()?;
            if !entry.is_directory() {
                let checksum = checksum_reader(
                    input,
                    &mut total,
                    options.limits.max_expanded_bytes,
                    Some(&options.cancellation),
                    Some(&options.progress),
                )?;
                let relative = safe_relative_path(entry.name(), options.limits.max_path_depth)?;
                checksums.insert(relative, checksum);
                options.progress.advance_entry();
            }
            Ok(())
        })();
        match operation {
            Ok(()) => Ok(true),
            Err(error) => {
                let message = error.to_string();
                operation_error = Some(error);
                Err(sevenz_rust2::Error::from(io::Error::other(message)))
            }
        }
    });
    if let Some(error) = operation_error {
        return Err(error);
    }
    result?;
    Ok(checksums)
}

fn extract_seven_zip(
    path: &Path,
    destination: &Path,
    options: &ExtractOptions,
) -> ZiFileResult<OperationSummary> {
    let mut reader = SevenZReader::open(path, seven_password(options.password.as_deref()))?;
    let mut summary = OperationSummary::default();
    let mut claimed = HashSet::new();
    let mut directory_times = Vec::new();
    let mut operation_error = None;
    let result = reader.for_each_entries(|entry, input| {
        let operation = (|| -> ZiFileResult<()> {
            options.cancellation.check()?;
            let modified = entry
                .has_last_modified_date
                .then(|| SystemTime::from(entry.last_modified_date()));
            let relative = safe_relative_path(entry.name(), options.limits.max_path_depth)?;
            if !is_selected(&relative, options) {
                return Ok(());
            }
            let Some(output) = prepare_output(
                destination,
                &relative,
                entry.is_directory(),
                options.conflict,
                &mut claimed,
            )?
            else {
                summary.skipped += 1;
                return Ok(());
            };
            if entry.is_directory() {
                fs::create_dir_all(&output)?;
                if let Some(modified) = modified {
                    directory_times.push((output, modified));
                }
                summary.directories += 1;
            } else {
                write_atomic(&output, |writer| {
                    copy_limited(
                        input,
                        writer,
                        &mut summary.bytes,
                        options.limits.max_expanded_bytes,
                        Some(&options.cancellation),
                        Some(&options.progress),
                    )?;
                    Ok(())
                })?;
                set_modified_time_if_present(&output, modified)?;
                summary.files += 1;
                options.progress.advance_entry();
            }
            Ok(())
        })();
        match operation {
            Ok(()) => Ok(true),
            Err(error) => {
                let message = error.to_string();
                operation_error = Some(error);
                Err(sevenz_rust2::Error::from(io::Error::other(message)))
            }
        }
    });
    if let Some(error) = operation_error {
        return Err(error);
    }
    result?;
    restore_directory_times(directory_times)?;
    Ok(summary)
}

fn create_seven_zip(
    sources: &[PathBuf],
    destination: &Path,
    options: &CreateOptions,
) -> ZiFileResult<OperationSummary> {
    let entries = collect_sources(sources, destination)?;
    set_source_totals(&entries, &options.progress);
    let mut temporary = temporary_archive(destination)?;
    let mut writer = SevenZWriter::new(temporary.as_file_mut())?;
    let mut content_methods = Vec::new();
    if let Some(password) = options
        .password
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        use sevenz_rust2::encoder_options::AesEncoderOptions;
        content_methods.push(AesEncoderOptions::new(Password::new(password)).into());
    }
    content_methods.push(
        sevenz_rust2::encoder_options::Lzma2Options::from_level(u32::from(
            ArchiveFormat::SevenZip.clamp_compression_level(options.compression_level),
        ))
        .into(),
    );
    writer.set_content_methods(content_methods);
    let mut summary = OperationSummary::default();
    for source in entries {
        options.cancellation.check()?;
        let entry = sevenz_rust2::ArchiveEntry::from_path(
            &source.disk_path,
            archive_name(&source.archive_path),
        );
        if source.is_directory {
            writer.push_archive_entry(entry, None::<File>)?;
            summary.directories += 1;
        } else {
            writer.push_archive_entry(
                entry,
                Some(CancellableProgressReader {
                    inner: File::open(&source.disk_path)?,
                    cancellation: options.cancellation.clone(),
                    progress: options.progress.clone(),
                }),
            )?;
            summary.files += 1;
            summary.bytes += source.size;
            options.progress.advance_entry();
        }
    }
    writer.finish()?;
    persist_archive(temporary, destination)?;
    Ok(summary)
}

fn seven_password(password: Option<&str>) -> Password {
    password.map_or_else(Password::empty, Password::new)
}

fn list_stream(
    path: &Path,
    format: ArchiveFormat,
    options: &ListOptions,
) -> ZiFileResult<Vec<ArchiveEntryInfo>> {
    options.cancellation.check()?;
    options.progress.set_totals(1, 0);
    let mut reader = open_stream_decoder(path, format, options.limits)?;
    let compressed_size = fs::metadata(path)?.len();
    // Keep the stream path consistent with declared-entry validation: a
    // caller-provided ratio of zero is a valid strict limit, rather than an
    // implicit request to allow at least one compressed byte of output.
    let ratio_limit = compressed_size.saturating_mul(options.limits.max_expansion_ratio);
    let maximum = options.limits.max_expanded_bytes.min(ratio_limit);
    let mut size = 0;
    copy_limited(
        &mut reader,
        &mut io::sink(),
        &mut size,
        maximum,
        Some(&options.cancellation),
        Some(&options.progress),
    )?;
    options.progress.advance_entry();
    Ok(vec![ArchiveEntryInfo {
        path: stream_output_name(path, format),
        size,
        compressed_size,
        is_directory: false,
        encrypted: false,
        checksum: None,
        modified: None,
    }])
}

fn test_stream(
    path: &Path,
    format: ArchiveFormat,
    options: &TestOptions,
) -> ZiFileResult<HashMap<PathBuf, String>> {
    let mut reader = open_stream_decoder(path, format, options.limits)?;
    let mut total = 0;
    let checksum = checksum_reader(
        &mut reader,
        &mut total,
        options.limits.max_expanded_bytes,
        Some(&options.cancellation),
        Some(&options.progress),
    )?;
    options.progress.advance_entry();
    let mut checksums = HashMap::new();
    checksums.insert(stream_output_name(path, format), checksum);
    Ok(checksums)
}

fn extract_stream(
    path: &Path,
    destination: &Path,
    format: ArchiveFormat,
    options: &ExtractOptions,
) -> ZiFileResult<OperationSummary> {
    let relative = stream_output_name(path, format);
    if !is_selected(&relative, options) {
        return Ok(OperationSummary::default());
    }
    let mut claimed = HashSet::new();
    let Some(output) = prepare_output(
        destination,
        &relative,
        false,
        options.conflict,
        &mut claimed,
    )?
    else {
        return Ok(OperationSummary {
            skipped: 1,
            ..OperationSummary::default()
        });
    };
    let mut reader = open_stream_decoder(path, format, options.limits)?;
    let mut bytes = 0;
    write_atomic(&output, |writer| {
        copy_limited(
            &mut reader,
            writer,
            &mut bytes,
            options.limits.max_expanded_bytes,
            Some(&options.cancellation),
            Some(&options.progress),
        )?;
        Ok(())
    })?;
    options.progress.advance_entry();
    Ok(OperationSummary {
        files: 1,
        bytes,
        ..OperationSummary::default()
    })
}

fn create_stream(
    sources: &[PathBuf],
    destination: &Path,
    format: ArchiveFormat,
    options: &CreateOptions,
) -> ZiFileResult<OperationSummary> {
    let [source] = sources else {
        return Err(ZiFileError::InvalidInput(format!(
            "{format} streams require exactly one input file; use a TAR composition for directories"
        )));
    };
    if !source.is_file() {
        return Err(ZiFileError::InvalidInput(format!(
            "{format} streams require exactly one input file; use a TAR composition for directories"
        )));
    }
    if options
        .password
        .as_deref()
        .is_some_and(|value| !value.is_empty())
    {
        return Err(ZiFileError::UnsupportedEncryption(format));
    }
    let bytes = fs::metadata(source)?.len();
    options.progress.set_totals(1, bytes);
    let mut input = BufReader::new(File::open(source)?);
    let mut temporary = temporary_archive(destination)?;
    match format {
        ArchiveFormat::Gzip => {
            let mut output = GzEncoder::new(
                temporary.as_file_mut(),
                Compression::new(u32::from(options.compression_level.min(9))),
            );
            copy_cancellable(
                &mut input,
                &mut output,
                &options.cancellation,
                &options.progress,
            )?;
            output
                .finish()
                .map_err(|error| ZiFileError::Backend(error.to_string()))?;
        }
        ArchiveFormat::Zstandard => {
            let mut output = zstd::stream::write::Encoder::new(
                temporary.as_file_mut(),
                i32::from(options.compression_level.min(22)),
            )?;
            copy_cancellable(
                &mut input,
                &mut output,
                &options.cancellation,
                &options.progress,
            )?;
            output.finish()?;
        }
        ArchiveFormat::Xz => {
            let mut output = XzEncoder::new(
                temporary.as_file_mut(),
                u32::from(options.compression_level.min(9)),
            );
            copy_cancellable(
                &mut input,
                &mut output,
                &options.cancellation,
                &options.progress,
            )?;
            output.finish()?;
        }
        ArchiveFormat::Lzma => {
            let lzma_options =
                LzmaOptions::with_preset(u32::from(options.compression_level.min(9)));
            let mut output =
                LzmaWriter::new_use_header(temporary.as_file_mut(), &lzma_options, Some(bytes))
                    .map_err(|error| ZiFileError::Backend(error.to_string()))?;
            copy_cancellable(
                &mut input,
                &mut output,
                &options.cancellation,
                &options.progress,
            )?;
            output
                .finish()
                .map_err(|error| ZiFileError::Backend(error.to_string()))?;
        }
        ArchiveFormat::Bzip2 => {
            let mut output = BzEncoder::new(
                temporary.as_file_mut(),
                bzip2::Compression::new(u32::from(options.compression_level.clamp(1, 9))),
            );
            copy_cancellable(
                &mut input,
                &mut output,
                &options.cancellation,
                &options.progress,
            )?;
            output.finish()?;
        }
        ArchiveFormat::Lz4 => {
            let mut output = FrameEncoder::new(temporary.as_file_mut());
            copy_cancellable(
                &mut input,
                &mut output,
                &options.cancellation,
                &options.progress,
            )?;
            output
                .finish()
                .map_err(|error| ZiFileError::Backend(error.to_string()))?;
        }
        ArchiveFormat::Brotli => {
            let mut output = brotli::CompressorWriter::new(
                temporary.as_file_mut(),
                64 * 1024,
                u32::from(options.compression_level.min(11)),
                22,
            );
            copy_cancellable(
                &mut input,
                &mut output,
                &options.cancellation,
                &options.progress,
            )?;
            output.flush()?;
        }
        _ => return Err(ZiFileError::UnsupportedOperation(format)),
    }
    persist_archive(temporary, destination)?;
    options.progress.advance_entry();
    Ok(OperationSummary {
        files: 1,
        bytes,
        ..OperationSummary::default()
    })
}

const LZMA_ALONE_MEMORY_LIMIT_KIB: u32 = 512 * 1024;

fn open_stream_decoder(
    path: &Path,
    format: ArchiveFormat,
    limits: SafetyLimits,
) -> ZiFileResult<Box<dyn Read>> {
    let file = BufReader::new(File::open(path)?);
    match format {
        ArchiveFormat::Gzip => Ok(Box::new(GzDecoder::new(file))),
        ArchiveFormat::Zstandard => Ok(Box::new(zstd::stream::read::Decoder::new(file)?)),
        ArchiveFormat::Lzma => {
            let memory_limit = lzma_memory_limit_kib(limits);
            LzmaReader::new_mem_limit(file, memory_limit, None)
                .map(|reader| Box::new(reader) as Box<dyn Read>)
                .map_err(|error| ZiFileError::Backend(error.to_string()))
        }
        ArchiveFormat::Xz => Ok(Box::new(XzDecoder::new(file))),
        ArchiveFormat::Bzip2 => Ok(Box::new(BzDecoder::new(file))),
        ArchiveFormat::Lz4 => Ok(Box::new(FrameDecoder::new(file))),
        ArchiveFormat::Brotli => Ok(Box::new(brotli::Decompressor::new(file, 64 * 1024))),
        _ => Err(ZiFileError::UnsupportedOperation(format)),
    }
}

fn lzma_memory_limit_kib(limits: SafetyLimits) -> u32 {
    limits
        .max_expanded_bytes
        .saturating_add(1023)
        .checked_div(1024)
        .and_then(|value| u32::try_from(value).ok())
        .unwrap_or(LZMA_ALONE_MEMORY_LIMIT_KIB)
        .clamp(1, LZMA_ALONE_MEMORY_LIMIT_KIB)
}

fn stream_output_name(path: &Path, format: ArchiveFormat) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let suffixes: &[&str] = match format {
        ArchiveFormat::Gzip => &[".gz"],
        ArchiveFormat::Zstandard => &[".zst"],
        ArchiveFormat::Xz => &[".xz"],
        ArchiveFormat::Lzma => &[".lzma"],
        ArchiveFormat::Bzip2 => &[".bz2", ".bz"],
        ArchiveFormat::Lz4 => &[".lz4"],
        ArchiveFormat::Brotli => &[".br"],
        _ => &[],
    };
    let lowercase = name.to_ascii_lowercase();
    let stripped = suffixes
        .iter()
        .find(|suffix| lowercase.ends_with(**suffix))
        .map_or_else(
            || format!("{name}.out"),
            |suffix| name[..name.len() - suffix.len()].to_owned(),
        );
    PathBuf::from(if stripped.is_empty() {
        "output"
    } else {
        &stripped
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stream_output_name_removes_canonical_and_legacy_aliases() {
        let cases = [
            ("payload.gz", ArchiveFormat::Gzip, "payload"),
            ("payload.zst", ArchiveFormat::Zstandard, "payload"),
            ("payload.XZ", ArchiveFormat::Xz, "payload"),
            ("payload.lzma", ArchiveFormat::Lzma, "payload"),
            ("payload.bz2", ArchiveFormat::Bzip2, "payload"),
            ("payload.BZ", ArchiveFormat::Bzip2, "payload"),
            ("payload.lz4", ArchiveFormat::Lz4, "payload"),
            ("payload.br", ArchiveFormat::Brotli, "payload"),
        ];
        for (path, format, expected) in cases {
            assert_eq!(
                stream_output_name(Path::new(path), format),
                PathBuf::from(expected)
            );
        }
    }

    #[test]
    fn stream_output_name_has_a_safe_fallback_for_unknown_suffixes() {
        assert_eq!(
            stream_output_name(Path::new("payload.data"), ArchiveFormat::Gzip),
            PathBuf::from("payload.data.out")
        );
        assert_eq!(
            stream_output_name(Path::new(".gz"), ArchiveFormat::Gzip),
            PathBuf::from("output")
        );
    }

    #[test]
    fn atomic_file_write_replaces_existing_file() {
        let temporary = tempfile::tempdir().unwrap();
        let destination = temporary.path().join("output.bin");
        fs::write(&destination, b"old contents").unwrap();

        write_atomic(&destination, |writer| {
            writer.write_all(b"new contents")?;
            Ok(())
        })
        .unwrap();

        assert_eq!(fs::read(&destination).unwrap(), b"new contents");
    }

    #[test]
    fn expansion_ratio_limit_uses_exact_arithmetic() {
        assert!(!expansion_ratio_exceeds_limit(2_000, 2, 1_000));
        assert!(expansion_ratio_exceeds_limit(2_001, 2, 1_000));
        assert!(expansion_ratio_exceeds_limit(u64::MAX, 1, u64::MAX - 1));
        assert!(!expansion_ratio_exceeds_limit(u64::MAX, u64::MAX, u64::MAX));
        assert!(expansion_ratio_exceeds_limit(1, 1, 0));
        assert!(!expansion_ratio_exceeds_limit(0, 0, 0));
    }
}

fn validate_declared_limits(info: &ArchiveInfo, limits: SafetyLimits) -> ZiFileResult<()> {
    if info.entries.len() as u64 > limits.max_entries {
        return Err(ZiFileError::LimitExceeded(format!(
            "entry count exceeds {}",
            limits.max_entries
        )));
    }
    if info.total_size > limits.max_expanded_bytes {
        return Err(ZiFileError::LimitExceeded(format!(
            "expanded size exceeds {} bytes",
            limits.max_expanded_bytes
        )));
    }
    if expansion_ratio_exceeds_limit(
        info.total_size,
        info.compressed_size,
        limits.max_expansion_ratio,
    ) {
        return Err(ZiFileError::LimitExceeded(format!(
            "expansion ratio exceeds {}:1",
            limits.max_expansion_ratio
        )));
    }
    validate_entry_names(&info.entries, limits)
}

fn expansion_ratio_exceeds_limit(total_size: u64, compressed_size: u64, limit: u64) -> bool {
    compressed_size > 0 && u128::from(total_size) > u128::from(compressed_size) * u128::from(limit)
}

fn is_selected(relative: &Path, options: &ExtractOptions) -> bool {
    options
        .selected_paths
        .as_ref()
        .is_none_or(|selected| selected.contains(relative))
}

fn validate_entry_names(entries: &[ArchiveEntryInfo], limits: SafetyLimits) -> ZiFileResult<()> {
    if entries.len() as u64 > limits.max_entries {
        return Err(ZiFileError::LimitExceeded("too many entries".to_owned()));
    }
    let mut names = HashSet::with_capacity(entries.len());
    for entry in entries {
        let safe = safe_relative_path(&entry.path.to_string_lossy(), limits.max_path_depth)?;
        let key = archive_collision_key(&safe);
        if !names.insert(key) {
            return Err(ZiFileError::NameCollision(entry.path.clone()));
        }
    }
    Ok(())
}

struct Sha256Sink<'a>(&'a mut Sha256);

impl Write for Sha256Sink<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.0.update(buffer);
        Ok(buffer.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn checksum_reader(
    reader: &mut dyn Read,
    total: &mut u64,
    maximum: u64,
    cancellation: Option<&CancellationToken>,
    progress: Option<&OperationProgress>,
) -> ZiFileResult<String> {
    let mut hasher = Sha256::new();
    {
        let mut sink = Sha256Sink(&mut hasher);
        copy_limited(reader, &mut sink, total, maximum, cancellation, progress)?;
    }
    Ok(hex_digest(&hasher.finalize()))
}

fn hex_digest(bytes: &[u8]) -> String {
    let mut output = String::with_capacity(bytes.len() * 2);
    for &byte in bytes {
        output.push(hex_nibble(byte >> 4));
        output.push(hex_nibble(byte & 0x0f));
    }
    output
}

fn hex_nibble(value: u8) -> char {
    match value {
        0..=9 => char::from(b'0' + value),
        10..=15 => char::from(b'a' + (value - 10)),
        _ => unreachable!("hex nibbles are always in the range 0..=15"),
    }
}

fn copy_limited(
    reader: &mut dyn Read,
    writer: &mut dyn Write,
    total: &mut u64,
    maximum: u64,
    cancellation: Option<&CancellationToken>,
    progress: Option<&OperationProgress>,
) -> ZiFileResult<u64> {
    let mut copied = 0_u64;
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        if let Some(cancellation) = cancellation {
            cancellation.check()?;
        }
        let count = reader.read(&mut buffer)?;
        if count == 0 {
            break;
        }
        let buffer_chunk = buffer.get(..count).ok_or_else(|| {
            ZiFileError::Backend("reader returned more bytes than the supplied buffer".to_owned())
        })?;
        let count = count as u64;
        *total = total
            .checked_add(count)
            .ok_or_else(|| ZiFileError::LimitExceeded("expanded size overflow".to_owned()))?;
        if *total > maximum {
            return Err(ZiFileError::LimitExceeded(format!(
                "expanded data exceeds {maximum} bytes"
            )));
        }
        writer.write_all(buffer_chunk)?;
        if let Some(progress) = progress {
            progress.advance_bytes(count);
        }
        copied += count;
    }
    Ok(copied)
}

struct CancellableProgressReader<R> {
    inner: R,
    cancellation: CancellationToken,
    progress: OperationProgress,
}

impl<R: Read> Read for CancellableProgressReader<R> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.cancellation.check().map_err(io::Error::other)?;
        let count = self.inner.read(buffer)?;
        self.progress.advance_bytes(count as u64);
        Ok(count)
    }
}

fn copy_cancellable(
    reader: &mut dyn Read,
    writer: &mut dyn Write,
    cancellation: &CancellationToken,
    progress: &OperationProgress,
) -> ZiFileResult<u64> {
    let mut total = 0;
    copy_limited(
        reader,
        writer,
        &mut total,
        u64::MAX,
        Some(cancellation),
        Some(progress),
    )
}

fn prepare_output(
    root: &Path,
    relative: &Path,
    is_directory: bool,
    policy: ConflictPolicy,
    claimed: &mut HashSet<String>,
) -> ZiFileResult<Option<PathBuf>> {
    let key = archive_collision_key(relative);
    if !claimed.insert(key) {
        return Err(ZiFileError::NameCollision(relative.to_path_buf()));
    }
    let path = root.join(relative);
    reject_symlink_components(&path)?;
    if !path.exists() {
        return Ok(Some(path));
    }
    if is_directory && path.is_dir() {
        return Ok(Some(path));
    }
    match policy {
        ConflictPolicy::Overwrite => Ok(Some(path)),
        ConflictPolicy::Skip => Ok(None),
        ConflictPolicy::Rename => Ok(Some(unique_path(&path))),
        ConflictPolicy::Error => Err(ZiFileError::DestinationExists(path)),
        ConflictPolicy::Ask => Err(ZiFileError::ConflictPolicyRequired),
    }
}

/// Reject an extraction root or output parent that resolves through a
/// symbolic link or Windows reparse point. Archive entries are already
/// normalized and link entries are rejected separately, but a pre-existing
/// host link could otherwise redirect an apparently safe relative output
/// outside the selected destination.
fn reject_symlink_components(path: &Path) -> ZiFileResult<()> {
    let mut current = path.to_path_buf();
    loop {
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata_is_link_like(&metadata) => {
                return Err(ZiFileError::UnsafeDestination(current));
            }
            Ok(_) => {}
            Err(error) if error.kind() != io::ErrorKind::NotFound => {
                return Err(ZiFileError::Io(error));
            }
            Err(_) => {}
        }

        let Some(parent) = current.parent().map(Path::to_path_buf) else {
            break;
        };
        if parent == current {
            break;
        }
        current = parent;
    }
    Ok(())
}

fn metadata_is_link_like(metadata: &fs::Metadata) -> bool {
    if metadata.file_type().is_symlink() {
        return true;
    }

    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;

        u64::from(metadata.file_attributes()) & WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT != 0
    }

    #[cfg(not(windows))]
    {
        false
    }
}

fn unique_path(path: &Path) -> PathBuf {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let extension = path.extension().map(|value| value.to_string_lossy());
    for index in 2..=10_000 {
        let mut name = format!("{stem} ({index})");
        if let Some(extension) = &extension {
            name.push('.');
            name.push_str(extension);
        }
        let candidate = parent.join(name);
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem} (copy)"))
}

fn timestamp_from_primitive(
    value: time::PrimitiveDateTime,
    offset: ArchiveTimestampOffset,
    precision: ArchiveTimestampPrecision,
) -> ArchiveTimestamp {
    ArchiveTimestamp {
        year: value.year().try_into().unwrap_or_default(),
        month: value.month() as u8,
        day: value.day(),
        hour: value.hour(),
        minute: value.minute(),
        second: value.second(),
        nanosecond: value.nanosecond(),
        offset,
        precision,
    }
}

fn timestamp_from_system_time(
    value: SystemTime,
    precision: ArchiveTimestampPrecision,
) -> ArchiveTimestamp {
    let value = time::OffsetDateTime::from(value);
    timestamp_from_primitive(
        time::PrimitiveDateTime::new(value.date(), value.time()),
        ArchiveTimestampOffset::Utc,
        precision,
    )
}

fn timestamp_from_unix_seconds(value: u64) -> Option<ArchiveTimestamp> {
    let value = i64::try_from(value).ok()?;
    let value = time::OffsetDateTime::from_unix_timestamp(value).ok()?;
    Some(timestamp_from_primitive(
        time::PrimitiveDateTime::new(value.date(), value.time()),
        ArchiveTimestampOffset::Utc,
        ArchiveTimestampPrecision::Second,
    ))
}

fn timestamp_from_zip_datetime(value: zip::DateTime) -> ArchiveTimestamp {
    ArchiveTimestamp {
        year: value.year(),
        month: value.month(),
        day: value.day(),
        hour: value.hour(),
        minute: value.minute(),
        second: value.second(),
        nanosecond: 0,
        offset: ArchiveTimestampOffset::Unspecified,
        precision: ArchiveTimestampPrecision::TwoSeconds,
    }
}

fn rar_archive_timestamp(value: u32) -> Option<ArchiveTimestamp> {
    if value == 0 {
        return None;
    }
    let date = (value >> 16) as u16;
    let time = value as u16;
    zip::DateTime::try_from((date, time))
        .ok()
        .map(timestamp_from_zip_datetime)
}

fn primitive_datetime_to_system_time(value: time::PrimitiveDateTime) -> SystemTime {
    SystemTime::from(value.assume_utc())
}

fn zip_datetime_to_system_time(value: zip::DateTime) -> Option<SystemTime> {
    time::PrimitiveDateTime::try_from(value)
        .ok()
        .map(primitive_datetime_to_system_time)
}

fn system_time_to_zip_datetime(value: SystemTime) -> Option<zip::DateTime> {
    let value = time::OffsetDateTime::from(value);
    zip::DateTime::try_from(time::PrimitiveDateTime::new(value.date(), value.time())).ok()
}

fn rar_dos_time(value: SystemTime) -> Option<u32> {
    let value = system_time_to_zip_datetime(value)?;
    let year = value.year();
    if !(1980..=2107).contains(&year) {
        return None;
    }
    let date =
        (u32::from(year - 1980) << 9) | (u32::from(value.month()) << 5) | u32::from(value.day());
    let time = (u32::from(value.hour()) << 11)
        | (u32::from(value.minute()) << 5)
        | u32::from(value.second() / 2);
    Some((date << 16) | time)
}

fn rar_modified_time(value: u32, refinement: Option<rars::TimeRefinement>) -> Option<SystemTime> {
    if value == 0 {
        return None;
    }
    let date = (value >> 16) as u16;
    let time = value as u16;
    let mut modified = zip_datetime_to_system_time(zip::DateTime::try_from((date, time)).ok()?)?;
    if let Some(refinement) = refinement {
        modified = modified.checked_add(Duration::new(
            u64::from(refinement.add_second),
            refinement.nanoseconds,
        ))?;
    }
    Some(modified)
}

fn set_modified_time_if_present(path: &Path, modified: Option<SystemTime>) -> ZiFileResult<()> {
    if let Some(modified) = modified {
        filetime::set_file_mtime(path, filetime::FileTime::from_system_time(modified))?;
    }
    Ok(())
}

fn restore_directory_times(mut entries: Vec<(PathBuf, SystemTime)>) -> ZiFileResult<()> {
    entries.sort_by(|(left, _), (right, _)| {
        right.components().count().cmp(&left.components().count())
    });
    for (path, modified) in entries {
        set_modified_time_if_present(&path, Some(modified))?;
    }
    Ok(())
}

fn write_atomic(
    destination: &Path,
    write: impl FnOnce(&mut dyn Write) -> ZiFileResult<()>,
) -> ZiFileResult<()> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    write(temporary.as_file_mut())?;
    temporary.as_file_mut().sync_all()?;
    if destination.is_dir() {
        return Err(ZiFileError::DestinationExists(destination.to_path_buf()));
    }
    temporary
        .persist(destination)
        .map_err(|error| ZiFileError::Io(error.error))?;
    Ok(())
}

#[derive(Debug)]
struct SourceEntry {
    disk_path: PathBuf,
    archive_path: PathBuf,
    is_directory: bool,
    size: u64,
    modified: Option<SystemTime>,
}

fn set_source_totals(entries: &[SourceEntry], progress: &OperationProgress) {
    let files = entries.iter().filter(|entry| !entry.is_directory);
    let (count, bytes) = files.fold((0_u64, 0_u64), |(count, bytes), entry| {
        (count + 1, bytes.saturating_add(entry.size))
    });
    progress.set_totals(count, bytes);
}

fn collect_sources(sources: &[PathBuf], destination: &Path) -> ZiFileResult<Vec<SourceEntry>> {
    let mut result = Vec::new();
    let mut names = HashSet::new();
    for source in sources {
        if !source.exists() {
            return Err(ZiFileError::InvalidInput(format!(
                "source does not exist: {}",
                source.display()
            )));
        }
        let root_name = source.file_name().ok_or_else(|| {
            ZiFileError::InvalidInput(format!("source has no file name: {}", source.display()))
        })?;
        for item in WalkDir::new(source).follow_links(false) {
            let item = item.map_err(|error| ZiFileError::Backend(error.to_string()))?;
            let metadata = fs::symlink_metadata(item.path())?;
            if metadata_is_link_like(&metadata) {
                return Err(ZiFileError::LinkEntry(item.path().display().to_string()));
            }
            if item.path() == destination {
                return Err(ZiFileError::InvalidInput(
                    "destination cannot be one of the sources".to_owned(),
                ));
            }
            let relative = item.path().strip_prefix(source).map_err(|error| {
                ZiFileError::InvalidInput(format!("cannot relativize source: {error}"))
            })?;
            let archive_path = PathBuf::from(root_name).join(relative);
            let safe = safe_relative_path(
                &archive_path.to_string_lossy(),
                SafetyLimits::default().max_path_depth,
            )?;
            let key = archive_collision_key(&safe);
            if !names.insert(key) {
                return Err(ZiFileError::NameCollision(safe));
            }
            result.push(SourceEntry {
                disk_path: item.path().to_path_buf(),
                archive_path: safe,
                is_directory: metadata.is_dir(),
                size: if metadata.is_file() {
                    metadata.len()
                } else {
                    0
                },
                modified: metadata.modified().ok(),
            });
        }
    }
    result.sort_by(|left, right| left.archive_path.cmp(&right.archive_path));
    Ok(result)
}

fn archive_name(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn archive_collision_key(path: &Path) -> String {
    let name = archive_name(path);
    if cfg!(windows) {
        name.to_lowercase()
    } else {
        name.to_ascii_lowercase()
    }
}

fn temporary_archive(destination: &Path) -> ZiFileResult<NamedTempFile> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    Ok(NamedTempFile::new_in(parent)?)
}

fn canonicalize_with_missing(path: &Path) -> ZiFileResult<PathBuf> {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()?.join(path)
    };
    let mut existing = absolute;
    let mut missing = Vec::new();
    while !existing.exists() {
        let Some(name) = existing.file_name() else {
            return Err(ZiFileError::InvalidInput(format!(
                "path has no file name: {}",
                path.display()
            )));
        };
        missing.push(name.to_os_string());
        if !existing.pop() {
            return Err(ZiFileError::InvalidInput(format!(
                "path could not be resolved: {}",
                path.display()
            )));
        }
    }
    let mut canonical = fs::canonicalize(existing)?;
    for name in missing.iter().rev() {
        canonical.push(name);
    }
    Ok(canonical)
}

fn path_is_same_or_descendant(candidate: &Path, root: &Path) -> bool {
    let normalize = |path: &Path| {
        let value = path.to_string_lossy().replace('\\', "/");
        if cfg!(windows) {
            value.to_lowercase()
        } else {
            value
        }
    };
    let candidate = normalize(candidate);
    let root = normalize(root);
    if candidate == root {
        return true;
    }
    let prefix = if root.ends_with('/') {
        root
    } else {
        format!("{root}/")
    };
    candidate.starts_with(prefix.as_str())
}

fn persist_archive(mut temporary: NamedTempFile, destination: &Path) -> ZiFileResult<()> {
    temporary.as_file_mut().sync_all()?;
    if destination.exists() {
        return Err(ZiFileError::DestinationExists(destination.to_path_buf()));
    }
    temporary
        .persist(destination)
        .map_err(|error| ZiFileError::Io(error.error))?;
    Ok(())
}
