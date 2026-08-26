use std::cmp::Ordering;

use zifile_core::{ArchiveEntryInfo, ArchiveInfo};

pub const ENTRIES_PER_PAGE: usize = 500;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EntrySort {
    #[default]
    Name,
    Size,
    Packed,
    Modified,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SortDirection {
    #[default]
    Ascending,
    Descending,
}

impl SortDirection {
    pub const fn toggle(self) -> Self {
        match self {
            Self::Ascending => Self::Descending,
            Self::Descending => Self::Ascending,
        }
    }
}

pub fn next_sort(
    current: EntrySort,
    direction: SortDirection,
    requested: EntrySort,
) -> (EntrySort, SortDirection) {
    if current == requested {
        (current, direction.toggle())
    } else {
        (requested, SortDirection::Ascending)
    }
}

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
    sorted_filtered_entry_page(
        archive,
        filter,
        page,
        EntrySort::Name,
        SortDirection::Ascending,
    )
}

pub fn sorted_filtered_entry_page<'a>(
    archive: &'a ArchiveInfo,
    filter: &str,
    page: usize,
    sort: EntrySort,
    direction: SortDirection,
) -> Vec<&'a ArchiveEntryInfo> {
    let filter_lower = filter.to_lowercase();
    let mut entries = archive
        .entries
        .iter()
        .filter(|entry| entry_matches_filter(entry, &filter_lower))
        .collect::<Vec<_>>();
    entries.sort_unstable_by(|left, right| compare_entries(left, right, sort, direction));
    entries
        .into_iter()
        .skip(page.saturating_mul(ENTRIES_PER_PAGE))
        .take(ENTRIES_PER_PAGE)
        .collect()
}

fn compare_entries(
    left: &ArchiveEntryInfo,
    right: &ArchiveEntryInfo,
    sort: EntrySort,
    direction: SortDirection,
) -> Ordering {
    let directory_order = right.is_directory.cmp(&left.is_directory);
    if directory_order != Ordering::Equal {
        return directory_order;
    }
    let order = match sort {
        EntrySort::Name => left.path.cmp(&right.path),
        EntrySort::Size => left.size.cmp(&right.size),
        EntrySort::Packed => left.compressed_size.cmp(&right.compressed_size),
        EntrySort::Modified => match (left.modified, right.modified) {
            (Some(left), Some(right)) => left.cmp(&right),
            (Some(_), None) => return Ordering::Less,
            (None, Some(_)) => return Ordering::Greater,
            (None, None) => Ordering::Equal,
        },
    };
    let order = match direction {
        SortDirection::Ascending => order,
        SortDirection::Descending => order.reverse(),
    };
    order.then_with(|| left.path.cmp(&right.path))
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
                    modified: None,
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

    #[test]
    fn sorting_is_bounded_and_keeps_missing_modified_times_last() {
        let mut archive = large_archive();
        archive.entries[0].modified = Some(zifile_core::ArchiveTimestamp {
            year: 2024,
            month: 1,
            day: 2,
            hour: 3,
            minute: 4,
            second: 6,
            nanosecond: 0,
            offset: zifile_core::ArchiveTimestampOffset::Unspecified,
            precision: zifile_core::ArchiveTimestampPrecision::TwoSeconds,
        });
        let modified = sorted_filtered_entry_page(
            &archive,
            "",
            0,
            EntrySort::Modified,
            SortDirection::Descending,
        );
        assert_eq!(modified.len(), ENTRIES_PER_PAGE);
        assert_eq!(modified[0].path, PathBuf::from("folder/file-000000.txt"));
        let sizes =
            sorted_filtered_entry_page(&archive, "", 0, EntrySort::Size, SortDirection::Descending);
        assert_eq!(sizes.len(), ENTRIES_PER_PAGE);
        assert_eq!(
            next_sort(EntrySort::Name, SortDirection::Ascending, EntrySort::Name,),
            (EntrySort::Name, SortDirection::Descending)
        );
        assert_eq!(
            next_sort(
                EntrySort::Name,
                SortDirection::Descending,
                EntrySort::Modified,
            ),
            (EntrySort::Modified, SortDirection::Ascending)
        );
    }
}
