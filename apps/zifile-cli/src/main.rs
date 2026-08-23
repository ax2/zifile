use std::path::PathBuf;

use clap::{Parser, Subcommand};
use zifile_core::{ArchiveFormat, detect_format_from_path};

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
    /// Show the planned format capability matrix.
    Formats,
    /// Detect a format from its file name or path.
    Detect { path: PathBuf },
}

fn main() {
    match Cli::parse().command {
        Command::Formats => print_formats(),
        Command::Detect { path } => match detect_format_from_path(&path) {
            Some(format) => println!(
                "{format} (.{}), planned for {}",
                format.canonical_extension(),
                format.capabilities().stage
            ),
            None => {
                eprintln!(
                    "ZiFile could not identify the format from: {}",
                    path.display()
                );
                std::process::exit(2);
            }
        },
    }
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
