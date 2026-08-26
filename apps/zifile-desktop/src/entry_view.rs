use std::borrow::Cow;
use std::cmp::Ordering;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

use zifile_core::{ArchiveEntryInfo, ArchiveInfo, ArchiveTimestamp};

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BrowserEntry<'a> {
    pub path: Cow<'a, Path>,
    pub size: u64,
    pub compressed_size: u64,
    pub is_directory: bool,
    pub encrypted: bool,
    pub modified: Option<ArchiveTimestamp>,
}

impl BrowserEntry<'_> {
    pub fn into_owned(self) -> BrowserEntry<'static> {
        BrowserEntry {
            path: Cow::Owned(self.path.into_owned()),
            size: self.size,
            compressed_size: self.compressed_size,
            is_directory: self.is_directory,
            encrypted: self.encrypted,
            modified: self.modified,
        }
    }
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

pub fn browser_entry_count(archive: &ArchiveInfo, directory: &Path, filter: &str) -> usize {
    browser_entries(archive, directory, filter).len()
}

pub fn directory_breadcrumbs(directory: &Path) -> Vec<(String, PathBuf)> {
    let mut path = PathBuf::new();
    directory
        .components()
        .map(|component| {
            path.push(component.as_os_str());
            (
                component.as_os_str().to_string_lossy().into_owned(),
                path.clone(),
            )
        })
        .collect()
}

pub fn browser_entry_page<'a>(
    archive: &'a ArchiveInfo,
    directory: &Path,
    filter: &str,
    page: usize,
    sort: EntrySort,
    direction: SortDirection,
) -> Vec<BrowserEntry<'a>> {
    let mut entries = browser_entries(archive, directory, filter);
    entries.sort_unstable_by(|left, right| compare_browser_entries(left, right, sort, direction));
    entries
        .into_iter()
        .skip(page.saturating_mul(ENTRIES_PER_PAGE))
        .take(ENTRIES_PER_PAGE)
        .collect()
}

fn browser_entries<'a>(
    archive: &'a ArchiveInfo,
    directory: &Path,
    filter: &str,
) -> Vec<BrowserEntry<'a>> {
    let filter_lower = filter.to_lowercase();
    if !filter_lower.is_empty() {
        return archive
            .entries
            .iter()
            .filter(|entry| entry_matches_filter(entry, &filter_lower))
            .map(real_browser_entry)
            .collect();
    }

    let mut files = Vec::new();
    let mut directories: HashMap<PathBuf, Option<&ArchiveEntryInfo>> = HashMap::new();
    for entry in &archive.entries {
        let Ok(relative) = entry.path.strip_prefix(directory) else {
            continue;
        };
        let mut components = relative.components();
        let Some(first) = components.next() else {
            continue;
        };
        let child_path = directory.join(first.as_os_str());
        if components.next().is_some() {
            directories.entry(child_path).or_insert(None);
        } else if entry.is_directory {
            directories.insert(child_path, Some(entry));
        } else {
            files.push(real_browser_entry(entry));
        }
    }
    files.extend(directories.into_iter().map(|(path, explicit)| {
        explicit.map_or_else(
            || BrowserEntry {
                path: Cow::Owned(path),
                size: 0,
                compressed_size: 0,
                is_directory: true,
                encrypted: false,
                modified: None,
            },
            real_browser_entry,
        )
    }));
    files
}

fn real_browser_entry(entry: &ArchiveEntryInfo) -> BrowserEntry<'_> {
    BrowserEntry {
        path: Cow::Borrowed(&entry.path),
        size: entry.size,
        compressed_size: entry.compressed_size,
        is_directory: entry.is_directory,
        encrypted: entry.encrypted,
        modified: entry.modified,
    }
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

fn compare_browser_entries(
    left: &BrowserEntry<'_>,
    right: &BrowserEntry<'_>,
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

    #[test]
    fn folder_browser_synthesizes_directories_and_searches_globally() {
        let archive = ArchiveInfo {
            path: PathBuf::from("folders.zip"),
            format: ArchiveFormat::Zip,
            entries: vec![
                ArchiveEntryInfo {
                    path: PathBuf::from("root.txt"),
                    size: 4,
                    compressed_size: 3,
                    is_directory: false,
                    encrypted: false,
                    modified: None,
                },
                ArchiveEntryInfo {
                    path: PathBuf::from("docs/readme.txt"),
                    size: 8,
                    compressed_size: 6,
                    is_directory: false,
                    encrypted: false,
                    modified: None,
                },
                ArchiveEntryInfo {
                    path: PathBuf::from("docs/reference/api.txt"),
                    size: 12,
                    compressed_size: 9,
                    is_directory: false,
                    encrypted: false,
                    modified: None,
                },
            ],
            total_size: 24,
            compressed_size: 18,
        };

        let root = browser_entry_page(
            &archive,
            Path::new(""),
            "",
            0,
            EntrySort::Name,
            SortDirection::Ascending,
        );
        assert_eq!(root.len(), 2);
        assert_eq!(root[0].path.as_ref(), Path::new("docs"));
        assert!(root[0].is_directory);
        assert_eq!(root[1].path.as_ref(), Path::new("root.txt"));

        let docs = browser_entry_page(
            &archive,
            Path::new("docs"),
            "",
            0,
            EntrySort::Name,
            SortDirection::Ascending,
        );
        assert_eq!(docs.len(), 2);
        assert_eq!(docs[0].path.as_ref(), Path::new("docs/reference"));
        assert!(docs[0].is_directory);
        assert_eq!(docs[1].path.as_ref(), Path::new("docs/readme.txt"));

        let search = browser_entry_page(
            &archive,
            Path::new("docs/reference"),
            "readme",
            0,
            EntrySort::Name,
            SortDirection::Ascending,
        );
        assert_eq!(search.len(), 1);
        assert_eq!(search[0].path.as_ref(), Path::new("docs/readme.txt"));
        assert_eq!(browser_entry_count(&archive, Path::new("docs"), ""), 2);
    }

    #[test]
    fn folder_browser_keeps_large_directories_bounded() {
        let archive = large_archive();
        let root = browser_entry_page(
            &archive,
            Path::new(""),
            "",
            0,
            EntrySort::Name,
            SortDirection::Ascending,
        );
        assert_eq!(root.len(), 1);
        assert_eq!(root[0].path.as_ref(), Path::new("folder"));

        let folder = browser_entry_page(
            &archive,
            Path::new("folder"),
            "",
            0,
            EntrySort::Name,
            SortDirection::Descending,
        );
        assert_eq!(folder.len(), ENTRIES_PER_PAGE);
        assert_eq!(folder[0].path.as_ref(), Path::new("folder/file-099999.txt"));
    }

    #[test]
    fn breadcrumbs_preserve_each_navigable_parent() {
        assert_eq!(
            directory_breadcrumbs(Path::new("docs/reference/api")),
            vec![
                ("docs".to_owned(), PathBuf::from("docs")),
                ("reference".to_owned(), PathBuf::from("docs/reference")),
                ("api".to_owned(), PathBuf::from("docs/reference/api")),
            ]
        );
        assert!(directory_breadcrumbs(Path::new("")).is_empty());
    }
}
