use std::collections::HashSet;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use tempfile::TempDir;
use zifile_core::{
    ArchiveFormat, ArchiveTimestampOffset, CancellationToken, ConflictPolicy, CreateOptions,
    ExtractOptions, ListOptions, OperationProgress, SafetyLimits, TestOptions, ZiFileError,
    create_archive, detect_format, extract_archive, list_archive, list_archive_with_limits,
    list_archive_with_options, test_archive, test_archive_with_limits, test_archive_with_options,
};

fn fixture() -> (TempDir, PathBuf) {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("input");
    fs::create_dir_all(source.join("nested")).unwrap();
    fs::write(source.join("hello.txt"), "hello ZiFile\n").unwrap();
    fs::write(source.join("nested/中文.txt"), "安全归档\n").unwrap();
    set_mtime(&source.join("hello.txt"), 1_700_000_000);
    set_mtime(&source.join("nested/中文.txt"), 1_700_000_002);
    set_mtime(&source.join("nested"), 1_700_000_004);
    set_mtime(&source, 1_700_000_006);
    (temp, source)
}

fn set_mtime(path: &Path, seconds: i64) {
    filetime::set_file_mtime(path, filetime::FileTime::from_unix_time(seconds, 0)).unwrap();
}

fn assert_mtime(path: &Path, expected: u64) {
    let actual = fs::metadata(path)
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(actual, expected, "unexpected mtime for {}", path.display());
}

fn assert_listed_mtime(path: &Path, entry_name: &str, offset: ArchiveTimestampOffset) {
    let info = list_archive(path, None).unwrap();
    let modified = info
        .entries
        .iter()
        .find(|entry| entry.path.ends_with(entry_name))
        .and_then(|entry| entry.modified)
        .unwrap();
    assert_eq!(
        (modified.year, modified.month, modified.day),
        (2023, 11, 14)
    );
    assert_eq!(
        (modified.hour, modified.minute, modified.second),
        (22, 13, 20)
    );
    assert_eq!(modified.offset, offset);
}

fn rar_fixture(path: &Path, password: Option<&str>) {
    let mut builder = rars::Builder::new(rars::ArchiveVersion::Rar50)
        .solid(true)
        .compression_level(Some(3));
    if let Some(password) = password {
        builder = builder
            .password(Some(password.as_bytes().to_vec()))
            .header_encryption(true);
    }
    builder
        .add_bytes(
            b"hello.txt".to_vec(),
            b"hello from RAR\n".to_vec(),
            None,
            None,
        )
        .unwrap();
    builder
        .add_bytes(
            "nested/中文.txt".as_bytes().to_vec(),
            "RAR 安全归档\n".as_bytes().to_vec(),
            None,
            None,
        )
        .unwrap();
    builder.write_to_path(path, None).unwrap();
}

fn rar_version_fixture(path: &Path, version: rars::ArchiveVersion) {
    let mut builder = rars::Builder::new(version)
        .solid(false)
        .compression_level(Some(0));
    builder
        .add_bytes(
            b"version.txt".to_vec(),
            format!("{version}\n").into_bytes(),
            None,
            None,
        )
        .unwrap();
    builder.write_to_path(path, None).unwrap();
}

fn cab_fixture(path: &Path, entries: &[(&str, &[u8])]) {
    cab_fixture_with_compression(path, entries, cab::CompressionType::MsZip);
}

fn cab_fixture_with_compression(
    path: &Path,
    entries: &[(&str, &[u8])],
    compression: cab::CompressionType,
) {
    let mut builder = cab::CabinetBuilder::new();
    let folder = builder.add_folder(compression);
    for (name, _) in entries {
        folder.add_file(*name);
    }
    let mut writer = builder.build(fs::File::create(path).unwrap()).unwrap();
    while let Some(mut file) = writer.next_file().unwrap() {
        let (_, contents) = entries
            .iter()
            .find(|(name, _)| *name == file.file_name())
            .unwrap();
        file.write_all(contents).unwrap();
    }
    writer.finish().unwrap();
}

fn assert_test_progress(path: &Path, password: Option<&str>) -> zifile_core::ArchiveInfo {
    assert_list_progress(path, password);
    let progress = OperationProgress::default();
    let info = test_archive_with_options(
        path,
        &TestOptions {
            password: password.map(str::to_owned),
            progress: progress.clone(),
            ..TestOptions::default()
        },
    )
    .unwrap();
    let snapshot = progress.snapshot();
    assert_eq!(
        snapshot.total_entries,
        info.entries
            .iter()
            .filter(|entry| !entry.is_directory)
            .count() as u64
    );
    assert_eq!(snapshot.processed_entries, snapshot.total_entries);
    assert_eq!(snapshot.total_bytes, info.total_size);
    assert_eq!(snapshot.processed_bytes, info.total_size);
    assert_eq!(snapshot.fraction(), 1.0);
    info
}

fn assert_list_progress(path: &Path, password: Option<&str>) -> zifile_core::ArchiveInfo {
    let progress = OperationProgress::default();
    let info = list_archive_with_options(
        path,
        &ListOptions {
            password: password.map(str::to_owned),
            progress: progress.clone(),
            ..ListOptions::default()
        },
    )
    .unwrap();
    let snapshot = progress.snapshot();
    assert_eq!(snapshot.total_entries, info.entries.len() as u64);
    assert_eq!(snapshot.processed_entries, snapshot.total_entries);
    assert_eq!(snapshot.processed_bytes, snapshot.total_bytes);
    assert_eq!(snapshot.fraction(), 1.0);
    info
}

#[test]
fn cab_uncompressed_content_is_supported() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("uncompressed.cab");
    cab_fixture_with_compression(
        &archive,
        &[("plain.txt", b"uncompressed cabinet")],
        cab::CompressionType::None,
    );
    assert_test_progress(&archive, None);
    let output = temp.path().join("cab-none");
    let summary = extract_archive(&archive, &output, &ExtractOptions::default()).unwrap();
    assert_eq!(summary.files, 1);
    assert_eq!(
        fs::read_to_string(output.join("plain.txt")).unwrap(),
        "uncompressed cabinet"
    );
}

#[test]
fn readonly_archives_restore_file_modified_times() {
    const EXPECTED: u64 = 1_700_000_000;
    const DOS_DATETIME: u32 = 0x576E_B1AA; // 2023-11-14 22:13:20
    let temp = tempfile::tempdir().unwrap();

    let rar = temp.path().join("dated.rar");
    let mut rar_builder = rars::Builder::new(rars::ArchiveVersion::Rar50);
    rar_builder
        .add_bytes(
            b"dated.txt".to_vec(),
            b"RAR timestamp".to_vec(),
            Some(DOS_DATETIME),
            None,
        )
        .unwrap();
    rar_builder.write_to_path(&rar, None).unwrap();
    let rar_output = temp.path().join("rar-dated");
    assert_listed_mtime(&rar, "dated.txt", ArchiveTimestampOffset::Unspecified);
    extract_archive(&rar, &rar_output, &ExtractOptions::default()).unwrap();
    assert_mtime(&rar_output.join("dated.txt"), EXPECTED);

    let cab = temp.path().join("dated.cab");
    let mut cab_builder = cab::CabinetBuilder::new();
    cab_builder
        .add_folder(cab::CompressionType::MsZip)
        .add_file("dated.txt")
        .set_datetime(
            time::Date::from_calendar_date(2023, time::Month::November, 14)
                .unwrap()
                .with_hms(22, 13, 20)
                .unwrap(),
        );
    let mut cab_writer = cab_builder.build(fs::File::create(&cab).unwrap()).unwrap();
    cab_writer
        .next_file()
        .unwrap()
        .unwrap()
        .write_all(b"CAB timestamp")
        .unwrap();
    cab_writer.finish().unwrap();
    let cab_output = temp.path().join("cab-dated");
    assert_listed_mtime(&cab, "dated.txt", ArchiveTimestampOffset::Unspecified);
    extract_archive(&cab, &cab_output, &ExtractOptions::default()).unwrap();
    assert_mtime(&cab_output.join("dated.txt"), EXPECTED);
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
    let timestamp_offset = if format == ArchiveFormat::Zip {
        ArchiveTimestampOffset::Unspecified
    } else {
        ArchiveTimestampOffset::Utc
    };
    assert_listed_mtime(&archive, "hello.txt", timestamp_offset);
    assert_test_progress(&archive, None);

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
    assert_mtime(&output.join("input/hello.txt"), 1_700_000_000);
    assert_mtime(&output.join("input/nested/中文.txt"), 1_700_000_002);
    assert_mtime(&output.join("input/nested"), 1_700_000_004);
    assert_mtime(&output.join("input"), 1_700_000_006);
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
        assert_test_progress(&archive, None);
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
fn encrypted_seven_zip_round_trip_reports_encryption_and_requires_password() {
    let temp = tempfile::tempdir().unwrap();
    let source = temp.path().join("secret.txt");
    fs::write(&source, "classified").unwrap();
    let archive = temp.path().join("secret.7z");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::SevenZip,
        &CreateOptions {
            password: Some("correct horse".to_owned()),
            ..CreateOptions::default()
        },
    )
    .unwrap();

    assert!(list_archive(&archive, None).is_err());
    assert!(list_archive(&archive, Some("wrong password")).is_err());
    let info = list_archive(&archive, Some("correct horse")).unwrap();
    assert_eq!(info.entries.len(), 1);
    assert!(info.entries[0].encrypted);
    assert!(test_archive(&archive, None).is_err());
    assert!(test_archive(&archive, Some("wrong password")).is_err());
    test_archive(&archive, Some("correct horse")).unwrap();

    let output = temp.path().join("sevenzip-encrypted");
    let summary = extract_archive(
        &archive,
        &output,
        &ExtractOptions {
            password: Some("correct horse".to_owned()),
            ..ExtractOptions::default()
        },
    )
    .unwrap();
    assert_eq!(summary.files, 1);
    assert_eq!(
        fs::read_to_string(output.join("secret.txt")).unwrap(),
        "classified"
    );
}

#[test]
fn rar_is_read_only_and_supports_solid_selected_extraction() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("fixture.rar");
    rar_fixture(&archive, None);

    let capabilities = ArchiveFormat::Rar.capabilities();
    assert!(capabilities.list);
    assert!(capabilities.extract);
    assert!(capabilities.encryption);
    assert!(!capabilities.create);

    let info = list_archive(&archive, None).unwrap();
    assert_eq!(info.format, ArchiveFormat::Rar);
    assert_eq!(info.entries.len(), 2);
    assert_eq!(info.total_size, 32);
    assert_test_progress(&archive, None);

    let output = temp.path().join("rar-selected");
    let selected = HashSet::from([PathBuf::from("nested/中文.txt")]);
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
    assert_eq!(summary.bytes, 17);
    assert!(!output.join("hello.txt").exists());
    assert_eq!(
        fs::read_to_string(output.join("nested/中文.txt")).unwrap(),
        "RAR 安全归档\n"
    );

    let source = temp.path().join("source.txt");
    fs::write(&source, "RAR creation remains disabled").unwrap();
    assert!(matches!(
        create_archive(
            &[source],
            temp.path().join("not-created.rar"),
            ArchiveFormat::Rar,
            &CreateOptions::default(),
        ),
        Err(ZiFileError::UnsupportedOperation(ArchiveFormat::Rar))
    ));
}

#[test]
fn cab_is_read_only_and_supports_safe_selected_extraction() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("fixture.cab");
    cab_fixture(
        &archive,
        &[
            ("hello.txt", b"hello from CAB\n"),
            ("nested\\unicode.txt", b"safe cabinet\n"),
        ],
    );

    let capabilities = ArchiveFormat::Cab.capabilities();
    assert!(capabilities.list);
    assert!(capabilities.extract);
    assert!(!capabilities.create);
    assert!(!capabilities.encryption);
    assert_eq!(detect_format(&archive).unwrap(), ArchiveFormat::Cab);

    let info = list_archive(&archive, None).unwrap();
    assert_eq!(info.format, ArchiveFormat::Cab);
    assert_eq!(info.entries.len(), 2);
    assert_eq!(info.total_size, 28);
    assert_test_progress(&archive, None);

    let output = temp.path().join("cab-selected");
    let selected = HashSet::from([PathBuf::from("nested/unicode.txt")]);
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
    assert_eq!(summary.bytes, 13);
    assert!(!output.join("hello.txt").exists());
    assert_eq!(
        fs::read_to_string(output.join("nested/unicode.txt")).unwrap(),
        "safe cabinet\n"
    );

    let source = temp.path().join("source.txt");
    fs::write(&source, "CAB creation remains disabled").unwrap();
    assert!(matches!(
        create_archive(
            &[source],
            temp.path().join("not-created.cab"),
            ArchiveFormat::Cab,
            &CreateOptions::default(),
        ),
        Err(ZiFileError::UnsupportedOperation(ArchiveFormat::Cab))
    ));
}

#[test]
fn cab_rejects_unsafe_paths_and_declared_limits() {
    let temp = tempfile::tempdir().unwrap();
    let unsafe_archive = temp.path().join("unsafe.cab");
    cab_fixture(&unsafe_archive, &[("..\\escape.txt", b"escape")]);
    assert!(matches!(
        list_archive(&unsafe_archive, None),
        Err(ZiFileError::UnsafePath(_))
    ));

    let archive = temp.path().join("limits.cab");
    cab_fixture(&archive, &[("one.txt", b"1"), ("two.txt", b"2")]);
    let limits = SafetyLimits {
        max_entries: 1,
        ..SafetyLimits::default()
    };
    assert!(matches!(
        list_archive_with_limits(&archive, None, limits),
        Err(ZiFileError::LimitExceeded(_))
    ));
}

#[test]
fn cab_rejects_multi_cabinet_sets_before_listing() {
    let temp = tempfile::tempdir().unwrap();
    let linked_archive = temp.path().join("linked.cab");
    cab_fixture(&linked_archive, &[("one.txt", b"one")]);

    let mut bytes = fs::read(&linked_archive).unwrap();
    bytes[30] |= 0x02;
    fs::write(&linked_archive, bytes).unwrap();

    assert!(matches!(
        list_archive(&linked_archive, None),
        Err(ZiFileError::InvalidInput(message))
            if message == "multi-cabinet sets are not supported"
    ));

    let indexed_archive = temp.path().join("indexed.cab");
    cab_fixture(&indexed_archive, &[("one.txt", b"one")]);
    let mut bytes = fs::read(&indexed_archive).unwrap();
    bytes[34..36].copy_from_slice(&1_u16.to_le_bytes());
    fs::write(&indexed_archive, bytes).unwrap();

    assert!(matches!(
        list_archive(&indexed_archive, None),
        Err(ZiFileError::InvalidInput(message))
            if message == "multi-cabinet sets are not supported"
    ));
}

#[test]
fn cab_corrupt_data_fails_integrity_test_without_committing_output() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("corrupt-data.cab");
    cab_fixture(&archive, &[("payload.bin", &vec![0x5a; 64 * 1024])]);

    let mut bytes = fs::read(&archive).unwrap();
    let folder_data_offset = u32::from_le_bytes(bytes[36..40].try_into().unwrap()) as usize;
    let first_data_byte = folder_data_offset + 8;
    assert!(first_data_byte < bytes.len(), "CAB fixture layout drifted");
    bytes[first_data_byte] ^= 0x40;
    fs::write(&archive, bytes).unwrap();

    let info = list_archive(&archive, None).unwrap();
    assert_eq!(info.entries.len(), 1);
    assert!(test_archive(&archive, None).is_err());

    let output = temp.path().join("corrupt-output");
    assert!(extract_archive(&archive, &output, &ExtractOptions::default()).is_err());
    assert!(!output.join("payload.bin").exists());
    assert_eq!(fs::read_dir(&output).unwrap().count(), 0);
}

#[test]
fn archive_testing_reports_progress_and_honors_precancellation() {
    let (temp, source) = fixture();
    let archive = temp.path().join("progress.zip");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::Zip,
        &CreateOptions::default(),
    )
    .unwrap();

    assert_test_progress(&archive, None);

    let cancellation = CancellationToken::default();
    cancellation.cancel();
    assert!(matches!(
        test_archive_with_options(
            &archive,
            &TestOptions {
                cancellation,
                ..TestOptions::default()
            },
        ),
        Err(ZiFileError::Cancelled)
    ));
}

#[test]
fn archive_listing_reports_progress_and_honors_precancellation() {
    let (temp, source) = fixture();
    let archive = temp.path().join("list-progress.zip");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::Zip,
        &CreateOptions::default(),
    )
    .unwrap();

    assert_list_progress(&archive, None);

    let cancellation = CancellationToken::default();
    cancellation.cancel();
    assert!(matches!(
        list_archive_with_options(
            &archive,
            &ListOptions {
                cancellation,
                ..ListOptions::default()
            },
        ),
        Err(ZiFileError::Cancelled)
    ));
}

#[test]
fn rar_reader_covers_every_supported_archive_version() {
    let temp = tempfile::tempdir().unwrap();
    for version in rars::ArchiveVersion::ALL {
        let archive = temp.path().join(format!("{version}.rar"));
        rar_version_fixture(&archive, version);
        let info = list_archive(&archive, None).unwrap();
        assert_eq!(info.entries.len(), 1, "failed to list {version}");
        assert_test_progress(&archive, None);
        let output = temp.path().join(format!("extract-{version}"));
        extract_archive(&archive, &output, &ExtractOptions::default()).unwrap();
        assert_eq!(
            fs::read_to_string(output.join("version.txt")).unwrap(),
            format!("{version}\n"),
            "failed to extract {version}"
        );
    }
}

#[test]
fn encrypted_rar_headers_require_the_correct_password() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("encrypted.rar");
    rar_fixture(&archive, Some("correct horse"));

    assert!(matches!(
        list_archive(&archive, None),
        Err(ZiFileError::PasswordRequired)
    ));
    assert!(list_archive(&archive, Some("wrong password")).is_err());
    assert_test_progress(&archive, Some("correct horse"));

    let output = temp.path().join("rar-encrypted");
    let summary = extract_archive(
        &archive,
        &output,
        &ExtractOptions {
            password: Some("correct horse".to_owned()),
            ..ExtractOptions::default()
        },
    )
    .unwrap();
    assert_eq!(summary.files, 2);
    assert_eq!(
        fs::read_to_string(output.join("hello.txt")).unwrap(),
        "hello from RAR\n"
    );
}

#[test]
fn rar_limits_and_cancellation_leave_no_output_file() {
    let temp = tempfile::tempdir().unwrap();
    let archive = temp.path().join("bounded.rar");
    rar_fixture(&archive, None);

    let limited_output = temp.path().join("rar-limited");
    let limited = extract_archive(
        &archive,
        &limited_output,
        &ExtractOptions {
            limits: SafetyLimits {
                max_expanded_bytes: 8,
                ..SafetyLimits::default()
            },
            ..ExtractOptions::default()
        },
    );
    assert!(matches!(limited, Err(ZiFileError::LimitExceeded(_))));
    assert!(!limited_output.join("hello.txt").exists());

    let cancellation = CancellationToken::default();
    cancellation.cancel();
    let cancelled_output = temp.path().join("rar-cancelled");
    let cancelled = extract_archive(
        &archive,
        &cancelled_output,
        &ExtractOptions {
            cancellation,
            ..ExtractOptions::default()
        },
    );
    assert!(matches!(cancelled, Err(ZiFileError::Cancelled)));
    assert!(!cancelled_output.join("hello.txt").exists());
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

#[test]
fn caller_entry_limits_apply_during_archive_listing() {
    for format in [
        ArchiveFormat::Zip,
        ArchiveFormat::SevenZip,
        ArchiveFormat::Tar,
    ] {
        let (temp, source) = fixture();
        let archive = temp
            .path()
            .join(format!("entry-limit.{}", format.canonical_extension()));
        create_archive(&[source], &archive, format, &CreateOptions::default()).unwrap();
        let result = list_archive_with_limits(
            &archive,
            None,
            SafetyLimits {
                max_entries: 1,
                ..SafetyLimits::default()
            },
        );
        assert!(
            matches!(result, Err(ZiFileError::LimitExceeded(_))),
            "listing limit was not enforced for {format}"
        );
    }
}

#[test]
fn extraction_uses_caller_limits_before_creating_destination() {
    let (temp, source) = fixture();
    let archive = temp.path().join("entry-limit.zip");
    create_archive(
        &[source],
        &archive,
        ArchiveFormat::Zip,
        &CreateOptions::default(),
    )
    .unwrap();
    let output = temp.path().join("must-not-exist");
    let result = extract_archive(
        &archive,
        &output,
        &ExtractOptions {
            limits: SafetyLimits {
                max_entries: 1,
                ..SafetyLimits::default()
            },
            ..ExtractOptions::default()
        },
    );
    assert!(matches!(result, Err(ZiFileError::LimitExceeded(_))));
    assert!(!output.exists());
}

#[test]
fn malformed_headers_for_every_supported_format_fail_without_panicking() {
    let temp = tempfile::tempdir().unwrap();
    let mut tar = vec![0_u8; 512];
    tar[257..262].copy_from_slice(b"ustar");
    let cases = [
        ("broken.zip", b"PK\x03\x04broken".to_vec()),
        ("broken.7z", b"7z\xBC\xAF\x27\x1Cbroken".to_vec()),
        ("broken.rar", b"Rar!\x1A\x07\x01\x00broken".to_vec()),
        ("broken.cab", b"MSCFbroken".to_vec()),
        ("broken.tar", tar),
        ("broken.tar.gz", b"\x1F\x8B\x08broken".to_vec()),
        ("broken.tar.zst", b"\x28\xB5\x2F\xFDbroken".to_vec()),
        ("broken.tar.xz", b"\xFD7zXZ\x00broken".to_vec()),
        ("broken.tar.bz2", b"BZhbroken".to_vec()),
        ("broken.gz", b"\x1F\x8B\x08broken".to_vec()),
        ("broken.zst", b"\x28\xB5\x2F\xFDbroken".to_vec()),
        ("broken.xz", b"\xFD7zXZ\x00broken".to_vec()),
        ("broken.bz2", b"BZhbroken".to_vec()),
        ("broken.lz4", b"\x04\x22\x4D\x18broken".to_vec()),
        ("broken.br", b"\xFFbroken".to_vec()),
    ];
    let limits = SafetyLimits {
        max_entries: 16,
        max_expanded_bytes: 1024 * 1024,
        max_expansion_ratio: 32,
        max_path_depth: 16,
    };
    for (name, bytes) in cases {
        let path = temp.path().join(name);
        fs::write(&path, bytes).unwrap();
        assert!(
            list_archive_with_limits(&path, None, limits).is_err(),
            "malformed input unexpectedly listed: {name}"
        );
        assert!(
            test_archive_with_limits(&path, None, limits).is_err(),
            "malformed input unexpectedly passed: {name}"
        );
    }
}

#[test]
fn fuzz_discovered_seven_zip_oversized_allocations_are_rejected() {
    let fixtures = [
        (
            "capacity-overflow",
            292,
            include_str!("../../../tests/fixtures/sevenz-capacity-overflow.hex"),
        ),
        (
            "oversized-allocation",
            173,
            include_str!("../../../tests/fixtures/sevenz-oversized-allocation.hex"),
        ),
    ];
    let temp = tempfile::tempdir().unwrap();
    for (name, expected_len, fixture) in fixtures {
        let fuzz_input = fixture
            .trim()
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let digits = std::str::from_utf8(pair).unwrap();
                u8::from_str_radix(digits, 16).unwrap()
            })
            .collect::<Vec<_>>();
        assert_eq!(fuzz_input.len(), expected_len, "{name} fixture drifted");
        assert_eq!(
            usize::from(fuzz_input[0]) % 13,
            1,
            "fixture must select the 7z fuzz case"
        );
        let mut bytes = b"7z\xBC\xAF\x27\x1C".to_vec();
        bytes.extend_from_slice(&fuzz_input[1..]);
        let archive = temp.path().join(format!("fuzz-{name}.7z"));
        fs::write(&archive, bytes).unwrap();

        let result = list_archive_with_limits(&archive, None, SafetyLimits::default());

        assert!(
            matches!(result, Err(ZiFileError::SevenZip(_))),
            "upstream parser did not reject {name} before ZiFile's panic fallback: {result:?}"
        );
    }
}
