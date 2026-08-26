use std::collections::HashSet;
use std::hint::black_box;
use std::path::{Path, PathBuf};

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use zifile_core::{ArchiveEntryInfo, ArchiveFormat, ArchiveInfo};
use zifile_desktop::entry_view::{
    EntrySort, SortDirection, browser_entry_page, child_directory_selections, filtered_entry_count,
    filtered_entry_page, sorted_filtered_entry_page,
};

fn archive_with_100k_entries() -> ArchiveInfo {
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

fn entry_browser(criterion: &mut Criterion) {
    let archive = archive_with_100k_entries();
    let selected = archive
        .entries
        .iter()
        .step_by(2)
        .map(|entry| entry.path.clone())
        .collect::<HashSet<_>>();
    let mut group = criterion.benchmark_group("entry browser 100k");
    group.throughput(Throughput::Elements(archive.entries.len() as u64));
    group.sample_size(20);
    group.bench_function("count selective filter", |bencher| {
        bencher.iter(|| {
            let count = filtered_entry_count(black_box(&archive), black_box("file-09"));
            assert_eq!(count, 10_000);
        });
    });
    group.bench_function("collect bounded page", |bencher| {
        bencher.iter(|| {
            let page = filtered_entry_page(black_box(&archive), black_box("file-09"), 4);
            assert_eq!(page.len(), 500);
            black_box(page);
        });
    });
    group.bench_function("sort all name descending bounded page", |bencher| {
        bencher.iter(|| {
            let page = sorted_filtered_entry_page(
                black_box(&archive),
                black_box(""),
                0,
                EntrySort::Name,
                SortDirection::Descending,
            );
            assert_eq!(page.len(), 500);
            assert!(page[0].path.ends_with("file-099999.txt"));
            black_box(page);
        });
    });
    group.bench_function("synthesize implicit root folder", |bencher| {
        bencher.iter(|| {
            let page = browser_entry_page(
                black_box(&archive),
                black_box(Path::new("")),
                black_box(""),
                0,
                EntrySort::Name,
                SortDirection::Ascending,
            );
            assert_eq!(page.len(), 1);
            assert_eq!(page[0].path.as_ref(), Path::new("folder"));
            black_box(page);
        });
    });
    group.bench_function("sort folder name descending bounded page", |bencher| {
        bencher.iter(|| {
            let page = browser_entry_page(
                black_box(&archive),
                black_box(Path::new("folder")),
                black_box(""),
                0,
                EntrySort::Name,
                SortDirection::Descending,
            );
            assert_eq!(page.len(), 500);
            assert!(page[0].path.ends_with("file-099999.txt"));
            black_box(page);
        });
    });
    group.bench_function("aggregate root folder selection", |bencher| {
        bencher.iter(|| {
            let counts = child_directory_selections(
                black_box(&archive),
                black_box(Path::new("")),
                black_box(&selected),
            );
            assert_eq!(counts[Path::new("folder")].selected, 50_000);
            assert_eq!(counts[Path::new("folder")].total, 100_000);
            black_box(counts);
        });
    });
    group.finish();
}

criterion_group!(benches, entry_browser);
criterion_main!(benches);
