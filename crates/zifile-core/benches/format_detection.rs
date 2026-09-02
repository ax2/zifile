use criterion::{Criterion, criterion_group, criterion_main};
use zifile_core::detect_format_from_path;

fn detect_extensions(criterion: &mut Criterion) {
    let samples = [
        "release.zip",
        "source.tar.gz",
        "backup.tar.zst",
        "compressed.tar.lzma",
        "dataset.7z",
        "legacy.rar",
    ];

    criterion.bench_function("detect common archive extensions", |bencher| {
        bencher.iter(|| {
            for sample in samples {
                std::hint::black_box(detect_format_from_path(sample));
            }
        });
    });
}

criterion_group!(benches, detect_extensions);
criterion_main!(benches);
