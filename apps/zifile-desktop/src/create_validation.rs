use std::path::PathBuf;

use zifile_core::{ArchiveFormat, CreateInputKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CreateSourceIssue {
    MissingSources,
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
        Some(CreateInputKind::SingleFile) if sources.len() != 1 || !sources[0].is_file() => {
            Some(CreateSourceIssue::SingleFileRequired)
        }
        Some(_) => None,
    }
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
    fn missing_and_read_only_inputs_are_reported() {
        assert_eq!(
            create_source_issue(ArchiveFormat::Zip, &[]),
            Some(CreateSourceIssue::MissingSources)
        );
        assert_eq!(
            create_source_issue(ArchiveFormat::Rar, &[PathBuf::from("source")]),
            Some(CreateSourceIssue::UnsupportedFormat)
        );
    }
}
