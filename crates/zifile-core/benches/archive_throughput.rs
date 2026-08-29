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

    let mut tar_payload = Vec::with_capacity(1024 * 1024);
    let mut tar_noise = 0x9E37_79B9_u32;
    for _ in 0..1024 * 1024 {
        tar_noise ^= tar_noise << 13;
        tar_noise ^= tar_noise >> 17;
        tar_noise ^= tar_noise << 5;
        tar_payload.push(tar_noise as u8);
    }
    let tar_source = temp.path().join("tar-payload.bin");
    fs::write(&tar_source, &tar_payload).unwrap();
    let mut tar_create_group = criterion.benchmark_group("create tar archive");
    tar_create_group.throughput(Throughput::Bytes(tar_payload.len() as u64));
    tar_create_group.sample_size(10);
    tar_create_group.bench_function("tar lzma 1 MiB", |bencher| {
        bencher.iter(|| {
            let archive = temp.path().join("benchmark.tar.lzma");
            create_archive(
                std::slice::from_ref(&tar_source),
                archive,
                ArchiveFormat::TarLzma,
                &CreateOptions::default(),
            )
            .unwrap();
        });
    });
    tar_create_group.finish();

    let archive = temp.path().join("verify.zip");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::Zip,
        &CreateOptions::default(),
    )
    .unwrap();
    let rar_archive = temp.path().join("verify.rar");
    let mut rar_payload = payload.clone();
    let mut noise = 0x9E37_79B9_u32;
    for byte in rar_payload.iter_mut().step_by(64) {
        noise ^= noise << 13;
        noise ^= noise >> 17;
        noise ^= noise << 5;
        *byte = noise as u8;
    }
    let mut rar_builder = rars::Builder::new(rars::ArchiveVersion::Rar50)
        .solid(false)
        .compression_level(Some(3));
    rar_builder
        .add_bytes(b"payload.bin".to_vec(), rar_payload, None, None)
        .unwrap();
    rar_builder.write_to_path(&rar_archive, None).unwrap();
    let mut read_group = criterion.benchmark_group("verify archive");
    read_group.throughput(Throughput::Bytes(payload.len() as u64));
    read_group.sample_size(10);
    read_group.bench_function("zip deflate 8 MiB", |bencher| {
        bencher.iter(|| test_archive(&archive, None).unwrap());
    });
    read_group.bench_function("rar5 method 3 8 MiB", |bencher| {
        bencher.iter(|| test_archive(&rar_archive, None).unwrap());
    });
    read_group.finish();
    let tar_archive = temp.path().join("verify.tar.lzma");
    create_archive(
        &[tar_source],
        &tar_archive,
        ArchiveFormat::TarLzma,
        &CreateOptions::default(),
    )
    .unwrap();
    let mut tar_read_group = criterion.benchmark_group("verify tar archive");
    tar_read_group.throughput(Throughput::Bytes(tar_payload.len() as u64));
    tar_read_group.sample_size(10);
    tar_read_group.bench_function("tar lzma 1 MiB", |bencher| {
        bencher.iter(|| test_archive(&tar_archive, None).unwrap());
    });
    tar_read_group.finish();
}

criterion_group!(benches, archive_throughput);
criterion_main!(benches);
