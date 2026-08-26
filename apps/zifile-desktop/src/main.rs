use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

mod i18n;
mod settings;
mod taskbar;
mod worker_client;

use iced::widget::{
    button, checkbox, column, container, pick_list, progress_bar, row, rule, scrollable, slider,
    space, text, text_input,
};
use iced::{Element, Fill, Length, Subscription, Task, Theme};
use rfd::FileDialog;
use zifile_core::{
    ArchiveFormat, ArchiveInfo, CancellationToken, ConflictPolicy, CreateInputKind,
    OPEN_ARCHIVE_EXTENSIONS, OperationProgress, OperationSummary, SafetyLimits,
    detect_format_from_path,
};
use zifile_worker_protocol::WorkerRequest;

use i18n::{Locale, Text, format_archive_modified};
use settings::AppSettings;
use worker_client::{WorkerOutput, run_worker};
use zifile_desktop::create_validation::{CreateSourceIssue, create_source_issue};
use zifile_desktop::entry_view::{
    DirectorySelection, ENTRIES_PER_PAGE, EntrySort, SortDirection, browser_entry_count,
    browser_entry_page, child_directory_selections, descendant_file_paths, directory_breadcrumbs,
    next_sort,
};
use zifile_desktop::operation_queue::{Job, OperationQueue, Submission};
use zifile_desktop::startup::{self, StartupRequest};

const CREATE_FORMATS: [ArchiveFormat; 13] = [
    ArchiveFormat::Zip,
    ArchiveFormat::SevenZip,
    ArchiveFormat::Tar,
    ArchiveFormat::TarGzip,
    ArchiveFormat::TarZstd,
    ArchiveFormat::TarXz,
    ArchiveFormat::TarBzip2,
    ArchiveFormat::Gzip,
    ArchiveFormat::Zstandard,
    ArchiveFormat::Xz,
    ArchiveFormat::Bzip2,
    ArchiveFormat::Lz4,
    ArchiveFormat::Brotli,
];

pub fn main() -> iced::Result {
    iced::application(initialize, update, view)
        .title(title)
        .theme(theme)
        .subscription(subscription)
        .window_size((1_180.0, 780.0))
        .antialiasing(true)
        .run()
}

fn initialize() -> (ZiFile, Task<Message>) {
    let mut state = ZiFile::default();
    let task = match startup::parse(std::env::args_os().skip(1)) {
        StartupRequest::Home => Task::none(),
        StartupRequest::OpenArchive(path) => begin_load(&mut state, path),
        StartupRequest::ExtractHere(path) => {
            state.automatic_extract_destination = Some(startup::extraction_destination(&path));
            begin_load(&mut state, path)
        }
        StartupRequest::CreateFrom(sources) => {
            state.page = Page::Create;
            append_unique(&mut state.create_sources, sources);
            Task::none()
        }
    };
    (state, task)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Archive,
    Create,
}

#[derive(Debug)]
struct ZiFile {
    page: Page,
    archive: Option<ArchiveInfo>,
    pending_archive: Option<PathBuf>,
    automatic_extract_destination: Option<PathBuf>,
    selected: HashSet<PathBuf>,
    entry_directory: PathBuf,
    entry_filter: String,
    entry_page: usize,
    entry_sort: EntrySort,
    entry_sort_direction: SortDirection,
    password: String,
    conflict: ConflictChoice,
    create_sources: Vec<PathBuf>,
    create_format: ArchiveFormat,
    create_password: String,
    compression_level: u8,
    status: String,
    busy: bool,
    cancellation: Option<CancellationToken>,
    progress: Option<OperationProgress>,
    operations: OperationQueue<QueuedOperation>,
    dark: bool,
    locale: Locale,
}

impl Default for ZiFile {
    fn default() -> Self {
        let settings = AppSettings::load();
        Self {
            page: Page::Home,
            archive: None,
            pending_archive: None,
            automatic_extract_destination: None,
            selected: HashSet::new(),
            entry_directory: PathBuf::new(),
            entry_filter: String::new(),
            entry_page: 0,
            entry_sort: EntrySort::default(),
            entry_sort_direction: SortDirection::default(),
            password: String::new(),
            conflict: ConflictChoice::Rename,
            create_sources: Vec::new(),
            create_format: ArchiveFormat::Zip,
            create_password: String::new(),
            compression_level: 6,
            status: settings.locale.text(Text::Ready).to_owned(),
            busy: false,
            cancellation: None,
            progress: None,
            operations: OperationQueue::default(),
            dark: settings.dark,
            locale: settings.locale,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Navigate(Page),
    ToggleTheme,
    ToggleLocale,
    OpenArchiveDialog,
    ArchiveLoaded(u64, Result<ArchiveInfo, String>),
    PasswordChanged(String),
    ReloadArchive,
    ToggleEntry(PathBuf, bool),
    ToggleDirectory(PathBuf, bool),
    SelectAll(bool),
    NavigateArchiveDirectory(PathBuf),
    EntryFilterChanged(String),
    SortEntries(EntrySort),
    PreviousEntryPage,
    NextEntryPage,
    ConflictChanged(ConflictChoice),
    Extract,
    ExtractFinished(u64, Result<OperationSummary, String>),
    TestArchive,
    TestFinished(u64, Result<ArchiveInfo, String>),
    AddFiles,
    AddFolder,
    RemoveSource(usize),
    ClearSources,
    CreateFormatChanged(ArchiveFormat),
    CreatePasswordChanged(String),
    CompressionLevelChanged(u8),
    Create,
    CreateFinished(u64, Result<OperationSummary, String>),
    ClearQueued,
    Cancel,
    ProgressTick,
    FileDropped(PathBuf),
    KeyboardShortcut(Shortcut),
}

#[derive(Debug, Clone, Copy)]
enum Shortcut {
    Open,
    Create,
    SelectAll,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationKind {
    List,
    Test,
    Extract,
    Create,
}

struct QueuedOperation {
    kind: OperationKind,
    request: WorkerRequest,
    status: String,
    archive_path: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ConflictChoice {
    Overwrite,
    Skip,
    Rename,
    Error,
}

impl ConflictChoice {
    const ALL: [Self; 4] = [Self::Rename, Self::Overwrite, Self::Skip, Self::Error];
}

impl fmt::Display for ConflictChoice {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Overwrite => "Overwrite existing",
            Self::Skip => "Skip existing",
            Self::Rename => "Keep both (rename)",
            Self::Error => "Stop on conflict",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct LocalizedConflict {
    choice: ConflictChoice,
    locale: Locale,
}

impl fmt::Display for LocalizedConflict {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let key = match self.choice {
            ConflictChoice::Overwrite => Text::ConflictOverwrite,
            ConflictChoice::Skip => Text::ConflictSkip,
            ConflictChoice::Rename => Text::ConflictRename,
            ConflictChoice::Error => Text::ConflictError,
        };
        formatter.write_str(self.locale.text(key))
    }
}

impl From<ConflictChoice> for ConflictPolicy {
    fn from(value: ConflictChoice) -> Self {
        match value {
            ConflictChoice::Overwrite => Self::Overwrite,
            ConflictChoice::Skip => Self::Skip,
            ConflictChoice::Rename => Self::Rename,
            ConflictChoice::Error => Self::Error,
        }
    }
}

fn title(state: &ZiFile) -> String {
    state.archive.as_ref().map_or_else(
        || "ZiFile".to_owned(),
        |archive| format!("{} — ZiFile", archive.path.display()),
    )
}

fn theme(state: &ZiFile) -> Theme {
    if state.dark {
        Theme::TokyoNight
    } else {
        Theme::Light
    }
}

fn update(state: &mut ZiFile, message: Message) -> Task<Message> {
    match message {
        Message::Navigate(page) => state.page = page,
        Message::ToggleTheme => {
            state.dark = !state.dark;
            save_settings(state);
        }
        Message::ToggleLocale => {
            state.locale = state.locale.toggle();
            state.status = state.locale.text(Text::Ready).to_owned();
            save_settings(state);
        }
        Message::OpenArchiveDialog => {
            if let Some(path) = archive_dialog(state.locale).pick_file() {
                state.password.clear();
                state.automatic_extract_destination = None;
                return begin_load(state, path);
            }
        }
        Message::ArchiveLoaded(id, result) => {
            state.busy = false;
            state.cancellation = None;
            state.progress = None;
            match result {
                Ok(archive) => {
                    state.selected = archive
                        .entries
                        .iter()
                        .filter(|entry| !entry.is_directory)
                        .map(|entry| entry.path.clone())
                        .collect();
                    state.entry_filter.clear();
                    state.entry_directory.clear();
                    state.entry_page = 0;
                    state.entry_sort = EntrySort::Name;
                    state.entry_sort_direction = SortDirection::Ascending;
                    state.status = if state.locale == Locale::ZhCn {
                        format!(
                            "已打开 {} 个项目 · 展开后 {}",
                            archive.entries.len(),
                            format_bytes(archive.total_size)
                        )
                    } else {
                        format!(
                            "Opened {} entries · {} expanded",
                            archive.entries.len(),
                            format_bytes(archive.total_size)
                        )
                    };
                    state.archive = Some(archive);
                    state.pending_archive = None;
                    state.page = Page::Archive;
                    if let Some(destination) = state.automatic_extract_destination.take()
                        && let Some(operation) = extract_operation(state, destination)
                    {
                        drop(submit_operation(state, operation));
                    }
                }
                Err(error) => {
                    state.status = format!(
                        "{}: {error}",
                        choose(state.locale, "Open failed", "打开失败")
                    )
                }
            }
            return continue_queue(state, id);
        }
        Message::PasswordChanged(password) => state.password = password,
        Message::ReloadArchive => {
            if let Some(path) = state
                .archive
                .as_ref()
                .map(|archive| archive.path.clone())
                .or_else(|| state.pending_archive.clone())
            {
                return begin_load(state, path);
            }
        }
        Message::ToggleEntry(path, selected) => {
            if selected {
                state.selected.insert(path);
            } else {
                state.selected.remove(&path);
            }
        }
        Message::ToggleDirectory(directory, selected) => {
            if let Some(archive) = &state.archive {
                let descendants = descendant_file_paths(archive, &directory);
                if selected {
                    state.selected.extend(descendants);
                } else {
                    for path in descendants {
                        state.selected.remove(&path);
                    }
                }
            }
        }
        Message::SelectAll(selected) => {
            state.selected.clear();
            if selected && let Some(archive) = &state.archive {
                state.selected.extend(
                    archive
                        .entries
                        .iter()
                        .filter(|entry| !entry.is_directory)
                        .map(|entry| entry.path.clone()),
                );
            }
        }
        Message::NavigateArchiveDirectory(directory) => {
            state.entry_directory = directory;
            state.entry_filter.clear();
            state.entry_page = 0;
        }
        Message::EntryFilterChanged(filter) => {
            state.entry_filter = filter;
            state.entry_page = 0;
        }
        Message::SortEntries(sort) => {
            (state.entry_sort, state.entry_sort_direction) =
                next_sort(state.entry_sort, state.entry_sort_direction, sort);
            state.entry_page = 0;
        }
        Message::PreviousEntryPage => {
            state.entry_page = state.entry_page.saturating_sub(1);
        }
        Message::NextEntryPage => {
            if let Some(archive) = &state.archive {
                let count =
                    browser_entry_count(archive, &state.entry_directory, &state.entry_filter);
                let last_page = count.saturating_sub(1) / ENTRIES_PER_PAGE;
                state.entry_page = (state.entry_page + 1).min(last_page);
            }
        }
        Message::ConflictChanged(conflict) => state.conflict = conflict,
        Message::Extract => {
            let Some(archive) = state.archive.as_ref() else {
                return Task::none();
            };
            let default_folder = archive
                .path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join(archive.path.file_stem().unwrap_or_default());
            let Some(destination) = FileDialog::new()
                .set_title(state.locale.text(Text::ChooseExtractionFolder))
                .set_directory(default_folder.parent().unwrap_or_else(|| Path::new(".")))
                .pick_folder()
            else {
                return Task::none();
            };
            return extract_operation(state, destination)
                .map_or_else(Task::none, |operation| submit_operation(state, operation));
        }
        Message::ExtractFinished(id, result) => {
            state.busy = false;
            state.cancellation = None;
            state.progress = None;
            state.status = match result {
                Ok(summary) if state.locale == Locale::ZhCn => format!(
                    "已解压 {} 个文件 · {} · 跳过 {} 个",
                    summary.files,
                    format_bytes(summary.bytes),
                    summary.skipped
                ),
                Ok(summary) => format!(
                    "Extracted {} files · {} · skipped {}",
                    summary.files,
                    format_bytes(summary.bytes),
                    summary.skipped
                ),
                Err(error) => format!(
                    "{}: {error}",
                    choose(state.locale, "Extraction failed", "解压失败")
                ),
            };
            return continue_queue(state, id);
        }
        Message::TestArchive => {
            let Some(path) = state.archive.as_ref().map(|archive| archive.path.clone()) else {
                return Task::none();
            };
            let password = non_empty(&state.password);
            let request = WorkerRequest::Test {
                archive: path,
                password,
            };
            let status = choose(
                state.locale,
                "Testing every entry and checksum…",
                "正在校验所有项目与校验和…",
            )
            .to_owned();
            return submit_operation(
                state,
                QueuedOperation {
                    kind: OperationKind::Test,
                    request,
                    status,
                    archive_path: None,
                },
            );
        }
        Message::TestFinished(id, result) => {
            state.busy = false;
            state.cancellation = None;
            state.progress = None;
            state.status = match result {
                Ok(info) if state.locale == Locale::ZhCn => format!(
                    "压缩文件完好 · {} 个项目 · {}",
                    info.entries.len(),
                    format_bytes(info.total_size)
                ),
                Ok(info) => format!(
                    "Archive is healthy · {} entries · {}",
                    info.entries.len(),
                    format_bytes(info.total_size)
                ),
                Err(error) => format!(
                    "{}: {error}",
                    choose(state.locale, "Integrity test failed", "完整性校验失败")
                ),
            };
            return continue_queue(state, id);
        }
        Message::AddFiles => {
            if let Some(paths) = FileDialog::new()
                .set_title(state.locale.text(Text::AddFilesDialog))
                .pick_files()
            {
                append_unique(&mut state.create_sources, paths);
                state.page = Page::Create;
            }
        }
        Message::AddFolder => {
            if state.create_format.create_input() == Some(CreateInputKind::SingleFile) {
                state.status = state.locale.text(Text::SingleFileRequired).to_owned();
            } else if let Some(path) = FileDialog::new()
                .set_title(state.locale.text(Text::AddFolderDialog))
                .pick_folder()
            {
                append_unique(&mut state.create_sources, vec![path]);
                state.page = Page::Create;
            }
        }
        Message::RemoveSource(index) => {
            if index < state.create_sources.len() {
                state.create_sources.remove(index);
            }
        }
        Message::ClearSources => state.create_sources.clear(),
        Message::CreateFormatChanged(format) => {
            state.create_format = format;
            if !format.capabilities().encryption {
                state.create_password.clear();
            }
            state.status = create_input_help(state.locale, format).to_owned();
        }
        Message::CreatePasswordChanged(password) => state.create_password = password,
        Message::CompressionLevelChanged(level) => state.compression_level = level,
        Message::Create => {
            if let Some(issue) = create_source_issue(state.create_format, &state.create_sources) {
                state.status = create_source_issue_text(state.locale, issue).to_owned();
                return Task::none();
            }
            let extension = state.create_format.canonical_extension();
            let Some(destination) = FileDialog::new()
                .set_title(state.locale.text(Text::CreateDialog))
                .add_filter(state.create_format.to_string(), &[extension])
                .set_file_name(format!("archive.{extension}"))
                .save_file()
            else {
                return Task::none();
            };
            let sources = state.create_sources.clone();
            let format = state.create_format;
            let request = WorkerRequest::Create {
                sources,
                destination: destination.clone(),
                format,
                compression_level: state.compression_level,
                password: non_empty(&state.create_password),
            };
            let status = format!(
                "{} {}…",
                choose(state.locale, "Creating", "正在创建"),
                destination.display()
            );
            return submit_operation(
                state,
                QueuedOperation {
                    kind: OperationKind::Create,
                    request,
                    status,
                    archive_path: None,
                },
            );
        }
        Message::CreateFinished(id, result) => {
            state.busy = false;
            state.cancellation = None;
            state.progress = None;
            state.status = match result {
                Ok(summary) if state.locale == Locale::ZhCn => format!(
                    "压缩文件已创建 · {} 个文件 · 输入 {}",
                    summary.files,
                    format_bytes(summary.bytes)
                ),
                Ok(summary) => format!(
                    "Archive created · {} files · {} input",
                    summary.files,
                    format_bytes(summary.bytes)
                ),
                Err(error) => format!(
                    "{}: {error}",
                    choose(state.locale, "Creation failed", "创建失败")
                ),
            };
            return continue_queue(state, id);
        }
        Message::ClearQueued => {
            let cleared = state.operations.clear_pending().len();
            state.status = match state.locale {
                Locale::En => format!("Cleared {cleared} queued operations"),
                Locale::ZhCn => format!("已清除 {cleared} 个排队操作"),
            };
        }
        Message::Cancel => {
            if let Some(cancellation) = &state.cancellation {
                cancellation.cancel();
                state.status = choose(
                    state.locale,
                    "Cancelling safely after the current block…",
                    "正在当前数据块结束后安全取消…",
                )
                .to_owned();
            }
        }
        Message::ProgressTick => {}
        Message::FileDropped(path) => {
            if path.is_file()
                && detect_format_from_path(&path).is_some_and(|format| format.capabilities().list)
            {
                state.password.clear();
                state.automatic_extract_destination = None;
                return begin_load(state, path);
            } else if path.exists() {
                append_unique(&mut state.create_sources, vec![path]);
                state.page = Page::Create;
                state.status =
                    choose(state.locale, "Added dropped source", "已添加拖入的来源").to_owned();
            }
        }
        Message::KeyboardShortcut(shortcut) => match shortcut {
            Shortcut::Open => return update(state, Message::OpenArchiveDialog),
            Shortcut::Create => state.page = Page::Create,
            Shortcut::SelectAll if state.page == Page::Archive => {
                return update(state, Message::SelectAll(true));
            }
            Shortcut::Cancel if state.cancellation.is_some() => {
                return update(state, Message::Cancel);
            }
            _ => {}
        },
    }
    taskbar::sync(
        state.busy,
        state
            .cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled),
        state.progress.as_ref().map(OperationProgress::snapshot),
    );
    Task::none()
}

fn subscription(state: &ZiFile) -> Subscription<Message> {
    let progress = if state.busy && state.progress.is_some() {
        iced::time::every(Duration::from_millis(100)).map(|_| Message::ProgressTick)
    } else {
        Subscription::none()
    };
    Subscription::batch([progress, iced::event::listen_with(ui_event)])
}

fn ui_event(
    event: iced::Event,
    status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Window(iced::window::Event::FileDropped(path)) => {
            Some(Message::FileDropped(path))
        }
        iced::Event::Keyboard(iced::keyboard::Event::KeyPressed {
            key,
            physical_key,
            modifiers,
            repeat: false,
            ..
        }) if status == iced::event::Status::Ignored => {
            if key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) {
                return Some(Message::KeyboardShortcut(Shortcut::Cancel));
            }
            if !modifiers.control() {
                return None;
            }
            match key.to_latin(physical_key) {
                Some('o') => Some(Message::KeyboardShortcut(Shortcut::Open)),
                Some('n') => Some(Message::KeyboardShortcut(Shortcut::Create)),
                Some('a') => Some(Message::KeyboardShortcut(Shortcut::SelectAll)),
                _ => None,
            }
        }
        _ => None,
    }
}

fn save_settings(state: &ZiFile) {
    AppSettings {
        locale: state.locale,
        dark: state.dark,
    }
    .save();
}

fn begin_load(state: &mut ZiFile, path: PathBuf) -> Task<Message> {
    state.archive = None;
    state.pending_archive = Some(path.clone());
    state.selected.clear();
    state.page = Page::Archive;
    let password = non_empty(&state.password);
    let request = WorkerRequest::List {
        archive: path.clone(),
        password,
    };
    let status = format!(
        "{} {}…",
        choose(state.locale, "Opening", "正在打开"),
        path.display()
    );
    submit_operation(
        state,
        QueuedOperation {
            kind: OperationKind::List,
            request,
            status,
            archive_path: Some(path),
        },
    )
}

fn extract_operation(state: &ZiFile, destination: PathBuf) -> Option<QueuedOperation> {
    let archive = state.archive.as_ref()?;
    let file_count = archive
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .count();
    let selected_paths = if state.selected.len() == file_count {
        None
    } else {
        Some(state.selected.iter().cloned().collect())
    };
    Some(QueuedOperation {
        kind: OperationKind::Extract,
        request: WorkerRequest::Extract {
            archive: archive.path.clone(),
            destination: destination.clone(),
            conflict: state.conflict.into(),
            limits: SafetyLimits::default(),
            password: non_empty(&state.password),
            selected_paths,
        },
        status: format!(
            "{} {}…",
            choose(state.locale, "Extracting to", "正在解压到"),
            destination.display()
        ),
        archive_path: None,
    })
}

fn submit_operation(state: &mut ZiFile, operation: QueuedOperation) -> Task<Message> {
    match state.operations.submit(operation) {
        Ok(Submission::Start(job)) => start_operation(state, job),
        Ok(Submission::Queued { position, .. }) => {
            state.status = match state.locale {
                Locale::En => format!(
                    "Queued operation at position {position}; the current operation continues"
                ),
                Locale::ZhCn => format!("操作已排队，位置 {position}；当前操作继续运行"),
            };
            Task::none()
        }
        Err(error) => {
            state.status = match state.locale {
                Locale::En => format!("Operation queue is full (maximum {})", error.capacity),
                Locale::ZhCn => format!("操作队列已满（最多 {} 个）", error.capacity),
            };
            Task::none()
        }
    }
}

fn start_operation(state: &mut ZiFile, job: Job<QueuedOperation>) -> Task<Message> {
    let Job { id, payload } = job;
    let QueuedOperation {
        kind,
        request,
        status,
        archive_path,
    } = payload;
    if let Some(path) = archive_path {
        state.archive = None;
        state.pending_archive = Some(path);
        state.selected.clear();
        state.page = Page::Archive;
    }
    let cancellation = CancellationToken::default();
    let progress = OperationProgress::default();
    state.busy = true;
    state.cancellation = Some(cancellation.clone());
    state.progress = Some(progress.clone());
    state.status = status;
    match kind {
        OperationKind::List => Task::perform(
            async move { run_worker(request, progress, cancellation).and_then(expect_archive) },
            move |result| Message::ArchiveLoaded(id, result),
        ),
        OperationKind::Test => Task::perform(
            async move { run_worker(request, progress, cancellation).and_then(expect_archive) },
            move |result| Message::TestFinished(id, result),
        ),
        OperationKind::Extract => Task::perform(
            async move { run_worker(request, progress, cancellation).and_then(expect_summary) },
            move |result| Message::ExtractFinished(id, result),
        ),
        OperationKind::Create => Task::perform(
            async move { run_worker(request, progress, cancellation).and_then(expect_summary) },
            move |result| Message::CreateFinished(id, result),
        ),
    }
}

fn continue_queue(state: &mut ZiFile, completed_id: u64) -> Task<Message> {
    state.busy = false;
    state.cancellation = None;
    state.progress = None;
    match state.operations.complete(completed_id) {
        Ok(Some(next)) => start_operation(state, next),
        Ok(None) => Task::none(),
        Err(error) => {
            state.status = format!("Internal operation queue error: {error}");
            Task::none()
        }
    }
}

fn expect_archive(output: WorkerOutput) -> Result<ArchiveInfo, String> {
    match output {
        WorkerOutput::Archive(archive) => Ok(archive),
        WorkerOutput::Summary(_) => {
            Err("worker returned an operation summary instead of an archive".to_owned())
        }
    }
}

fn expect_summary(output: WorkerOutput) -> Result<OperationSummary, String> {
    match output {
        WorkerOutput::Summary(summary) => Ok(summary),
        WorkerOutput::Archive(_) => {
            Err("worker returned an archive instead of an operation summary".to_owned())
        }
    }
}

fn view(state: &ZiFile) -> Element<'_, Message> {
    let navigation = container(
        column![
            text("ZiFile").size(30),
            text(state.locale.text(Text::ArchiveStudio)).size(13),
            rule::horizontal(1),
            nav_button(state.locale.text(Text::Home), Page::Home, state.page),
            nav_button(state.locale.text(Text::Archive), Page::Archive, state.page),
            nav_button(state.locale.text(Text::Create), Page::Create, state.page),
            space().height(Fill),
            row![
                button(if state.dark {
                    state.locale.text(Text::Light)
                } else {
                    state.locale.text(Text::Dark)
                })
                .style(button::secondary)
                .width(Fill)
                .on_press(Message::ToggleTheme),
                button(state.locale.text(Text::SwitchLanguage))
                    .style(button::secondary)
                    .width(Fill)
                    .on_press(Message::ToggleLocale),
            ]
            .spacing(8),
        ]
        .spacing(12)
        .padding(20),
    )
    .width(210)
    .height(Fill)
    .style(container::secondary);

    let page = match state.page {
        Page::Home => home_view(state),
        Page::Archive => archive_view(state),
        Page::Create => create_view(state),
    };
    let progress: Element<'_, Message> = state.progress.as_ref().map_or_else(
        || space().height(0).into(),
        |progress| {
            let snapshot = progress.snapshot();
            let progress_text = if snapshot.total_bytes == 0 && snapshot.processed_bytes > 0 {
                match state.locale {
                    Locale::En => {
                        format!("Scanning · {} read", format_bytes(snapshot.processed_bytes))
                    }
                    Locale::ZhCn => format!(
                        "正在扫描 · 已读取 {}",
                        format_bytes(snapshot.processed_bytes)
                    ),
                }
            } else if snapshot.total_entries == 0 && snapshot.processed_entries > 0 {
                match state.locale {
                    Locale::En => {
                        format!("Scanning · {} entries found", snapshot.processed_entries)
                    }
                    Locale::ZhCn => format!("正在扫描 · 已发现 {} 项", snapshot.processed_entries),
                }
            } else {
                format!(
                    "{} / {} · {} / {}",
                    snapshot.processed_entries,
                    snapshot.total_entries,
                    format_bytes(snapshot.processed_bytes),
                    format_bytes(snapshot.total_bytes)
                )
            };
            row![
                container(progress_bar(0.0..=1.0, snapshot.fraction())).width(Fill),
                text(progress_text).size(12),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .into()
        },
    );
    let status_line = format!(
        "{} {}",
        if state.busy { "\u{25cf}" } else { "\u{2022}" },
        state.status
    );
    let status = container(
        column![
            row![
                text(status_line).size(13).width(Fill),
                text(match state.locale {
                    Locale::En => format!("{} queued", state.operations.pending_count()),
                    Locale::ZhCn => format!("{} 个排队", state.operations.pending_count()),
                })
                .size(12),
                button(match state.locale {
                    Locale::En => "Clear queue",
                    Locale::ZhCn => "清空队列",
                })
                .style(button::secondary)
                .on_press_maybe(
                    (state.operations.pending_count() > 0).then_some(Message::ClearQueued)
                ),
                button(state.locale.text(Text::Cancel))
                    .style(button::danger)
                    .on_press_maybe(state.cancellation.as_ref().map(|_| Message::Cancel)),
            ]
            .spacing(12),
            progress,
        ]
        .spacing(6),
    )
    .padding([10, 18])
    .width(Fill)
    .style(container::rounded_box);

    row![navigation, column![page, status].spacing(12).padding(20)]
        .height(Fill)
        .into()
}

fn home_view(state: &ZiFile) -> Element<'_, Message> {
    let open = action_card(
        state.locale.text(Text::OpenArchive),
        state.locale.text(Text::OpenDescription),
        state.locale.text(Text::OpenAction),
        Message::OpenArchiveDialog,
        false,
    );
    let create = action_card(
        state.locale.text(Text::CreateArchive),
        state.locale.text(Text::CreateDescription),
        state.locale.text(Text::StartCreating),
        Message::Navigate(Page::Create),
        false,
    );
    column![
        text(state.locale.text(Text::Hero)).size(38),
        text(state.locale.text(Text::HeroSub)).size(17),
        row![open, create].spacing(18),
        container(
            column![
                text(state.locale.text(Text::Privacy)).size(20),
                text(state.locale.text(Text::PrivacyDescription)),
            ]
            .spacing(8)
        )
        .padding(20)
        .width(Fill)
        .style(container::rounded_box),
    ]
    .spacing(24)
    .width(Fill)
    .into()
}

fn sort_header_label(
    label: &str,
    column: EntrySort,
    active: EntrySort,
    direction: SortDirection,
) -> String {
    if column != active {
        return label.to_owned();
    }
    let arrow = match direction {
        SortDirection::Ascending => "↑",
        SortDirection::Descending => "↓",
    };
    format!("{label} {arrow}")
}

fn folder_selection_summary(locale: Locale, selection: DirectorySelection) -> String {
    match locale {
        Locale::En => format!("{}/{} selected", selection.selected, selection.total),
        Locale::ZhCn => format!("已选 {}/{}", selection.selected, selection.total),
    }
}

fn archive_view(state: &ZiFile) -> Element<'_, Message> {
    let Some(archive) = &state.archive else {
        let pending = state.pending_archive.as_ref();
        let heading = pending.and_then(|path| path.file_name()).map_or_else(
            || state.locale.text(Text::NoArchive).to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        let description = if pending.is_some() {
            state.locale.text(Text::EncryptedArchiveDescription)
        } else {
            state.locale.text(Text::NoArchiveDescription)
        };
        return container(
            column![
                text(heading).size(30),
                text(description),
                text_input(state.locale.text(Text::PasswordEncrypted), &state.password)
                    .secure(true)
                    .on_input(Message::PasswordChanged)
                    .width(280),
                button(state.locale.text(Text::UnlockArchive))
                    .style(button::primary)
                    .on_press_maybe(
                        (pending.is_some() && !state.busy).then_some(Message::ReloadArchive)
                    ),
                button(state.locale.text(Text::OpenAction))
                    .style(button::secondary)
                    .on_press(Message::OpenArchiveDialog),
            ]
            .spacing(16),
        )
        .center_x(Fill)
        .center_y(Fill)
        .width(Fill)
        .height(Fill)
        .into();
    };

    let all_files = archive
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .count();
    let all_selected = all_files > 0 && state.selected.len() == all_files;
    let header = row![
        column![
            text(
                archive
                    .path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
            )
            .size(28),
            text(if state.locale == Locale::ZhCn {
                format!(
                    "{} · {} 个项目 · 展开后 {}",
                    archive.format,
                    archive.entries.len(),
                    format_bytes(archive.total_size)
                )
            } else {
                format!(
                    "{} · {} entries · {} expanded",
                    archive.format,
                    archive.entries.len(),
                    format_bytes(archive.total_size)
                )
            }),
        ]
        .spacing(5)
        .width(Fill),
        button(state.locale.text(Text::OpenAnother))
            .style(button::secondary)
            .on_press(Message::OpenArchiveDialog),
        button(state.locale.text(Text::TestArchive))
            .style(button::secondary)
            .on_press(Message::TestArchive),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(10);

    let controls = row![
        checkbox(all_selected)
            .label(format!(
                "{} {}",
                state.selected.len(),
                state.locale.text(Text::Selected)
            ))
            .on_toggle(Message::SelectAll),
        space().width(Fill),
        text_input(state.locale.text(Text::PasswordEncrypted), &state.password)
            .secure(true)
            .on_input(Message::PasswordChanged)
            .width(220),
        button(state.locale.text(Text::Reload)).on_press(Message::ReloadArchive),
        pick_list(
            ConflictChoice::ALL.map(|choice| LocalizedConflict {
                choice,
                locale: state.locale
            }),
            Some(LocalizedConflict {
                choice: state.conflict,
                locale: state.locale
            }),
            |value| Message::ConflictChanged(value.choice)
        ),
        button(state.locale.text(Text::ExtractSelected))
            .style(button::primary)
            .on_press_maybe((!state.selected.is_empty()).then_some(Message::Extract)),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(10);

    let filtered_count = browser_entry_count(archive, &state.entry_directory, &state.entry_filter);
    let last_page = filtered_count.saturating_sub(1) / ENTRIES_PER_PAGE;
    let current_page = state.entry_page.min(last_page);
    let browser_controls = row![
        text_input(state.locale.text(Text::Search), &state.entry_filter)
            .on_input(Message::EntryFilterChanged)
            .width(Fill),
        text(format!(
            "{} {} / {} · {}",
            state.locale.text(Text::Page),
            current_page + 1,
            last_page + 1,
            filtered_count
        )),
        button(state.locale.text(Text::Previous))
            .style(button::secondary)
            .on_press_maybe((current_page > 0).then_some(Message::PreviousEntryPage)),
        button(state.locale.text(Text::Next))
            .style(button::secondary)
            .on_press_maybe((current_page < last_page).then_some(Message::NextEntryPage)),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);

    let mut breadcrumbs = row![
        button(choose(state.locale, "Archive root", "归档根目录"))
            .style(button::text)
            .on_press(Message::NavigateArchiveDirectory(PathBuf::new()))
    ]
    .spacing(4)
    .align_y(iced::Alignment::Center);
    for (label, path) in directory_breadcrumbs(&state.entry_directory) {
        breadcrumbs = breadcrumbs.push(text("›")).push(
            button(text(label))
                .style(button::text)
                .on_press(Message::NavigateArchiveDirectory(path)),
        );
    }

    let mut entries = column![
        row![
            text(" ").width(34),
            button(text(sort_header_label(
                state.locale.text(Text::Name),
                EntrySort::Name,
                state.entry_sort,
                state.entry_sort_direction,
            )))
            .style(button::text)
            .on_press(Message::SortEntries(EntrySort::Name))
            .width(Fill),
            button(text(sort_header_label(
                state.locale.text(Text::Original),
                EntrySort::Size,
                state.entry_sort,
                state.entry_sort_direction,
            )))
            .style(button::text)
            .on_press(Message::SortEntries(EntrySort::Size))
            .width(110),
            button(text(sort_header_label(
                state.locale.text(Text::Packed),
                EntrySort::Packed,
                state.entry_sort,
                state.entry_sort_direction,
            )))
            .style(button::text)
            .on_press(Message::SortEntries(EntrySort::Packed))
            .width(110),
            button(text(sort_header_label(
                state.locale.text(Text::Modified),
                EntrySort::Modified,
                state.entry_sort,
                state.entry_sort_direction,
            )))
            .style(button::text)
            .on_press(Message::SortEntries(EntrySort::Modified))
            .width(220),
            text(state.locale.text(Text::Flags)).width(150),
        ]
        .spacing(10),
        rule::horizontal(1),
    ]
    .spacing(4);
    let directory_selections = if state.entry_filter.is_empty() {
        child_directory_selections(archive, &state.entry_directory, &state.selected)
    } else {
        Default::default()
    };
    for entry in browser_entry_page(
        archive,
        &state.entry_directory,
        &state.entry_filter,
        current_page,
        state.entry_sort,
        state.entry_sort_direction,
    ) {
        let path = entry.path.as_ref().to_path_buf();
        let selected = state.selected.contains(&path);
        let selection_path = path.clone();
        let directory_selection = directory_selections.get(&path).copied().unwrap_or_default();
        let directory_selection_path = path.clone();
        let display_path = if state.entry_filter.is_empty() {
            entry.path.file_name().unwrap_or_default().to_string_lossy()
        } else {
            entry.path.to_string_lossy()
        };
        let flags = if entry.encrypted {
            state.locale.text(Text::Locked).to_owned()
        } else if entry.is_directory {
            folder_selection_summary(state.locale, directory_selection)
        } else {
            "—".to_owned()
        };
        entries = entries.push(
            row![
                if entry.is_directory {
                    checkbox(directory_selection.all_selected()).on_toggle_maybe(
                        (directory_selection.total > 0).then_some(move |value| {
                            Message::ToggleDirectory(directory_selection_path.clone(), value)
                        }),
                    )
                } else {
                    checkbox(selected)
                        .on_toggle(move |value| Message::ToggleEntry(selection_path.clone(), value))
                }
                .width(34),
                if entry.is_directory {
                    button(text(format!("▸ {display_path}")))
                        .style(button::text)
                        .on_press(Message::NavigateArchiveDirectory(path.clone()))
                        .width(Fill)
                } else {
                    button(text(format!("• {display_path}")))
                        .style(button::text)
                        .width(Fill)
                },
                text(format_bytes(entry.size)).width(110),
                text(format_bytes(entry.compressed_size)).width(110),
                text(format_archive_modified(
                    state.locale,
                    entry.modified.as_ref()
                ))
                .width(220),
                text(flags).width(150),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .padding([7, 4]),
        );
    }

    column![
        header,
        controls,
        breadcrumbs,
        browser_controls,
        container(scrollable(entries).height(Fill))
            .padding(12)
            .height(Fill)
            .style(container::rounded_box),
    ]
    .spacing(14)
    .height(Fill)
    .into()
}

fn create_view(state: &ZiFile) -> Element<'_, Message> {
    let mut sources = column![].spacing(6);
    if state.create_sources.is_empty() {
        sources = sources.push(text(state.locale.text(Text::NoSources)));
    }
    for (index, source) in state.create_sources.iter().enumerate() {
        sources = sources.push(
            row![
                text(if source.is_dir() { "▣" } else { "•" }),
                text(source.display().to_string()).width(Fill),
                button(state.locale.text(Text::Remove))
                    .style(button::secondary)
                    .on_press(Message::RemoveSource(index)),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        );
    }
    let encryption_supported = state.create_format.capabilities().encryption;
    let source_issue = create_source_issue(state.create_format, &state.create_sources);
    let single_file_format =
        state.create_format.create_input() == Some(CreateInputKind::SingleFile);

    column![
        text(state.locale.text(Text::CreateHeading)).size(32),
        text(state.locale.text(Text::CreateHelp)),
        row![
            button(state.locale.text(Text::AddFiles)).on_press(Message::AddFiles),
            button(state.locale.text(Text::AddFolder))
                .on_press_maybe((!single_file_format).then_some(Message::AddFolder)),
            button(state.locale.text(Text::Clear))
                .style(button::secondary)
                .on_press_maybe(
                    (!state.create_sources.is_empty()).then_some(Message::ClearSources)
                ),
        ]
        .spacing(10),
        container(scrollable(sources).height(Length::Fixed(240.0)))
            .padding(16)
            .width(Fill)
            .style(container::rounded_box),
        container(
            column![
                row![
                    column![
                        text(state.locale.text(Text::Format)).size(13),
                        pick_list(
                            CREATE_FORMATS,
                            Some(state.create_format),
                            Message::CreateFormatChanged
                        )
                    ]
                    .spacing(6)
                    .width(240),
                    column![
                        text(format!(
                            "{} · {}",
                            state.locale.text(Text::CompressionLevel),
                            state.compression_level
                        ))
                        .size(13),
                        slider(
                            0..=9,
                            state.compression_level,
                            Message::CompressionLevelChanged
                        )
                    ]
                    .spacing(8)
                    .width(Fill),
                ]
                .spacing(20),
                text(create_input_help(state.locale, state.create_format)).size(13),
                column![
                    text(if encryption_supported {
                        state.locale.text(Text::PasswordOptional)
                    } else {
                        state.locale.text(Text::PasswordUnavailable)
                    })
                    .size(13),
                    text_input(
                        state.locale.text(Text::NoEncryption),
                        &state.create_password
                    )
                    .secure(true)
                    .on_input_maybe(encryption_supported.then_some(Message::CreatePasswordChanged)),
                ]
                .spacing(6),
            ]
            .spacing(16)
        )
        .padding(18)
        .width(Fill)
        .style(container::rounded_box),
        row![
            space().width(Fill),
            button(state.locale.text(Text::CreateAction))
                .style(button::primary)
                .on_press_maybe(source_issue.is_none().then_some(Message::Create)),
        ],
    ]
    .spacing(14)
    .height(Fill)
    .into()
}

fn create_input_help(locale: Locale, format: ArchiveFormat) -> &'static str {
    match format.create_input() {
        Some(CreateInputKind::FilesAndDirectories) => locale.text(Text::FilesAndFoldersSupported),
        Some(CreateInputKind::SingleFile) => locale.text(Text::SingleFileRequired),
        None => locale.text(Text::FormatCannotCreate),
    }
}

fn create_source_issue_text(locale: Locale, issue: CreateSourceIssue) -> &'static str {
    match issue {
        CreateSourceIssue::MissingSources => locale.text(Text::NoSources),
        CreateSourceIssue::SingleFileRequired => locale.text(Text::SingleFileRequired),
        CreateSourceIssue::UnsupportedFormat => locale.text(Text::FormatCannotCreate),
    }
}

fn nav_button(label: &str, page: Page, active: Page) -> iced::widget::Button<'_, Message> {
    button(label)
        .width(Fill)
        .style(if page == active {
            button::primary
        } else {
            button::text
        })
        .on_press(Message::Navigate(page))
}

fn action_card<'a>(
    heading: &'a str,
    description: &'a str,
    action: &'a str,
    message: Message,
    busy: bool,
) -> Element<'a, Message> {
    container(
        column![
            text(heading).size(24),
            text(description),
            button(action)
                .style(button::primary)
                .on_press_maybe((!busy).then_some(message)),
        ]
        .spacing(14),
    )
    .padding(22)
    .width(Fill)
    .height(190)
    .style(container::rounded_box)
    .into()
}

fn archive_dialog(locale: Locale) -> FileDialog {
    FileDialog::new()
        .set_title(locale.text(Text::OpenDialog))
        .add_filter(
            locale.text(Text::SupportedArchives),
            OPEN_ARCHIVE_EXTENSIONS,
        )
        .add_filter(locale.text(Text::AllFiles), &["*"])
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}

const fn choose<'a>(locale: Locale, english: &'a str, chinese: &'a str) -> &'a str {
    match locale {
        Locale::En => english,
        Locale::ZhCn => chinese,
    }
}

fn append_unique(destination: &mut Vec<PathBuf>, paths: Vec<PathBuf>) {
    for path in paths {
        if !destination.contains(&path) {
            destination.push(path);
        }
    }
}

fn format_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else {
        format!("{value:.1} {}", UNITS[unit])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zifile_core::ArchiveEntryInfo;
    use zifile_desktop::entry_view::{filtered_entry_count, sorted_filtered_entry_page};

    #[test]
    fn create_input_guidance_is_bilingual_and_matches_capabilities() {
        assert_eq!(
            create_input_help(Locale::En, ArchiveFormat::Tar),
            "This format accepts files and folders."
        );
        assert!(create_input_help(Locale::ZhCn, ArchiveFormat::Brotli).contains("TAR 组合格式"));
        assert_eq!(
            create_source_issue_text(Locale::ZhCn, CreateSourceIssue::UnsupportedFormat),
            "此格式不支持创建。"
        );
    }

    #[test]
    fn large_archive_filtering_keeps_rendered_page_bounded() {
        let entries = (0..100_000)
            .map(|index| ArchiveEntryInfo {
                path: PathBuf::from(format!("folder/file-{index:06}.txt")),
                size: 1,
                compressed_size: 1,
                is_directory: false,
                encrypted: false,
                modified: None,
            })
            .collect();
        let archive = ArchiveInfo {
            path: PathBuf::from("large.zip"),
            format: ArchiveFormat::Zip,
            entries,
            total_size: 100_000,
            compressed_size: 100_000,
        };
        assert_eq!(filtered_entry_count(&archive, "file-09"), 10_000);
        let rendered = sorted_filtered_entry_page(
            &archive,
            "file-09",
            0,
            EntrySort::Name,
            SortDirection::Ascending,
        )
        .len();
        assert_eq!(rendered, ENTRIES_PER_PAGE);
    }

    #[test]
    fn sort_header_identifies_the_active_direction() {
        assert_eq!(
            sort_header_label(
                "Modified",
                EntrySort::Modified,
                EntrySort::Modified,
                SortDirection::Descending,
            ),
            "Modified ↓"
        );
        assert_eq!(
            sort_header_label(
                "Packed",
                EntrySort::Packed,
                EntrySort::Name,
                SortDirection::Ascending,
            ),
            "Packed"
        );
    }

    #[test]
    fn directory_navigation_resets_filter_and_page() {
        let mut state = ZiFile {
            entry_filter: "readme".to_owned(),
            entry_page: 7,
            ..ZiFile::default()
        };
        drop(update(
            &mut state,
            Message::NavigateArchiveDirectory(PathBuf::from("docs/reference")),
        ));
        assert_eq!(state.entry_directory, PathBuf::from("docs/reference"));
        assert!(state.entry_filter.is_empty());
        assert_eq!(state.entry_page, 0);
    }

    #[test]
    fn directory_toggle_selects_and_clears_only_descendant_files() {
        let archive = ArchiveInfo {
            path: PathBuf::from("folders.zip"),
            format: ArchiveFormat::Zip,
            entries: vec![
                ArchiveEntryInfo {
                    path: PathBuf::from("docs/a.txt"),
                    size: 1,
                    compressed_size: 1,
                    is_directory: false,
                    encrypted: false,
                    modified: None,
                },
                ArchiveEntryInfo {
                    path: PathBuf::from("other.txt"),
                    size: 1,
                    compressed_size: 1,
                    is_directory: false,
                    encrypted: false,
                    modified: None,
                },
            ],
            total_size: 2,
            compressed_size: 2,
        };
        let mut state = ZiFile {
            archive: Some(archive),
            selected: HashSet::from([PathBuf::from("other.txt")]),
            ..ZiFile::default()
        };
        drop(update(
            &mut state,
            Message::ToggleDirectory(PathBuf::from("docs"), true),
        ));
        assert!(state.selected.contains(Path::new("docs/a.txt")));
        assert!(state.selected.contains(Path::new("other.txt")));
        drop(update(
            &mut state,
            Message::ToggleDirectory(PathBuf::from("docs"), false),
        ));
        assert!(!state.selected.contains(Path::new("docs/a.txt")));
        assert!(state.selected.contains(Path::new("other.txt")));
        assert_eq!(
            folder_selection_summary(
                Locale::ZhCn,
                DirectorySelection {
                    selected: 1,
                    total: 3
                }
            ),
            "已选 1/3"
        );
    }

    #[test]
    fn extract_here_queues_extraction_after_listing_succeeds() {
        let archive_path = PathBuf::from(r"C:\archives\sample.zip");
        let destination = PathBuf::from(r"C:\archives\sample");
        let archive = ArchiveInfo {
            path: archive_path.clone(),
            format: ArchiveFormat::Zip,
            entries: vec![ArchiveEntryInfo {
                path: PathBuf::from("hello.txt"),
                size: 5,
                compressed_size: 5,
                is_directory: false,
                encrypted: false,
                modified: None,
            }],
            total_size: 5,
            compressed_size: 5,
        };
        let mut state = ZiFile {
            automatic_extract_destination: Some(destination.clone()),
            ..ZiFile::default()
        };
        drop(begin_load(&mut state, archive_path));
        assert_eq!(state.operations.active_id(), Some(1));

        drop(update(&mut state, Message::ArchiveLoaded(1, Ok(archive))));

        assert_eq!(state.operations.active_id(), Some(2));
        assert!(state.busy);
        assert!(state.automatic_extract_destination.is_none());
        assert!(state.status.contains(&destination.display().to_string()));
        assert_eq!(state.selected, HashSet::from([PathBuf::from("hello.txt")]));
    }
}
