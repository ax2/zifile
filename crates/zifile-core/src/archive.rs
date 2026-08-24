use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufReader, BufWriter, Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use bzip2::read::BzDecoder;
use bzip2::write::BzEncoder;
use flate2::Compression;
use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use lz4_flex::frame::{FrameDecoder, FrameEncoder};
use serde::{Deserialize, Serialize};
use sevenz_rust2::{ArchiveReader as SevenZReader, ArchiveWriter as SevenZWriter, Password};
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
    list_archive_with_limits(path, password, SafetyLimits::default())
}

pub fn list_archive_with_limits(
    path: impl AsRef<Path>,
    password: Option<&str>,
    limits: SafetyLimits,
) -> ZiFileResult<ArchiveInfo> {
    let path = path.as_ref();
    let format = detect_format(path)?;
    let entries = match format {
        ArchiveFormat::Zip => list_zip(path, password, limits)?,
        ArchiveFormat::SevenZip => list_seven_zip(path, password, limits)?,
        ArchiveFormat::Tar
        | ArchiveFormat::TarGzip
        | ArchiveFormat::TarZstd
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarBzip2 => list_tar(path, format, limits)?,
        ArchiveFormat::Gzip
        | ArchiveFormat::Zstandard
        | ArchiveFormat::Xz
        | ArchiveFormat::Bzip2
        | ArchiveFormat::Lz4
        | ArchiveFormat::Brotli => list_stream(path, format, limits)?,
        ArchiveFormat::Rar => return Err(ZiFileError::UnsupportedOperation(format)),
    };

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
    validate_declared_limits(&info, limits)?;
    Ok(info)
}

pub fn test_archive(path: impl AsRef<Path>, password: Option<&str>) -> ZiFileResult<ArchiveInfo> {
    test_archive_with_limits(path, password, SafetyLimits::default())
}

pub fn test_archive_with_limits(
    path: impl AsRef<Path>,
    password: Option<&str>,
    limits: SafetyLimits,
) -> ZiFileResult<ArchiveInfo> {
    let path = path.as_ref();
    let info = list_archive_with_limits(path, password, limits)?;
    validate_declared_limits(&info, limits)?;
    match info.format {
        ArchiveFormat::Zip => test_zip(path, password, limits)?,
        ArchiveFormat::SevenZip => test_seven_zip(path, password, limits)?,
        ArchiveFormat::Tar
        | ArchiveFormat::TarGzip
        | ArchiveFormat::TarZstd
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarBzip2 => test_tar(path, info.format, limits)?,
        ArchiveFormat::Gzip
        | ArchiveFormat::Zstandard
        | ArchiveFormat::Xz
        | ArchiveFormat::Bzip2
        | ArchiveFormat::Lz4
        | ArchiveFormat::Brotli => {
            let mut reader = open_stream_decoder(path, info.format)?;
            let mut total = 0;
            copy_limited(
                &mut reader,
                &mut io::sink(),
                &mut total,
                limits.max_expanded_bytes,
                None,
                None,
            )?;
        }
        ArchiveFormat::Rar => return Err(ZiFileError::UnsupportedOperation(info.format)),
    }
    Ok(info)
}

pub fn extract_archive(
    archive: impl AsRef<Path>,
    destination: impl AsRef<Path>,
    options: &ExtractOptions,
) -> ZiFileResult<OperationSummary> {
    if options.conflict == ConflictPolicy::Ask {
        return Err(ZiFileError::ConflictPolicyRequired);
    }
    let archive = archive.as_ref();
    let destination = destination.as_ref();
    let info = list_archive_with_limits(archive, options.password.as_deref(), options.limits)?;
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
    options.progress.set_totals(entry_count, total_bytes);
    fs::create_dir_all(destination)?;

    match info.format {
        ArchiveFormat::Zip => extract_zip(archive, destination, options),
        ArchiveFormat::SevenZip => extract_seven_zip(archive, destination, options),
        ArchiveFormat::Tar
        | ArchiveFormat::TarGzip
        | ArchiveFormat::TarZstd
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarBzip2 => extract_tar(archive, destination, info.format, options),
        ArchiveFormat::Gzip
        | ArchiveFormat::Zstandard
        | ArchiveFormat::Xz
        | ArchiveFormat::Bzip2
        | ArchiveFormat::Lz4
        | ArchiveFormat::Brotli => extract_stream(archive, destination, info.format, options),
        ArchiveFormat::Rar => Err(ZiFileError::UnsupportedOperation(info.format)),
    }
}

pub fn create_archive(
    sources: &[PathBuf],
    destination: impl AsRef<Path>,
    format: ArchiveFormat,
    options: &CreateOptions,
) -> ZiFileResult<OperationSummary> {
    if sources.is_empty() {
        return Err(ZiFileError::InvalidInput(
            "at least one source is required".to_owned(),
        ));
    }
    if !format.capabilities().create {
        return Err(ZiFileError::UnsupportedOperation(format));
    }
    let destination = destination.as_ref();
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;

    match format {
        ArchiveFormat::Zip => create_zip(sources, destination, options),
        ArchiveFormat::SevenZip => create_seven_zip(sources, destination, options),
        ArchiveFormat::Tar
        | ArchiveFormat::TarGzip
        | ArchiveFormat::TarZstd
        | ArchiveFormat::TarXz
        | ArchiveFormat::TarBzip2 => create_tar(sources, destination, format, options),
        ArchiveFormat::Gzip
        | ArchiveFormat::Zstandard
        | ArchiveFormat::Xz
        | ArchiveFormat::Bzip2
        | ArchiveFormat::Lz4
        | ArchiveFormat::Brotli => create_stream(sources, destination, format, options),
        ArchiveFormat::Rar => Err(ZiFileError::UnsupportedOperation(format)),
    }
}

fn list_zip(
    path: &Path,
    _password: Option<&str>,
    limits: SafetyLimits,
) -> ZiFileResult<Vec<ArchiveEntryInfo>> {
    let mut archive = ZipArchive::new(BufReader::new(File::open(path)?))?;
    if archive.len() as u64 > limits.max_entries {
        return Err(ZiFileError::LimitExceeded(format!(
            "entry count exceeds {}",
            limits.max_entries
        )));
    }
    let mut entries = Vec::with_capacity(archive.len());
    for index in 0..archive.len() {
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
        });
    }
    validate_entry_names(&entries, limits)?;
    Ok(entries)
}

fn test_zip(path: &Path, password: Option<&str>, limits: SafetyLimits) -> ZiFileResult<()> {
    let mut archive = ZipArchive::new(BufReader::new(File::open(path)?))?;
    let mut total = 0;
    for index in 0..archive.len() {
        let mut entry = open_zip_entry(&mut archive, index, password)?;
        reject_zip_link(entry.unix_mode(), entry.name())?;
        if !entry.is_dir() {
            copy_limited(
                &mut entry,
                &mut io::sink(),
                &mut total,
                limits.max_expanded_bytes,
                None,
                None,
            )?;
        }
    }
    Ok(())
}

fn extract_zip(
    path: &Path,
    destination: &Path,
    options: &ExtractOptions,
) -> ZiFileResult<OperationSummary> {
    let mut archive = ZipArchive::new(BufReader::new(File::open(path)?))?;
    let mut summary = OperationSummary::default();
    let mut claimed = HashSet::new();
    for index in 0..archive.len() {
        options.cancellation.check()?;
        let mut entry = open_zip_entry(&mut archive, index, options.password.as_deref())?;
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
            summary.files += 1;
            options.progress.advance_entry();
        }
    }
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
    limits: SafetyLimits,
) -> ZiFileResult<Vec<ArchiveEntryInfo>> {
    let mut archive = tar::Archive::new(open_tar_reader(path, format)?);
    let mut result = Vec::new();
    for entry in archive.entries()? {
        if result.len() as u64 >= limits.max_entries {
            return Err(ZiFileError::LimitExceeded(format!(
                "entry count exceeds {}",
                limits.max_entries
            )));
        }
        let entry = entry?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(ZiFileError::LinkEntry(
                entry.path()?.to_string_lossy().into_owned(),
            ));
        }
        let path = entry.path()?.into_owned();
        let safe = safe_relative_path(&path.to_string_lossy(), limits.max_path_depth)?;
        result.push(ArchiveEntryInfo {
            path: safe,
            size: entry.size(),
            compressed_size: 0,
            is_directory: entry_type.is_dir(),
            encrypted: false,
        });
    }
    validate_entry_names(&result, limits)?;
    Ok(result)
}

fn test_tar(path: &Path, format: ArchiveFormat, limits: SafetyLimits) -> ZiFileResult<()> {
    let mut archive = tar::Archive::new(open_tar_reader(path, format)?);
    let mut total = 0;
    for entry in archive.entries()? {
        let mut entry = entry?;
        let entry_type = entry.header().entry_type();
        if entry_type.is_symlink() || entry_type.is_hard_link() {
            return Err(ZiFileError::LinkEntry(
                entry.path()?.to_string_lossy().into_owned(),
            ));
        }
        if entry_type.is_file() {
            copy_limited(
                &mut entry,
                &mut io::sink(),
                &mut total,
                limits.max_expanded_bytes,
                None,
                None,
            )?;
        }
    }
    Ok(())
}

fn extract_tar(
    path: &Path,
    destination: &Path,
    format: ArchiveFormat,
    options: &ExtractOptions,
) -> ZiFileResult<OperationSummary> {
    let mut archive = tar::Archive::new(open_tar_reader(path, format)?);
    let mut summary = OperationSummary::default();
    let mut claimed = HashSet::new();
    for entry in archive.entries()? {
        options.cancellation.check()?;
        let mut entry = entry?;
        let entry_type = entry.header().entry_type();
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
            summary.files += 1;
            options.progress.advance_entry();
        } else {
            return Err(ZiFileError::UnsupportedEntry(original));
        }
    }
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
        ArchiveFormat::TarBzip2 => write_tar(
            &entries,
            BzEncoder::new(
                BufWriter::new(temporary.as_file_mut()),
                bzip2::Compression::new(u32::from(options.compression_level.clamp(1, 9))),
            ),
            &options.cancellation,
            &options.progress,
        )?,
        _ => return Err(ZiFileError::UnsupportedOperation(format)),
    };
    persist_archive(temporary, destination)?;
    Ok(summary)
}

fn write_tar<W: Write>(
    entries: &[SourceEntry],
    output: W,
    cancellation: &CancellationToken,
    progress: &OperationProgress,
) -> ZiFileResult<OperationSummary> {
    let mut archive = tar::Builder::new(output);
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
    archive.finish()?;
    Ok(summary)
}

fn open_tar_reader(path: &Path, format: ArchiveFormat) -> ZiFileResult<Box<dyn Read>> {
    let file = BufReader::new(File::open(path)?);
    match format {
        ArchiveFormat::Tar => Ok(Box::new(file)),
        ArchiveFormat::TarGzip => Ok(Box::new(GzDecoder::new(file))),
        ArchiveFormat::TarZstd => Ok(Box::new(zstd::stream::read::Decoder::new(file)?)),
        ArchiveFormat::TarXz => Ok(Box::new(XzDecoder::new(file))),
        ArchiveFormat::TarBzip2 => Ok(Box::new(BzDecoder::new(file))),
        _ => Err(ZiFileError::UnsupportedOperation(format)),
    }
}

fn list_seven_zip(
    path: &Path,
    password: Option<&str>,
    limits: SafetyLimits,
) -> ZiFileResult<Vec<ArchiveEntryInfo>> {
    let reader = SevenZReader::open(path, seven_password(password))?;
    if reader.archive().files.len() as u64 > limits.max_entries {
        return Err(ZiFileError::LimitExceeded(format!(
            "entry count exceeds {}",
            limits.max_entries
        )));
    }
    let entries = reader
        .archive()
        .files
        .iter()
        .map(|entry| {
            Ok(ArchiveEntryInfo {
                path: safe_relative_path(entry.name(), limits.max_path_depth)?,
                size: entry.size(),
                compressed_size: entry.compressed_size,
                is_directory: entry.is_directory(),
                encrypted: false,
            })
        })
        .collect::<ZiFileResult<Vec<_>>>()?;
    validate_entry_names(&entries, limits)?;
    Ok(entries)
}

fn test_seven_zip(path: &Path, password: Option<&str>, limits: SafetyLimits) -> ZiFileResult<()> {
    let mut reader = SevenZReader::open(path, seven_password(password))?;
    let mut total = 0;
    reader.for_each_entries(|entry, input| {
        if !entry.is_directory() {
            copy_limited(
                input,
                &mut io::sink(),
                &mut total,
                limits.max_expanded_bytes,
                None,
                None,
            )
            .map_err(|error| sevenz_rust2::Error::from(io::Error::other(error.to_string())))?;
        }
        Ok(true)
    })?;
    Ok(())
}

fn extract_seven_zip(
    path: &Path,
    destination: &Path,
    options: &ExtractOptions,
) -> ZiFileResult<OperationSummary> {
    let mut reader = SevenZReader::open(path, seven_password(options.password.as_deref()))?;
    let mut summary = OperationSummary::default();
    let mut claimed = HashSet::new();
    let mut operation_error = None;
    let result = reader.for_each_entries(|entry, input| {
        let operation = (|| -> ZiFileResult<()> {
            options.cancellation.check()?;
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
                fs::create_dir_all(output)?;
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
    if let Some(password) = options
        .password
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        use sevenz_rust2::encoder_options::AesEncoderOptions;
        writer.set_content_methods(vec![
            AesEncoderOptions::new(Password::new(password)).into(),
            sevenz_rust2::EncoderMethod::LZMA2.into(),
        ]);
    }
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
    limits: SafetyLimits,
) -> ZiFileResult<Vec<ArchiveEntryInfo>> {
    let mut reader = open_stream_decoder(path, format)?;
    let compressed_size = fs::metadata(path)?.len();
    let ratio_limit = compressed_size
        .saturating_mul(limits.max_expansion_ratio)
        .max(compressed_size);
    let maximum = limits.max_expanded_bytes.min(ratio_limit);
    let mut size = 0;
    copy_limited(&mut reader, &mut io::sink(), &mut size, maximum, None, None)?;
    Ok(vec![ArchiveEntryInfo {
        path: stream_output_name(path, format),
        size,
        compressed_size,
        is_directory: false,
        encrypted: false,
    }])
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
    let mut reader = open_stream_decoder(path, format)?;
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
    if sources.len() != 1 || !sources[0].is_file() {
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
    let source = &sources[0];
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

fn open_stream_decoder(path: &Path, format: ArchiveFormat) -> ZiFileResult<Box<dyn Read>> {
    let file = BufReader::new(File::open(path)?);
    match format {
        ArchiveFormat::Gzip => Ok(Box::new(GzDecoder::new(file))),
        ArchiveFormat::Zstandard => Ok(Box::new(zstd::stream::read::Decoder::new(file)?)),
        ArchiveFormat::Xz => Ok(Box::new(XzDecoder::new(file))),
        ArchiveFormat::Bzip2 => Ok(Box::new(BzDecoder::new(file))),
        ArchiveFormat::Lz4 => Ok(Box::new(FrameDecoder::new(file))),
        ArchiveFormat::Brotli => Ok(Box::new(brotli::Decompressor::new(file, 64 * 1024))),
        _ => Err(ZiFileError::UnsupportedOperation(format)),
    }
}

fn stream_output_name(path: &Path, format: ArchiveFormat) -> PathBuf {
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let suffix = format!(".{}", format.canonical_extension());
    let stripped = name.to_ascii_lowercase().strip_suffix(&suffix).map_or_else(
        || format!("{name}.out"),
        |_| name[..name.len() - suffix.len()].to_owned(),
    );
    PathBuf::from(if stripped.is_empty() {
        "output"
    } else {
        &stripped
    })
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
    if info.compressed_size > 0
        && info.total_size / info.compressed_size.max(1) > limits.max_expansion_ratio
    {
        return Err(ZiFileError::LimitExceeded(format!(
            "expansion ratio exceeds {}:1",
            limits.max_expansion_ratio
        )));
    }
    validate_entry_names(&info.entries, limits)
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
        let key = archive_name(&safe).to_ascii_lowercase();
        if !names.insert(key) {
            return Err(ZiFileError::NameCollision(entry.path.clone()));
        }
    }
    Ok(())
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
        let count = count as u64;
        *total = total
            .checked_add(count)
            .ok_or_else(|| ZiFileError::LimitExceeded("expanded size overflow".to_owned()))?;
        if *total > maximum {
            return Err(ZiFileError::LimitExceeded(format!(
                "expanded data exceeds {maximum} bytes"
            )));
        }
        writer.write_all(&buffer[..count as usize])?;
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
    let key = archive_name(relative).to_ascii_lowercase();
    if !claimed.insert(key) {
        return Err(ZiFileError::NameCollision(relative.to_path_buf()));
    }
    let path = root.join(relative);
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

fn write_atomic(
    destination: &Path,
    write: impl FnOnce(&mut dyn Write) -> ZiFileResult<()>,
) -> ZiFileResult<()> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent)?;
    let mut temporary = NamedTempFile::new_in(parent)?;
    write(temporary.as_file_mut())?;
    temporary.as_file_mut().sync_all()?;
    if destination.exists() {
        if destination.is_dir() {
            return Err(ZiFileError::DestinationExists(destination.to_path_buf()));
        }
        fs::remove_file(destination)?;
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
            if metadata.file_type().is_symlink() {
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
            let key = archive_name(&safe).to_ascii_lowercase();
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

fn temporary_archive(destination: &Path) -> ZiFileResult<NamedTempFile> {
    let parent = destination.parent().unwrap_or_else(|| Path::new("."));
    Ok(NamedTempFile::new_in(parent)?)
}

fn persist_archive(mut temporary: NamedTempFile, destination: &Path) -> ZiFileResult<()> {
    temporary.as_file_mut().sync_all()?;
    if destination.exists() {
        fs::remove_file(destination)?;
    }
    temporary
        .persist(destination)
        .map_err(|error| ZiFileError::Io(error.error))?;
    Ok(())
}
