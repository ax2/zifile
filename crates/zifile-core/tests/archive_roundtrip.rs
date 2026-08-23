use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use tempfile::TempDir;
use zifile_core::{
    ArchiveFormat, CancellationToken, ConflictPolicy, CreateOptions, ExtractOptions,
    OperationProgress, SafetyLimits, ZiFileError, create_archive, detect_format, extract_archive,
    list_archive, test_archive,
};

fn fixture() -> (TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("input");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("hello.txt"), "hello ZiFile\n").unwrap();
    fs::write(source.join("nested/中文.txt"), "安全归档\n").unwrap();
    (temp, source)
}

fn assert_round_trip(format: ArchiveFormat) {
    let (temp, source) = fixture();
    let archive = temp
        .path()
        .join(format!("roundtrip.{}", format.canonical_extension()));
    let create_progress = OperationProgress::default();
    let created = create_archive(
        std::slice::from_ref(&source),
        &archive,
        format,
        &CreateOptions {
            progress: create_progress.clone(),
            ..CreateOptions::default()
        },
    )
    .unwrap();
    assert_eq!(created.files, 2);
    assert_eq!(create_progress.snapshot().fraction(), 1.0);

    let info = list_archive(&archive, None).unwrap();
    assert_eq!(info.format, format);
    assert!(
        info.entries
            .iter()
            .any(|entry| entry.path.ends_with("hello.txt"))
    );
    test_archive(&archive, None).unwrap();

    let output = temp.path().join("output");
    let extract_progress = OperationProgress::default();
    let extracted = extract_archive(
        &archive,
        &output,
        &ExtractOptions {
            conflict: ConflictPolicy::Error,
            progress: extract_progress.clone(),
            ..ExtractOptions::default()
        },
    )
    .unwrap();
    assert_eq!(extracted.files, 2);
    assert_eq!(extract_progress.snapshot().fraction(), 1.0);
    assert_eq!(
        fs::read_to_string(output.join("input/hello.txt")).unwrap(),
        "hello ZiFile\n"
    );
    assert_eq!(
        fs::read_to_string(output.join("input/nested/中文.txt")).unwrap(),
        "安全归档\n"
    );
}

#[test]
fn zip_round_trip() {
    assert_round_trip(ArchiveFormat::Zip);
}

#[test]
fn seven_zip_round_trip() {
    assert_round_trip(ArchiveFormat::SevenZip);
}

#[test]
fn tar_family_round_trips() {
    for format in [
        ArchiveFormat::Tar,
        ArchiveFormat::TarGzip,
        ArchiveFormat::TarZstd,
        ArchiveFormat::TarXz,
        ArchiveFormat::TarBzip2,
    ] {
        assert_round_trip(format);
    }
}

#[test]
fn stream_formats_round_trip_single_files() {
    for format in [
        ArchiveFormat::Gzip,
        ArchiveFormat::Zstandard,
        ArchiveFormat::Xz,
        ArchiveFormat::Bzip2,
        ArchiveFormat::Lz4,
        ArchiveFormat::Brotli,
    ] {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("payload.txt");
        fs::write(&source, "stream payload\n").unwrap();
        let archive = temp
            .path()
            .join(format!("payload.txt.{}", format.canonical_extension()));
        create_archive(
            std::slice::from_ref(&source),
            &archive,
            format,
            &CreateOptions::default(),
        )
        .unwrap();
        test_archive(&archive, None).unwrap();
        let output = temp.path().join("output");
        extract_archive(&archive, &output, &ExtractOptions::default()).unwrap();
        assert_eq!(
            fs::read_to_string(output.join("payload.txt")).unwrap(),
            "stream payload\n",
            "failed {format}"
        );
    }
}

#[test]
fn encrypted_zip_round_trip_requires_password() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("secret.txt");
    fs::write(&source, "classified").unwrap();
    let archive = temp.path().join("secret.zip");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::Zip,
        &CreateOptions {
            password: Some("correct horse".to_owned()),
            ..CreateOptions::default()
        },
    )
    .unwrap();
    assert!(matches!(
        test_archive(&archive, None),
        Err(ZiFileError::PasswordRequired)
    ));
    test_archive(&archive, Some("correct horse")).unwrap();
}

#[test]
fn traversal_zip_is_rejected_before_extraction() {
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("evil.zip");
    let file = fs::File::create(&archive_path).unwrap();
    let mut writer = zip::ZipWriter::new(file);
    writer
        .start_file("../escape.txt", zip::write::SimpleFileOptions::default())
        .unwrap();
    writer.write_all(b"nope").unwrap();
    writer.finish().unwrap();

    assert!(matches!(
        list_archive(&archive_path, None),
        Err(ZiFileError::UnsafePath(_))
    ));
    assert!(!Path::new(temp.path()).join("escape.txt").exists());
}

#[test]
fn existing_destination_obeys_conflict_policy() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("payload.txt");
    fs::write(&source, "new").unwrap();
    let archive = temp.path().join("payload.zip");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::Zip,
        &CreateOptions::default(),
    )
    .unwrap();
    let output = temp.path().join("output");
    fs::create_dir_all(&output).unwrap();
    fs::write(output.join("payload.txt"), "old").unwrap();

    assert!(matches!(
        extract_archive(&archive, &output, &ExtractOptions::default()),
        Err(ZiFileError::DestinationExists(_))
    ));
    let skipped = extract_archive(
        &archive,
        &output,
        &ExtractOptions {
            conflict: ConflictPolicy::Skip,
            ..ExtractOptions::default()
        },
    )
    .unwrap();
    assert_eq!(skipped.skipped, 1);
    assert_eq!(
        fs::read_to_string(output.join("payload.txt")).unwrap(),
        "old"
    );
}

#[test]
fn selected_extraction_writes_only_selected_files() {
    let (temp, source) = fixture();
    let archive = temp.path().join("selected.zip");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::Zip,
        &CreateOptions::default(),
    )
    .unwrap();
    let output = temp.path().join("selected-output");
    let selected = HashSet::from([PathBuf::from("input/hello.txt")]);
    let summary = extract_archive(
        &archive,
        &output,
        &ExtractOptions {
            selected_paths: Some(selected),
            ..ExtractOptions::default()
        },
    )
    .unwrap();
    assert_eq!(summary.files, 1);
    assert!(output.join("input/hello.txt").is_file());
    assert!(!output.join("input/nested/中文.txt").exists());
}

#[test]
fn declared_expansion_limit_blocks_output() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("large.txt");
    fs::write(&source, vec![b'a'; 4096]).unwrap();
    let archive = temp.path().join("limited.zip");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::Zip,
        &CreateOptions::default(),
    )
    .unwrap();
    let output = temp.path().join("limited-output");
    let result = extract_archive(
        &archive,
        &output,
        &ExtractOptions {
            limits: SafetyLimits {
                max_expanded_bytes: 32,
                ..SafetyLimits::default()
            },
            ..ExtractOptions::default()
        },
    );
    assert!(matches!(result, Err(ZiFileError::LimitExceeded(_))));
    assert!(!output.join("large.txt").exists());
}

#[test]
fn tar_symbolic_link_is_rejected() {
    let temp = tempfile::tempdir().unwrap();
    let archive_path = temp.path().join("link.tar");
    let file = fs::File::create(&archive_path).unwrap();
    let mut archive = tar::Builder::new(file);
    let mut header = tar::Header::new_gnu();
    header.set_entry_type(tar::EntryType::Symlink);
    header.set_size(0);
    header.set_mode(0o777);
    header.set_cksum();
    archive
        .append_link(&mut header, "link", "../../outside")
        .unwrap();
    archive.finish().unwrap();
    assert!(matches!(
        list_archive(&archive_path, None),
        Err(ZiFileError::LinkEntry(_))
    ));
}

#[test]
fn signatures_win_over_misleading_extensions() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("payload.txt");
    fs::write(&source, "signature").unwrap();
    let archive = temp.path().join("actually-a-zip.bin");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::Zip,
        &CreateOptions::default(),
    )
    .unwrap();
    assert_eq!(detect_format(&archive).unwrap(), ArchiveFormat::Zip);
}

#[test]
fn cancellation_stops_before_writing_output() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("cancel.txt");
    fs::write(&source, vec![b'x'; 1024]).unwrap();
    let archive = temp.path().join("cancel.zip");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::Zip,
        &CreateOptions::default(),
    )
    .unwrap();

    let cancellation = CancellationToken::default();
    cancellation.cancel();
    let output = temp.path().join("cancel-output");
    let result = extract_archive(
        &archive,
        &output,
        &ExtractOptions {
            cancellation,
            ..ExtractOptions::default()
        },
    );
    assert!(matches!(result, Err(ZiFileError::Cancelled)));
    assert!(!output.join("cancel.txt").exists());
}
