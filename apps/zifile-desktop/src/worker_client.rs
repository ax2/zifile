use std::ffi::c_void;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use zifile_core::{
    ArchiveEntryInfo, ArchiveFormat, ArchiveInfo, CancellationToken, OperationProgress,
    OperationSummary,
};
use zifile_worker_protocol::{
    Envelope, PROTOCOL_VERSION, WorkerControl, WorkerEvent, WorkerRequest,
};

const MAX_EVENT_BYTES: usize = 4 * 1024 * 1024;
const MAX_STDERR_BYTES: u64 = 64 * 1024;

#[derive(Debug)]
pub enum WorkerOutput {
    Archive(ArchiveInfo),
    Summary(OperationSummary),
}

pub fn run_worker(
    request: WorkerRequest,
    progress: OperationProgress,
    cancellation: CancellationToken,
) -> Result<WorkerOutput, String> {
    let worker_path = worker_path()?;
    let mut command = Command::new(&worker_path);
    command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        use windows_sys::Win32::System::Threading::CREATE_NO_WINDOW;
        command.creation_flags(CREATE_NO_WINDOW);
    }

    let mut child = command
        .spawn()
        .map_err(|error| format!("could not start {}: {error}", worker_path.display()))?;
    let _job = ProcessJob::assign(&mut child)?;

    let mut stdin = child.stdin.take().ok_or("worker stdin was unavailable")?;
    serde_json::to_writer(&mut stdin, &Envelope::new(request))
        .map_err(|error| format!("could not encode worker request: {error}"))?;
    stdin
        .write_all(b"\n")
        .map_err(|error| format!("could not send worker request: {error}"))?;
    let stdout = child.stdout.take().ok_or("worker stdout was unavailable")?;
    let reader_progress = progress.clone();
    let reader = thread::spawn(move || parse_events(BufReader::new(stdout), &reader_progress));
    let stderr = child.stderr.take().ok_or("worker stderr was unavailable")?;
    let stderr_reader = thread::spawn(move || {
        let mut message = String::new();
        let _ = stderr.take(MAX_STDERR_BYTES).read_to_string(&mut message);
        message
    });

    let mut cancel_sent = None;
    let status = loop {
        if cancellation.is_cancelled() && cancel_sent.is_none() {
            let sent = serde_json::to_writer(&mut stdin, &Envelope::new(WorkerControl::Cancel))
                .is_ok()
                && stdin.write_all(b"\n").is_ok()
                && stdin.flush().is_ok();
            if !sent {
                let _ = child.kill();
            }
            cancel_sent = Some(Instant::now());
        }
        if cancel_sent.is_some_and(|sent| sent.elapsed() >= Duration::from_secs(2)) {
            let _ = child.kill();
        }
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                let _ = child.kill();
                return Err(format!("could not monitor worker: {error}"));
            }
        }
    };
    drop(stdin);

    let output = reader
        .join()
        .map_err(|_| "worker event reader panicked".to_owned())?;
    let stderr = stderr_reader
        .join()
        .map_err(|_| "worker stderr reader panicked".to_owned())?;
    if cancel_sent.is_some() {
        return Err("operation cancelled".to_owned());
    }
    if !status.success() {
        return Err(output.err().unwrap_or_else(|| {
            if stderr.trim().is_empty() {
                format!("worker exited unexpectedly with {status}")
            } else {
                format!("worker exited unexpectedly: {}", stderr.trim())
            }
        }));
    }
    output
}

fn parse_events<R: BufRead>(
    mut reader: R,
    progress: &OperationProgress,
) -> Result<WorkerOutput, String> {
    let mut archive: Option<(PathBuf, ArchiveFormat, u64, u64, Vec<ArchiveEntryInfo>)> = None;
    let mut terminal = None;
    loop {
        let mut line = Vec::new();
        let read = reader
            .read_until(b'\n', &mut line)
            .map_err(|error| format!("could not read worker event: {error}"))?;
        if read == 0 {
            break;
        }
        if line.len() > MAX_EVENT_BYTES {
            return Err("worker event exceeded the 4 MiB IPC limit".to_owned());
        }
        let envelope: Envelope<WorkerEvent> = serde_json::from_slice(&line)
            .map_err(|error| format!("worker emitted invalid JSON: {error}"))?;
        if envelope.version != PROTOCOL_VERSION {
            return Err(format!(
                "worker protocol version {} is incompatible with {PROTOCOL_VERSION}",
                envelope.version
            ));
        }
        match envelope.payload {
            WorkerEvent::ArchiveStart {
                path,
                format,
                total_size,
                compressed_size,
            } => archive = Some((path, format, total_size, compressed_size, Vec::new())),
            WorkerEvent::ArchiveEntry { entry } => archive
                .as_mut()
                .ok_or("worker emitted an entry before archive metadata")?
                .4
                .push(entry),
            WorkerEvent::ArchiveEnd => {
                let (path, format, total_size, compressed_size, entries) = archive
                    .take()
                    .ok_or("worker ended an archive it had not started")?;
                set_terminal(
                    &mut terminal,
                    WorkerOutput::Archive(ArchiveInfo {
                        path,
                        format,
                        entries,
                        total_size,
                        compressed_size,
                    }),
                )?;
            }
            WorkerEvent::Progress { snapshot } => progress.update(snapshot),
            WorkerEvent::Summary { summary } => {
                set_terminal(&mut terminal, WorkerOutput::Summary(summary))?;
            }
            WorkerEvent::Error { message } => return Err(message),
        }
    }
    terminal.ok_or_else(|| "worker closed IPC without a terminal event".to_owned())
}

fn set_terminal(slot: &mut Option<WorkerOutput>, value: WorkerOutput) -> Result<(), String> {
    if slot.replace(value).is_some() {
        Err("worker emitted multiple terminal events".to_owned())
    } else {
        Ok(())
    }
}

fn worker_path() -> Result<PathBuf, String> {
    if let Some(path) = std::env::var_os("ZIFILE_WORKER_PATH") {
        return Ok(PathBuf::from(path));
    }
    let executable = std::env::current_exe()
        .map_err(|error| format!("could not locate the desktop executable: {error}"))?;
    let default = executable.with_file_name(if cfg!(windows) {
        "zifile-worker.exe"
    } else {
        "zifile-worker"
    });
    if default.is_file() {
        return Ok(default);
    }
    let file_name = executable
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    let release_name = file_name.replacen("zifile-desktop", "zifile-worker", 1);
    let release = executable.with_file_name(release_name);
    if release.is_file() {
        Ok(release)
    } else {
        Ok(default)
    }
}

#[cfg(windows)]
struct ProcessJob(windows_sys::Win32::Foundation::HANDLE);

#[cfg(windows)]
impl ProcessJob {
    fn assign(child: &mut Child) -> Result<Self, String> {
        use std::mem::{size_of, zeroed};
        use std::os::windows::io::AsRawHandle;
        use windows_sys::Win32::Foundation::CloseHandle;
        use windows_sys::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_ACTIVE_PROCESS,
            JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE, JOB_OBJECT_LIMIT_PROCESS_MEMORY,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };

        let handle = unsafe { CreateJobObjectW(std::ptr::null(), std::ptr::null()) };
        if handle.is_null() {
            let _ = child.kill();
            return Err(format!(
                "could not create worker Job Object: {}",
                std::io::Error::last_os_error()
            ));
        }
        let mut info: JOBOBJECT_EXTENDED_LIMIT_INFORMATION = unsafe { zeroed() };
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE
            | JOB_OBJECT_LIMIT_PROCESS_MEMORY
            | JOB_OBJECT_LIMIT_ACTIVE_PROCESS;
        info.BasicLimitInformation.ActiveProcessLimit = 1;
        info.ProcessMemoryLimit = 4 * 1024 * 1024 * 1024usize;
        let configured = unsafe {
            SetInformationJobObject(
                handle,
                JobObjectExtendedLimitInformation,
                &info as *const _ as *const c_void,
                size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
        };
        let assigned = configured != 0
            && unsafe { AssignProcessToJobObject(handle, child.as_raw_handle() as _) } != 0;
        if !assigned {
            unsafe { CloseHandle(handle) };
            let _ = child.kill();
            return Err(format!(
                "could not constrain worker process: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(Self(handle))
    }
}

#[cfg(windows)]
impl Drop for ProcessJob {
    fn drop(&mut self) {
        unsafe { windows_sys::Win32::Foundation::CloseHandle(self.0) };
    }
}

#[cfg(not(windows))]
struct ProcessJob;

#[cfg(not(windows))]
impl ProcessJob {
    fn assign(_child: &mut Child) -> Result<Self, String> {
        Ok(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn rejects_streams_without_a_terminal_event() {
        let progress = OperationProgress::default();
        let event = serde_json::to_string(&Envelope::new(WorkerEvent::Progress {
            snapshot: Default::default(),
        }))
        .unwrap();
        assert!(parse_events(Cursor::new(format!("{event}\n")), &progress).is_err());
    }

    #[test]
    fn parses_streamed_archive_entries() {
        let events = [
            WorkerEvent::ArchiveStart {
                path: PathBuf::from("a.zip"),
                format: ArchiveFormat::Zip,
                total_size: 3,
                compressed_size: 2,
            },
            WorkerEvent::ArchiveEntry {
                entry: ArchiveEntryInfo {
                    path: PathBuf::from("a.txt"),
                    size: 3,
                    compressed_size: 2,
                    is_directory: false,
                    encrypted: false,
                    modified: None,
                },
            },
            WorkerEvent::ArchiveEnd,
        ];
        let stream = events
            .into_iter()
            .map(|event| serde_json::to_string(&Envelope::new(event)).unwrap())
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        let output = parse_events(Cursor::new(stream), &OperationProgress::default()).unwrap();
        let WorkerOutput::Archive(archive) = output else {
            panic!("expected archive output")
        };
        assert_eq!(archive.entries.len(), 1);
    }
}
