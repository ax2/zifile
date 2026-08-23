use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};
use zifile_core::{
    ArchiveFormat, ConflictPolicy, CreateOptions, ExtractOptions, create_archive, detect_format,
    detect_format_from_path, extract_archive, list_archive, test_archive,
};

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
        #[arg(long)]
        password: Option<String>,
    },
    /// Verify every archive entry without extracting it.
    Test {
        archive: PathBuf,
        #[arg(long)]
        password: Option<String>,
    },
    /// Safely extract an archive.
    Extract {
        archive: PathBuf,
        destination: PathBuf,
        #[arg(long, value_enum, default_value_t = ConflictArg::Error)]
        conflict: ConflictArg,
        #[arg(long)]
        password: Option<String>,
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
        #[arg(long)]
        password: Option<String>,
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
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<(), Box<dyn std::error::Error>> {
    match cli.command {
        Command::Formats => print_formats(),
        Command::Detect { path } => {
            let format = detect_format(&path)?;
            println!("{format}\t{}", format.canonical_extension());
        }
        Command::List { archive, password } => {
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
        Command::Test { archive, password } => {
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
            password,
        } => {
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
            password,
        } => {
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

fn print_formats() {
    println!("FORMAT\tLIST\tEXTRACT\tCREATE\tENCRYPTION\tSTAGE");
    for format in ArchiveFormat::ALL {
        let capabilities = format.capabilities();
        println!(
            "{format}\t{}\t{}\t{}\t{}\t{}",
            yes_no(capabilities.list),
            yes_no(capabilities.extract),
            yes_no(capabilities.create),
            yes_no(capabilities.encryption),
            capabilities.stage
        );
    }
}

const fn yes_no(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
