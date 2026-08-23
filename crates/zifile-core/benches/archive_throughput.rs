use std::fs;

use criterion::{Criterion, Throughput, criterion_group, criterion_main};
use zifile_core::{ArchiveFormat, CreateOptions, create_archive, test_archive};

fn archive_throughput(criterion: &mut Criterion) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("payload.bin");
    let payload = (0..8 * 1024 * 1024)
        .map(|index| ((index * 31) % 251) as u8)
        .collect::<Vec<_>>();
    fs::write(&source, &payload).unwrap();

    let mut create_group = criterion.benchmark_group("create archive");
    create_group.throughput(Throughput::Bytes(payload.len() as u64));
    create_group.sample_size(10);
    create_group.bench_function("zip deflate 8 MiB", |bencher| {
        bencher.iter(|| {
            let archive = temp.path().join("benchmark.zip");
            create_archive(
                std::slice::from_ref(&source),
                archive,
                ArchiveFormat::Zip,
                &CreateOptions::default(),
            )
            .unwrap();
        });
    });
    create_group.finish();

    let archive = temp.path().join("verify.zip");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::Zip,
        &CreateOptions::default(),
    )
    .unwrap();
    let mut read_group = criterion.benchmark_group("verify archive");
    read_group.throughput(Throughput::Bytes(payload.len() as u64));
    read_group.sample_size(10);
    read_group.bench_function("zip deflate 8 MiB", |bencher| {
        bencher.iter(|| test_archive(&archive, None).unwrap());
    });
    read_group.finish();
}

criterion_group!(benches, archive_throughput);
criterion_main!(benches);
