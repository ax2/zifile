use std::path::PathBuf;

use zifile_core::{ArchiveFormat, CreateInputKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateSourceIssue {
    MissingSources,
    MissingSource,
    LinkSource,
    SingleFileRequired,
    UnsupportedFormat,
}

/// Validates source selection before the UI opens a destination dialog.
pub fn create_source_issue(
    format: ArchiveFormat,
    sources: &[PathBuf],
) -> Option<CreateSourceIssue> {
    match format.create_input() {
        None => Some(CreateSourceIssue::UnsupportedFormat),
        Some(_) if sources.is_empty() => Some(CreateSourceIssue::MissingSources),
        Some(_) if sources.iter().any(|source| !source.exists()) => {
            Some(CreateSourceIssue::MissingSource)
        }
        Some(_) if sources.iter().any(|source| is_link_like(source)) => {
            Some(CreateSourceIssue::LinkSource)
        }
        Some(CreateInputKind::SingleFile) if !matches!(sources, [source] if source.is_file()) => {
            Some(CreateSourceIssue::SingleFileRequired)
        }
        Some(_) => None,
    }
}

#[cfg(windows)]
const WINDOWS_FILE_ATTRIBUTE_REPARSE_POINT: u64 = 0x400;

fn is_link_like(path: &std::path::Path) -> bool {
    std::fs::symlink_metadata(path).is_ok_and(|metadata| {
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_formats_accept_files_and_folders() {
        let temp = tempfile::tempdir().unwrap();
        let sources = vec![temp.path().to_path_buf()];
        assert_eq!(create_source_issue(ArchiveFormat::Zip, &sources), None);
        assert_eq!(create_source_issue(ArchiveFormat::TarZstd, &sources), None);
    }

    #[test]
    fn stream_formats_require_exactly_one_existing_file() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("source.txt");
        std::fs::write(&file, b"source").unwrap();

        for format in [
            ArchiveFormat::Gzip,
            ArchiveFormat::Zstandard,
            ArchiveFormat::Xz,
            ArchiveFormat::Lzma,
            ArchiveFormat::Bzip2,
            ArchiveFormat::Lz4,
            ArchiveFormat::Brotli,
        ] {
            assert_eq!(
                create_source_issue(format, std::slice::from_ref(&file)),
                None
            );
            assert_eq!(
                create_source_issue(format, &[file.clone(), file.clone()]),
                Some(CreateSourceIssue::SingleFileRequired)
            );
            assert_eq!(
                create_source_issue(format, &[temp.path().to_path_buf()]),
                Some(CreateSourceIssue::SingleFileRequired)
            );
        }
    }

    #[test]
    fn missing_inputs_are_reported() {
        assert_eq!(
            create_source_issue(ArchiveFormat::Zip, &[]),
            Some(CreateSourceIssue::MissingSources)
        );
        assert_eq!(
            create_source_issue(ArchiveFormat::Zip, &[PathBuf::from("missing-source")]),
            Some(CreateSourceIssue::MissingSource)
        );
    }

    #[cfg(unix)]
    #[test]
    fn symlink_sources_are_rejected_before_the_save_dialog() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("source.txt");
        let link = temp.path().join("source-link.txt");
        std::fs::write(&source, b"source").unwrap();
        std::os::unix::fs::symlink(&source, &link).unwrap();

        assert_eq!(
            create_source_issue(ArchiveFormat::Zip, &[link]),
            Some(CreateSourceIssue::LinkSource)
        );
    }

    #[cfg(windows)]
    #[test]
    fn reparse_point_sources_are_rejected_before_the_save_dialog() {
        let temp = tempfile::tempdir().unwrap();
        let target = temp.path().join("target");
        let link = temp.path().join("source-link");
        std::fs::create_dir(&target).unwrap();
        if let Err(error) = std::os::windows::fs::symlink_dir(&target, &link) {
            if error.kind() == std::io::ErrorKind::PermissionDenied {
                return;
            }
            panic!("could not create test symlink: {error}");
        }

        assert_eq!(
            create_source_issue(ArchiveFormat::Zip, &[link]),
            Some(CreateSourceIssue::LinkSource)
        );
    }
}
