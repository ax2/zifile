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

use std::collections::HashSet;
use std::io::{self, BufRead, BufWriter, Read, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use zifile_core::{
    CreateOptions, ExtractOptions, ListOptions, OperationProgress, TestOptions, create_archive,
    extract_archive, list_archive_with_options, test_archive_with_options,
};
use zifile_worker_protocol::{
    Envelope, PROTOCOL_VERSION, WorkerControl, WorkerEvent, WorkerRequest,
};

type SharedWriter = Arc<Mutex<BufWriter<io::Stdout>>>;
const MAX_REQUEST_BYTES: u64 = 16 * 1024 * 1024;

fn main() {
    if let Err(error) = run() {
        let writer = shared_stdout();
        let _ = emit(
            &writer,
            WorkerEvent::Error {
                message: error.to_string(),
            },
        );
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let request = read_request(&mut io::stdin().lock())?;
    let writer = shared_stdout();
    match request {
        WorkerRequest::List { archive, password } => {
            let progress = OperationProgress::default();
            let cancellation = zifile_core::CancellationToken::default();
            listen_for_cancel(cancellation.clone());
            let options = ListOptions {
                password,
                cancellation,
                progress: progress.clone(),
                ..ListOptions::default()
            };
            let archive = with_progress(writer.clone(), progress, move || {
                list_archive_with_options(archive, &options)
            })?;
            emit_archive(&writer, archive)?;
        }
        WorkerRequest::Test { archive, password } => {
            let progress = OperationProgress::default();
            let cancellation = zifile_core::CancellationToken::default();
            listen_for_cancel(cancellation.clone());
            let options = TestOptions {
                password,
                cancellation,
                progress: progress.clone(),
                ..TestOptions::default()
            };
            let archive = with_progress(writer.clone(), progress, move || {
                test_archive_with_options(archive, &options)
            })?;
            emit_archive(&writer, archive)?;
        }
        WorkerRequest::Extract {
            archive,
            destination,
            conflict,
            limits,
            password,
            selected_paths,
        } => {
            let progress = OperationProgress::default();
            let cancellation = zifile_core::CancellationToken::default();
            listen_for_cancel(cancellation.clone());
            let options = ExtractOptions {
                conflict,
                limits,
                password,
                selected_paths: selected_paths
                    .map(|paths| paths.into_iter().collect::<HashSet<_>>()),
                progress: progress.clone(),
                cancellation,
            };
            let summary = with_progress(writer.clone(), progress, move || {
                extract_archive(archive, destination, &options)
            })?;
            emit(&writer, WorkerEvent::Summary { summary })?;
        }
        WorkerRequest::Create {
            sources,
            destination,
            format,
            compression_level,
            password,
        } => {
            let progress = OperationProgress::default();
            let cancellation = zifile_core::CancellationToken::default();
            listen_for_cancel(cancellation.clone());
            let options = CreateOptions {
                compression_level,
                password,
                progress: progress.clone(),
                cancellation,
            };
            let summary = with_progress(writer.clone(), progress, move || {
                create_archive(&sources, destination, format, &options)
            })?;
            emit(&writer, WorkerEvent::Summary { summary })?;
        }
    }
    Ok(())
}

fn read_request(reader: &mut impl BufRead) -> Result<WorkerRequest, Box<dyn std::error::Error>> {
    let input = read_bounded_line(reader, MAX_REQUEST_BYTES)?;
    if input.len() as u64 > MAX_REQUEST_BYTES {
        return Err("worker request exceeds the 16 MiB IPC limit".into());
    }
    let envelope: Envelope<WorkerRequest> = serde_json::from_slice(&input)?;
    if envelope.version != PROTOCOL_VERSION {
        return Err(format!(
            "unsupported worker protocol version {}; expected {PROTOCOL_VERSION}",
            envelope.version
        )
        .into());
    }

    Ok(envelope.payload)
}

fn read_bounded_line(reader: &mut impl BufRead, limit: u64) -> io::Result<Vec<u8>> {
    let mut line = Vec::new();
    Read::take(reader, limit + 1).read_until(b'\n', &mut line)?;
    Ok(line)
}

fn listen_for_cancel(cancellation: zifile_core::CancellationToken) {
    thread::spawn(move || {
        let Ok(line) = read_bounded_line(&mut io::stdin().lock(), 4096) else {
            return;
        };
        let Ok(envelope) = serde_json::from_slice::<Envelope<WorkerControl>>(&line) else {
            return;
        };
        if envelope.version == PROTOCOL_VERSION && matches!(envelope.payload, WorkerControl::Cancel)
        {
            cancellation.cancel();
        }
    });
}

fn shared_stdout() -> SharedWriter {
    Arc::new(Mutex::new(BufWriter::new(io::stdout())))
}

fn emit_archive(writer: &SharedWriter, archive: zifile_core::ArchiveInfo) -> io::Result<()> {
    emit(
        writer,
        WorkerEvent::ArchiveStart {
            path: archive.path,
            format: archive.format,
            total_size: archive.total_size,
            compressed_size: archive.compressed_size,
        },
    )?;
    for entry in archive.entries {
        emit(writer, WorkerEvent::ArchiveEntry { entry })?;
    }
    emit(writer, WorkerEvent::ArchiveEnd)
}

fn with_progress<T, F>(
    writer: SharedWriter,
    progress: OperationProgress,
    operation: F,
) -> zifile_core::ZiFileResult<T>
where
    F: FnOnce() -> zifile_core::ZiFileResult<T>,
{
    let done = Arc::new(AtomicBool::new(false));
    let reporter_done = done.clone();
    let reporter_writer = writer.clone();
    let reporter_progress = progress.clone();
    thread::scope(|scope| {
        let reporter = scope.spawn(move || {
            while !reporter_done.load(Ordering::Acquire) {
                let _ = emit(
                    &reporter_writer,
                    WorkerEvent::Progress {
                        snapshot: reporter_progress.snapshot(),
                    },
                );
                thread::sleep(Duration::from_millis(100));
            }
        });
        let result = operation();
        done.store(true, Ordering::Release);
        let _ = reporter.join();
        let _ = emit(
            &writer,
            WorkerEvent::Progress {
                snapshot: progress.snapshot(),
            },
        );
        result
    })
}

fn emit(writer: &SharedWriter, event: WorkerEvent) -> io::Result<()> {
    let mut writer = writer
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    serde_json::to_writer(&mut *writer, &Envelope::new(event))?;
    writer.write_all(b"\n")?;
    writer.flush()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn rejects_incompatible_protocol_versions() {
        let input =
            br#"{"version":2,"payload":{"operation":"list","archive":"a.zip","password":null}}"#;
        assert!(read_request(&mut Cursor::new(input)).is_err());
    }

    #[test]
    fn rejects_requests_over_the_ipc_limit() {
        let mut input = io::BufReader::new(io::repeat(b' ').take(MAX_REQUEST_BYTES + 1));
        assert!(read_request(&mut input).is_err());
    }
}
