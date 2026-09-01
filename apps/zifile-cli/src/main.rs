#![cfg_attr(
    not(test),
    deny(
        clippy::unwrap_used,
        clippy::expect_used,
        clippy::panic,
        clippy::indexing_slicing,
        clippy::undocumented_unsafe_blocks
    )
)]

use std::{
    io::{self, BufRead},
    path::PathBuf,
};

use clap::{Parser, Subcommand, ValueEnum};
use zifile_core::{
    ArchiveFormat, ArchiveRename, ConflictPolicy, CreateInputKind, CreateOptions, ExtractOptions,
    UpdateOptions, create_archive, detect_format, detect_format_from_path, extract_archive,
    list_archive, rename_archive, test_archive, update_archive,
};

const RUNTIME_ERROR_EXIT_CODE: i32 = 1;

#[derive(Debug, Parser)]
#[command(
    name = "zifile",
    version,
    about = "Fast, safe archive tools for Windows"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Show the implemented format capability matrix.
    Formats,
    /// Detect an existing archive from its signature.
    Detect { path: PathBuf },
    /// List archive contents.
    List {
        archive: PathBuf,
        /// Read the archive password from one line of standard input.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Verify every archive entry without extracting it.
    Test {
        archive: PathBuf,
        /// Read the archive password from one line of standard input.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Safely extract an archive.
    Extract {
        archive: PathBuf,
        destination: PathBuf,
        #[arg(long, value_enum, default_value_t = ConflictArg::Error)]
        conflict: ConflictArg,
        /// Read the archive password from one line of standard input.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Create an archive from one or more files/directories.
    Create {
        destination: PathBuf,
        #[arg(required = true)]
        sources: Vec<PathBuf>,
        #[arg(long, value_enum)]
        format: Option<FormatArg>,
        /// Compression level; defaults to 6 for adjustable formats (see `zifile formats`).
        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=22))]
        level: Option<u8>,
        /// Read the archive password from one line of standard input.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Add files or directories to an existing multi-entry archive.
    Update {
        archive: PathBuf,
        /// Files or directories to add. May be omitted when --remove is used.
        additions: Vec<PathBuf>,
        /// Archive-relative files or directories to remove; may be repeated.
        #[arg(long = "remove", value_name = "ARCHIVE_PATH")]
        remove_paths: Vec<PathBuf>,
        /// Compression level; defaults to 6 for adjustable formats.
        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=22))]
        level: Option<u8>,
        /// Read the archive password from one line of standard input.
        #[arg(long)]
        password_stdin: bool,
    },
    /// Rename files or directories inside an existing multi-entry archive.
    Rename {
        archive: PathBuf,
        /// Archive-relative rename mapping; may be repeated.
        #[arg(long = "rename", value_name = "FROM=TO", required = true)]
        renames: Vec<String>,
        /// Compression level; defaults to 6 for adjustable formats.
        #[arg(long, value_parser = clap::value_parser!(u8).range(0..=22))]
        level: Option<u8>,
        /// Read the archive password from one line of standard input.
        #[arg(long)]
        password_stdin: bool,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum ConflictArg {
    Overwrite,
    Skip,
    Rename,
    Error,
}

impl From<ConflictArg> for ConflictPolicy {
    fn from(value: ConflictArg) -> Self {
        match value {
            ConflictArg::Overwrite => Self::Overwrite,
            ConflictArg::Skip => Self::Skip,
            ConflictArg::Rename => Self::Rename,
            ConflictArg::Error => Self::Error,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum FormatArg {
    Zip,
    SevenZip,
    Tar,
    TarGzip,
    TarZstd,
    TarXz,
    TarLzma,
    TarBzip2,
    TarLz4,
    Gzip,
    Zstandard,
    Xz,
    Lzma,
    Bzip2,
    Lz4,
    Brotli,
    Cab,
}

impl From<FormatArg> for ArchiveFormat {
    fn from(value: FormatArg) -> Self {
        match value {
            FormatArg::Zip => Self::Zip,
            FormatArg::SevenZip => Self::SevenZip,
            FormatArg::Tar => Self::Tar,
            FormatArg::TarGzip => Self::TarGzip,
            FormatArg::TarZstd => Self::TarZstd,
            FormatArg::TarXz => Self::TarXz,
            FormatArg::TarLzma => Self::TarLzma,
            FormatArg::TarBzip2 => Self::TarBzip2,
            FormatArg::TarLz4 => Self::TarLz4,
            FormatArg::Gzip => Self::Gzip,
            FormatArg::Zstandard => Self::Zstandard,
            FormatArg::Xz => Self::Xz,
            FormatArg::Lzma => Self::Lzma,
            FormatArg::Bzip2 => Self::Bzip2,
            FormatArg::Lz4 => Self::Lz4,
            FormatArg::Brotli => Self::Brotli,
            FormatArg::Cab => Self::Cab,
        }
    }
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("error: {error}");
        std::process::exit(RUNTIME_ERROR_EXIT_CODE);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Formats => print_formats(),
        Command::Detect { path } => {
            let format = detect_format(&path)?;
            println!("{format}\t{}", format.canonical_extension());
        }
        Command::List {
            archive,
            password_stdin,
        } => {
            let password = read_password(password_stdin)?;
            let info = list_archive(&archive, password.as_deref())?;
            println!(
                "{}\t{} entries\t{} bytes expanded\t{} bytes archive",
                info.format,
                info.entries.len(),
                info.total_size,
                info.compressed_size
            );
            println!("TYPE\tSIZE\tCOMPRESSED\tENCRYPTED\tPATH");
            for entry in info.entries {
                println!(
                    "{}\t{}\t{}\t{}\t{}",
                    if entry.is_directory { "dir" } else { "file" },
                    entry.size,
                    entry.compressed_size,
                    yes_no(entry.encrypted),
                    entry.path.display()
                );
            }
        }
        Command::Test {
            archive,
            password_stdin,
        } => {
            let password = read_password(password_stdin)?;
            let info = test_archive(&archive, password.as_deref())?;
            println!(
                "OK: {} entries, {} expanded bytes, {} SHA-256 checksums",
                info.entries.len(),
                info.total_size,
                info.entries
                    .iter()
                    .filter(|entry| entry.checksum.is_some())
                    .count()
            );
            println!("SHA256\tPATH");
            for entry in info.entries {
                if let Some(checksum) = entry.checksum {
                    println!("{checksum}\t{}", entry.path.display());
                }
            }
        }
        Command::Extract {
            archive,
            destination,
            conflict,
            password_stdin,
        } => {
            let password = read_password(password_stdin)?;
            let summary = extract_archive(
                archive,
                destination,
                &ExtractOptions {
                    conflict: conflict.into(),
                    password,
                    ..ExtractOptions::default()
                },
            )?;
            println!(
                "Extracted {} files and {} directories ({} bytes); skipped {}",
                summary.files, summary.directories, summary.bytes, summary.skipped
            );
        }
        Command::Create {
            destination,
            sources,
            format,
            level,
            password_stdin,
        } => {
            let password = read_password(password_stdin)?;
            let format = format
                .map(ArchiveFormat::from)
                .or_else(|| detect_format_from_path(&destination))
                .ok_or("cannot infer output format; pass --format")?;
            let compression_level = resolve_compression_level(format, level)?;
            let summary = create_archive(
                &sources,
                destination,
                format,
                &CreateOptions {
                    compression_level,
                    password,
                    ..CreateOptions::default()
                },
            )?;
            println!(
                "Created {format} from {} files and {} directories ({} input bytes)",
                summary.files, summary.directories, summary.bytes
            );
        }
        Command::Update {
            archive,
            additions,
            remove_paths,
            level,
            password_stdin,
        } => {
            if additions.is_empty() && remove_paths.is_empty() {
                return Err("update requires at least one addition or --remove path".into());
            }
            let password = read_password(password_stdin)?;
            let format = detect_format(&archive)?;
            if !format.supports_update() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{format} archives cannot be updated"),
                )
                .into());
            }
            let compression_level = resolve_compression_level(format, level)?;
            let summary = update_archive(
                archive,
                &additions,
                &UpdateOptions {
                    compression_level,
                    password,
                    remove_paths,
                    ..UpdateOptions::default()
                },
            )?;
            println!(
                "Updated {format} with {} files and {} directories ({} total input bytes)",
                summary.files, summary.directories, summary.bytes
            );
        }
        Command::Rename {
            archive,
            renames,
            level,
            password_stdin,
        } => {
            let renames = renames
                .iter()
                .map(|spec| parse_rename_spec(spec))
                .collect::<Result<Vec<_>, _>>()?;
            let password = read_password(password_stdin)?;
            let format = detect_format(&archive)?;
            if !format.supports_update() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!("{format} archives cannot be renamed"),
                )
                .into());
            }
            let compression_level = resolve_compression_level(format, level)?;
            let summary = rename_archive(
                archive,
                &renames,
                &UpdateOptions {
                    compression_level,
                    password,
                    ..UpdateOptions::default()
                },
            )?;
            println!(
                "Renamed {} entries in {format}; archive now has {} files and {} directories ({} bytes)",
                renames.len(),
                summary.files,
                summary.directories,
                summary.bytes
            );
        }
    }
    Ok(())
}

fn parse_rename_spec(spec: &str) -> io::Result<ArchiveRename> {
    let Some((from, to)) = spec.split_once('=') else {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("rename must use FROM=TO syntax: {spec}"),
        ));
    };
    if from.is_empty() || to.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("rename paths cannot be empty: {spec}"),
        ));
    }
    Ok(ArchiveRename {
        from: PathBuf::from(from),
        to: PathBuf::from(to),
    })
}

fn read_password(enabled: bool) -> io::Result<Option<String>> {
    let stdin = io::stdin();
    read_password_from(&mut stdin.lock(), enabled)
}

fn read_password_from(reader: &mut impl BufRead, enabled: bool) -> io::Result<Option<String>> {
    if !enabled {
        return Ok(None);
    }
    let mut password = String::new();
    if reader.read_line(&mut password)? == 0 {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "--password-stdin requires one line on standard input",
        ));
    }
    while matches!(password.chars().last(), Some('\r' | '\n')) {
        password.pop();
    }
    if password.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "--password-stdin does not accept an empty password",
        ));
    }
    Ok(Some(password))
}

fn print_formats() {
    print!("{}", format_matrix());
}

fn format_matrix() -> String {
    let mut output = String::from(
        "FORMAT\tLIST\tEXTRACT\tCREATE\tCREATE_INPUT\tCOMPRESSION_LEVEL\tENCRYPTION\tSTAGE\n",
    );
    for format in ArchiveFormat::ALL {
        let capabilities = format.capabilities();
        output.push_str(&format!(
            "{format}\t{}\t{}\t{}\t{}\t{}\t{}\t{}",
            yes_no(capabilities.list),
            yes_no(capabilities.extract),
            yes_no(capabilities.create),
            create_input_label(format.create_input()),
            compression_level_label(format),
            yes_no(capabilities.encryption),
            capabilities.stage
        ));
        output.push('\n');
    }
    output
}

fn resolve_compression_level(format: ArchiveFormat, requested: Option<u8>) -> io::Result<u8> {
    let default = CreateOptions::default().compression_level;
    match (format.compression_level_range(), requested) {
        (Some((minimum, maximum)), requested) => {
            let level = requested.unwrap_or(default);
            if (minimum..=maximum).contains(&level) {
                Ok(level)
            } else {
                Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    format!(
                        "compression level for {format} must be between {minimum} and {maximum}; received {level}"
                    ),
                ))
            }
        }
        (None, Some(level)) if format.capabilities().create => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!(
                "compression level is fixed for {format}; omit --level instead of passing {level}"
            ),
        )),
        (None, _) => Ok(default),
    }
}

fn compression_level_label(format: ArchiveFormat) -> String {
    match format.compression_level_range() {
        Some((minimum, maximum)) => format!("{minimum}-{maximum}"),
        None if format.capabilities().create => "fixed".to_owned(),
        None => "none".to_owned(),
    }
}

const fn create_input_label(input: Option<CreateInputKind>) -> &'static str {
    match input {
        Some(CreateInputKind::FilesAndDirectories) => "files-or-directories",
        Some(CreateInputKind::SingleFile) => "single-file",
        None => "none",
    }
}

const fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;
    use std::path::PathBuf;

    use clap::{CommandFactory, Parser, ValueEnum};

    use super::{
        Cli, Command, ConflictArg, FormatArg, RUNTIME_ERROR_EXIT_CODE, format_matrix,
        read_password_from, resolve_compression_level,
    };
    use zifile_core::ArchiveFormat;

    #[test]
    fn public_cli_surface_and_usage_exit_code_are_stable() {
        let command = Cli::command();
        let subcommands: Vec<_> = command
            .get_subcommands()
            .map(|subcommand| subcommand.get_name())
            .collect();
        assert_eq!(
            subcommands,
            [
                "formats", "detect", "list", "test", "extract", "create", "update", "rename"
            ]
        );
        assert_eq!(command.get_version(), Some(env!("CARGO_PKG_VERSION")));
        assert_eq!(RUNTIME_ERROR_EXIT_CODE, 1);
        assert_eq!(
            Cli::try_parse_from(["zifile", "unknown"])
                .unwrap_err()
                .exit_code(),
            2
        );
    }

    #[test]
    fn public_value_names_are_stable() {
        let conflicts: Vec<_> = ConflictArg::value_variants()
            .iter()
            .map(|value| value.to_possible_value().unwrap().get_name().to_owned())
            .collect();
        assert_eq!(conflicts, ["overwrite", "skip", "rename", "error"]);

        let formats: Vec<_> = FormatArg::value_variants()
            .iter()
            .map(|value| value.to_possible_value().unwrap().get_name().to_owned())
            .collect();
        assert_eq!(
            formats,
            [
                "zip",
                "seven-zip",
                "tar",
                "tar-gzip",
                "tar-zstd",
                "tar-xz",
                "tar-lzma",
                "tar-bzip2",
                "tar-lz4",
                "gzip",
                "zstandard",
                "xz",
                "lzma",
                "bzip2",
                "lz4",
                "brotli",
                "cab",
            ]
        );
    }

    #[test]
    fn update_accepts_removals_without_additions() {
        let cli = Cli::try_parse_from([
            "zifile",
            "update",
            "archive.zip",
            "--remove",
            "folder/file.txt",
        ])
        .unwrap();
        let Command::Update {
            additions,
            remove_paths,
            ..
        } = cli.command
        else {
            panic!("expected update command");
        };
        assert!(additions.is_empty());
        assert_eq!(remove_paths, [PathBuf::from("folder/file.txt")]);
    }

    #[test]
    fn rename_accepts_repeated_from_to_mappings() {
        let cli = Cli::try_parse_from([
            "zifile",
            "rename",
            "archive.zip",
            "--rename",
            "old.txt=new.txt",
            "--rename",
            "folder=renamed-folder",
        ])
        .unwrap();
        let Command::Rename { renames, .. } = cli.command else {
            panic!("expected rename command");
        };
        assert_eq!(renames, ["old.txt=new.txt", "folder=renamed-folder"]);
        let parsed: Vec<_> = renames
            .iter()
            .map(|spec| super::parse_rename_spec(spec).unwrap())
            .collect();
        assert_eq!(parsed[0].from, PathBuf::from("old.txt"));
        assert_eq!(parsed[0].to, PathBuf::from("new.txt"));
        assert_eq!(parsed[1].from, PathBuf::from("folder"));
        assert_eq!(parsed[1].to, PathBuf::from("renamed-folder"));
    }

    #[test]
    fn rename_spec_requires_non_empty_from_to_paths() {
        for spec in ["old.txt", "=new.txt", "old.txt="] {
            assert!(super::parse_rename_spec(spec).is_err(), "accepted {spec}");
        }
        let parsed = super::parse_rename_spec("folder\\old.txt=folder/new.txt").unwrap();
        assert_eq!(parsed.from, PathBuf::from("folder\\old.txt"));
        assert_eq!(parsed.to, PathBuf::from("folder/new.txt"));
    }

    #[test]
    fn format_matrix_exposes_creation_contract() {
        let matrix = format_matrix();
        assert!(matrix.starts_with(
            "FORMAT\tLIST\tEXTRACT\tCREATE\tCREATE_INPUT\tCOMPRESSION_LEVEL\tENCRYPTION\tSTAGE\n"
        ));
        assert!(matrix.contains("gzip\tyes\tyes\tyes\tsingle-file\t0-9\tno\tAlpha"));
        assert!(matrix.contains("ZIP\tyes\tyes\tyes\tfiles-or-directories\t0-9\tyes\tAlpha"));
        assert!(matrix.contains("Zstandard\tyes\tyes\tyes\tsingle-file\t0-22\tno\tAlpha"));
        assert!(matrix.contains("LZMA\tyes\tyes\tyes\tsingle-file\t0-9\tno\tAlpha"));
        assert!(matrix.contains("Bzip2\tyes\tyes\tyes\tsingle-file\t1-9\tno\tAlpha"));
        assert!(matrix.contains("Brotli\tyes\tyes\tyes\tsingle-file\t0-11\tno\tAlpha"));
        assert!(matrix.contains("TAR\tyes\tyes\tyes\tfiles-or-directories\tfixed\tno\tAlpha"));
        assert!(matrix.contains("LZ4\tyes\tyes\tyes\tsingle-file\tfixed\tno\tAlpha"));
        assert!(matrix.contains("RAR\tyes\tyes\tno\tnone\tnone\tyes\tBeta"));
        assert!(matrix.contains("CAB\tyes\tyes\tyes\tfiles-or-directories\tfixed\tno\tBeta"));
    }

    #[test]
    fn compression_level_validation_is_format_specific() {
        assert_eq!(
            resolve_compression_level(ArchiveFormat::Zip, None).unwrap(),
            6
        );
        assert_eq!(
            resolve_compression_level(ArchiveFormat::Zip, Some(9)).unwrap(),
            9
        );
        assert!(resolve_compression_level(ArchiveFormat::Zip, Some(10)).is_err());
        assert!(resolve_compression_level(ArchiveFormat::Zstandard, Some(22)).is_ok());
        assert!(resolve_compression_level(ArchiveFormat::Bzip2, Some(0)).is_err());
        assert!(resolve_compression_level(ArchiveFormat::Bzip2, Some(1)).is_ok());
        assert!(resolve_compression_level(ArchiveFormat::Brotli, Some(11)).is_ok());
        assert!(resolve_compression_level(ArchiveFormat::Brotli, Some(12)).is_err());
        assert_eq!(
            resolve_compression_level(ArchiveFormat::Tar, None).unwrap(),
            6
        );
        assert!(resolve_compression_level(ArchiveFormat::Tar, Some(6)).is_err());
        assert_eq!(
            resolve_compression_level(ArchiveFormat::Lz4, None).unwrap(),
            6
        );
        assert!(resolve_compression_level(ArchiveFormat::Lz4, Some(6)).is_err());
    }

    #[test]
    fn password_stdin_is_opt_in() {
        let mut input = Cursor::new(b"ignored\n");
        assert_eq!(read_password_from(&mut input, false).unwrap(), None);
        assert_eq!(input.position(), 0);
    }

    #[test]
    fn password_stdin_removes_only_line_endings() {
        let mut input = Cursor::new(b"  secret phrase  \r\n");
        assert_eq!(
            read_password_from(&mut input, true).unwrap().as_deref(),
            Some("  secret phrase  ")
        );
    }

    #[test]
    fn password_stdin_rejects_missing_or_empty_input() {
        let mut missing = Cursor::new(Vec::<u8>::new());
        assert!(read_password_from(&mut missing, true).is_err());

        let mut empty = Cursor::new(b"\n");
        assert!(read_password_from(&mut empty, true).is_err());
    }
}
