use std::{
    io::{self, BufRead},
    path::PathBuf,
};

use clap::{Parser, Subcommand, ValueEnum};
use zifile_core::{
    ArchiveFormat, ConflictPolicy, CreateInputKind, CreateOptions, ExtractOptions, create_archive,
    detect_format, detect_format_from_path, extract_archive, list_archive, test_archive,
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
        #[arg(long, default_value_t = 6, value_parser = clap::value_parser!(u8).range(0..=22))]
        level: u8,
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
    TarBzip2,
    Gzip,
    Zstandard,
    Xz,
    Bzip2,
    Lz4,
    Brotli,
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
            FormatArg::TarBzip2 => Self::TarBzip2,
            FormatArg::Gzip => Self::Gzip,
            FormatArg::Zstandard => Self::Zstandard,
            FormatArg::Xz => Self::Xz,
            FormatArg::Bzip2 => Self::Bzip2,
            FormatArg::Lz4 => Self::Lz4,
            FormatArg::Brotli => Self::Brotli,
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
                "OK: {} entries, {} expanded bytes",
                info.entries.len(),
                info.total_size
            );
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
            let summary = create_archive(
                &sources,
                destination,
                format,
                &CreateOptions {
                    compression_level: level,
                    password,
                    ..CreateOptions::default()
                },
            )?;
            println!(
                "Created {format} from {} files and {} directories ({} input bytes)",
                summary.files, summary.directories, summary.bytes
            );
        }
    }
    Ok(())
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
    let mut output =
        String::from("FORMAT\tLIST\tEXTRACT\tCREATE\tCREATE_INPUT\tENCRYPTION\tSTAGE\n");
    for format in ArchiveFormat::ALL {
        let capabilities = format.capabilities();
        output.push_str(&format!(
            "{format}\t{}\t{}\t{}\t{}\t{}\t{}",
            yes_no(capabilities.list),
            yes_no(capabilities.extract),
            yes_no(capabilities.create),
            create_input_label(format.create_input()),
            yes_no(capabilities.encryption),
            capabilities.stage
        ));
        output.push('\n');
    }
    output
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

    use clap::{CommandFactory, Parser, ValueEnum};

    use super::{
        Cli, ConflictArg, FormatArg, RUNTIME_ERROR_EXIT_CODE, format_matrix, read_password_from,
    };

    #[test]
    fn public_cli_surface_and_usage_exit_code_are_stable() {
        let command = Cli::command();
        let subcommands: Vec<_> = command
            .get_subcommands()
            .map(|subcommand| subcommand.get_name())
            .collect();
        assert_eq!(
            subcommands,
            ["formats", "detect", "list", "test", "extract", "create"]
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
                "tar-bzip2",
                "gzip",
                "zstandard",
                "xz",
                "bzip2",
                "lz4",
                "brotli",
            ]
        );
    }

    #[test]
    fn format_matrix_exposes_creation_input_contract() {
        let matrix = format_matrix();
        assert!(
            matrix.starts_with("FORMAT\tLIST\tEXTRACT\tCREATE\tCREATE_INPUT\tENCRYPTION\tSTAGE\n")
        );
        assert!(matrix.contains("gzip\tyes\tyes\tyes\tsingle-file\tno\tAlpha"));
        assert!(matrix.contains("ZIP\tyes\tyes\tyes\tfiles-or-directories\tyes\tAlpha"));
        assert!(matrix.contains("RAR\tyes\tyes\tno\tnone\tyes\tBeta"));
        assert!(matrix.contains("CAB\tyes\tyes\tno\tnone\tno\tBeta"));
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
