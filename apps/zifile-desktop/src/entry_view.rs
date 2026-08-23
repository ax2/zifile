use zifile_core::{ArchiveEntryInfo, ArchiveInfo};

pub const ENTRIES_PER_PAGE: usize = 500;

pub fn filtered_entry_count(archive: &ArchiveInfo, filter: &str) -> usize {
    let filter_lower = filter.to_lowercase();
    archive
        .entries
        .iter()
        .filter(|entry| entry_matches_filter(entry, &filter_lower))
        .count()
}

pub fn filtered_entry_page<'a>(
    archive: &'a ArchiveInfo,
    filter: &str,
    page: usize,
) -> Vec<&'a ArchiveEntryInfo> {
    let filter_lower = filter.to_lowercase();
    archive
        .entries
        .iter()
        .filter(|entry| entry_matches_filter(entry, &filter_lower))
        .skip(page.saturating_mul(ENTRIES_PER_PAGE))
        .take(ENTRIES_PER_PAGE)
        .collect()
}

fn entry_matches_filter(entry: &ArchiveEntryInfo, filter_lower: &str) -> bool {
    filter_lower.is_empty()
        || entry
            .path
            .to_string_lossy()
            .to_lowercase()
            .contains(filter_lower)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use zifile_core::ArchiveFormat;

    use super::*;

    fn large_archive() -> ArchiveInfo {
        ArchiveInfo {
            path: PathBuf::from("large.zip"),
            format: ArchiveFormat::Zip,
            entries: (0..100_000)
                .map(|index| ArchiveEntryInfo {
                    path: PathBuf::from(format!("folder/file-{index:06}.txt")),
                    size: 1,
                    compressed_size: 1,
                    is_directory: false,
                    encrypted: false,
                })
                .collect(),
            total_size: 100_000,
            compressed_size: 100_000,
        }
    }

    #[test]
    fn large_archive_filtering_keeps_pages_bounded() {
        let archive = large_archive();
        assert_eq!(filtered_entry_count(&archive, "file-09"), 10_000);
        let page = filtered_entry_page(&archive, "file-09", 4);
        assert_eq!(page.len(), ENTRIES_PER_PAGE);
        assert_eq!(page[0].path, PathBuf::from("folder/file-092000.txt"));
    }

    #[test]
    fn page_offset_saturates_without_panicking() {
        let archive = large_archive();
        assert!(filtered_entry_page(&archive, "", usize::MAX).is_empty());
    }
}
