use std::hint::black_box;
use std::path::PathBuf;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use zifile_core::{ArchiveEntryInfo, ArchiveFormat, ArchiveInfo};
use zifile_desktop::entry_view::{filtered_entry_count, filtered_entry_page};

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
            })
            .collect(),
        total_size: 100_000,
        compressed_size: 100_000,
    }
}

fn entry_browser(criterion: &mut Criterion) {
    let archive = archive_with_100k_entries();
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
    group.finish();
}

criterion_group!(benches, entry_browser);
criterion_main!(benches);
