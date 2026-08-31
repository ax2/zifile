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
};
use zifile_worker_protocol::WorkerRequest;

use i18n::{
    Locale, Text, archive_empty_state_description, archive_filter_summary, archive_no_matches,
    create_source_removed_status, create_source_summary, create_sources_added_status,
    create_sources_cleared_status, format_archive_modified, format_worker_error,
    worker_error_may_require_password,
};
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
use zifile_desktop::{
    append_unique_paths as append_unique, create_passwords_match, ensure_archive_extension,
    invert_archive_file_selection, is_openable_archive_path, reveal_in_file_manager,
};

const CREATE_FORMATS: [ArchiveFormat; 15] = ArchiveFormat::CREATABLE;
const ARCHIVE_TABLE_MIN_WIDTH: f32 = 1_040.0;
const ARCHIVE_SEARCH_ID: &str = "archive-search";

pub fn main() -> iced::Result {
    if std::env::args_os().any(|argument| argument == zifile_worker::WORKER_MODE_ARGUMENT) {
        zifile_worker::run_process();
        return Ok(());
    }
    iced::application(initialize, update, view)
        .title(title)
        .theme(theme)
        .subscription(subscription)
        .window(window_settings())
        .antialiasing(true)
        .run()
}

fn window_settings() -> iced::window::Settings {
    iced::window::Settings {
        size: iced::Size::new(1_180.0, 780.0),
        min_size: Some(iced::Size::new(920.0, 620.0)),
        position: iced::window::Position::Centered,
        ..Default::default()
    }
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
    About,
}

#[derive(Debug)]
struct ZiFile {
    page: Page,
    archive: Option<ArchiveInfo>,
    pending_archive: Option<PathBuf>,
    pending_archive_requires_password: bool,
    automatic_extract_destination: Option<PathBuf>,
    completed_output: Option<PathBuf>,
    selected: HashSet<PathBuf>,
    entry_directory: PathBuf,
    entry_filter: String,
    entry_page: usize,
    entry_sort: EntrySort,
    entry_sort_direction: SortDirection,
    password: String,
    password_visible: bool,
    conflict: ConflictChoice,
    create_sources: Vec<PathBuf>,
    create_format: ArchiveFormat,
    create_password: String,
    create_password_confirmation: String,
    create_password_confirmation_touched: bool,
    create_password_visible: bool,
    compression_level: u8,
    status: String,
    status_kind: StatusKind,
    busy: bool,
    dialog_open: bool,
    cancellation: Option<CancellationToken>,
    progress: Option<OperationProgress>,
    operations: OperationQueue<QueuedOperation>,
    recent_archives: Vec<PathBuf>,
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
            pending_archive_requires_password: false,
            automatic_extract_destination: None,
            completed_output: None,
            selected: HashSet::new(),
            entry_directory: PathBuf::new(),
            entry_filter: String::new(),
            entry_page: 0,
            entry_sort: EntrySort::default(),
            entry_sort_direction: SortDirection::default(),
            password: String::new(),
            password_visible: false,
            conflict: ConflictChoice::Rename,
            create_sources: Vec::new(),
            create_format: ArchiveFormat::Zip,
            create_password: String::new(),
            create_password_confirmation: String::new(),
            create_password_confirmation_touched: false,
            create_password_visible: false,
            compression_level: 6,
            status: settings.locale.text(Text::Ready).to_owned(),
            status_kind: StatusKind::Informational,
            busy: false,
            dialog_open: false,
            cancellation: None,
            progress: None,
            operations: OperationQueue::default(),
            recent_archives: settings.recent_archives,
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
    OpenArchiveDialogFinished(Option<PathBuf>),
    OpenRecentArchive(PathBuf),
    RemoveRecentArchive(PathBuf),
    ClearRecentArchives,
    ArchiveLoaded(u64, Result<ArchiveInfo, String>),
    PasswordChanged(String),
    ArchivePasswordVisibilityChanged(bool),
    ReloadArchive,
    CloseArchive,
    RevealArchive,
    RevealCompletedOutput,
    ToggleEntry(PathBuf, bool),
    ToggleDirectory(PathBuf, bool),
    SelectAll(bool),
    InvertSelection,
    NavigateArchiveDirectory(PathBuf),
    EntryFilterChanged(String),
    ClearArchiveFilter,
    SortEntries(EntrySort),
    CopyChecksum(String),
    PreviousEntryPage,
    NextEntryPage,
    ConflictChanged(ConflictChoice),
    Extract,
    ExtractAll,
    ExtractToNamedFolder,
    ExtractDialogFinished(Option<PathBuf>, bool),
    ExtractFinished(u64, Option<PathBuf>, Result<OperationSummary, String>),
    TestArchive,
    TestFinished(u64, Result<ArchiveInfo, String>),
    AddFiles,
    AddFilesDialogFinished(Option<Vec<PathBuf>>),
    AddFolder,
    AddFolderDialogFinished(Option<PathBuf>),
    RemoveSource(usize),
    ClearSources,
    CreateFormatChanged(ArchiveFormat),
    CreatePasswordChanged(String),
    CreatePasswordConfirmationChanged(String),
    CreatePasswordVisibilityChanged(bool),
    CompressionLevelChanged(u8),
    Create,
    CreateDialogFinished(Option<PathBuf>),
    CreateFinished(u64, Option<PathBuf>, Result<OperationSummary, String>),
    ClearQueued,
    Cancel,
    ProgressTick,
    FileDropped(PathBuf),
    FileDropClassified(PathBuf, bool),
    KeyboardShortcut(Shortcut),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shortcut {
    Open,
    Create,
    Reload,
    Search,
    Close,
    About,
    SelectAll,
    InvertSelection,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OperationKind {
    List,
    Test,
    Extract,
    Create,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusKind {
    Informational,
    Error,
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

fn set_status(state: &mut ZiFile, status: impl Into<String>) {
    state.status = status.into();
    state.status_kind = StatusKind::Informational;
}

fn set_error(state: &mut ZiFile, status: impl Into<String>) {
    state.status = status.into();
    state.status_kind = StatusKind::Error;
}

fn status_container_style(theme: &Theme, kind: StatusKind) -> container::Style {
    match kind {
        StatusKind::Informational => container::rounded_box(theme),
        StatusKind::Error => container::danger(theme),
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
            set_status(state, state.locale.text(Text::Ready));
            save_settings(state);
        }
        Message::OpenArchiveDialog => {
            if state.dialog_open {
                return Task::none();
            }
            state.dialog_open = true;
            let dialog = archive_dialog(state.locale);
            return Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || dialog.pick_file())
                        .await
                        .unwrap_or(None)
                },
                Message::OpenArchiveDialogFinished,
            );
        }
        Message::OpenArchiveDialogFinished(path) => {
            state.dialog_open = false;
            if let Some(path) = path {
                clear_archive_password(state);
                state.automatic_extract_destination = None;
                return begin_load(state, path);
            }
        }
        Message::OpenRecentArchive(path) => {
            if !state.busy {
                clear_archive_password(state);
                state.automatic_extract_destination = None;
                return begin_load(state, path);
            }
        }
        Message::RemoveRecentArchive(path) => {
            if !state.busy {
                let mut settings = AppSettings {
                    locale: state.locale,
                    dark: state.dark,
                    recent_archives: std::mem::take(&mut state.recent_archives),
                };
                settings.remove_recent_archive(&path);
                state.recent_archives = settings.recent_archives;
                save_settings(state);
                set_status(
                    state,
                    choose(state.locale, "Recent archive removed", "已移除最近打开记录"),
                );
            }
        }
        Message::ClearRecentArchives => {
            if !state.busy {
                state.recent_archives.clear();
                save_settings(state);
                set_status(
                    state,
                    choose(
                        state.locale,
                        "Recent archives cleared",
                        "最近打开记录已清空",
                    ),
                );
            }
        }
        Message::ArchiveLoaded(id, result) => {
            if state.operations.active_id() != Some(id) {
                return Task::none();
            }
            state.busy = false;
            state.cancellation = None;
            state.progress = None;
            match result {
                Ok(archive) => {
                    let recent_path = archive.path.clone();
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
                    let status = if state.locale == Locale::ZhCn {
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
                    set_status(state, status);
                    state.archive = Some(archive);
                    record_recent_archive(state, recent_path);
                    state.pending_archive = None;
                    state.pending_archive_requires_password = false;
                    state.page = Page::Archive;
                    if let Some(destination) = state.automatic_extract_destination.take()
                        && let Some(operation) = extract_operation(state, destination, true)
                    {
                        drop(submit_operation(state, operation));
                    }
                }
                Err(error) => {
                    state.pending_archive_requires_password =
                        worker_error_may_require_password(&error);
                    set_error(
                        state,
                        format!(
                            "{}: {}",
                            choose(state.locale, "Open failed", "打开失败"),
                            format_worker_error(state.locale, &error)
                        ),
                    );
                }
            }
            return continue_queue(state, id);
        }
        Message::PasswordChanged(password) => {
            state.password = password;
            if state.password.is_empty() {
                state.password_visible = false;
            }
        }
        Message::ArchivePasswordVisibilityChanged(visible) => {
            state.password_visible = visible;
        }
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
        Message::CloseArchive => {
            if !state.busy && state.archive.is_some() {
                close_archive(state);
            }
        }
        Message::RevealArchive => {
            let Some(path) = state.archive.as_ref().map(|archive| archive.path.clone()) else {
                return Task::none();
            };
            match reveal_in_file_manager(&path) {
                Ok(()) => set_status(state, state.locale.text(Text::RevealedInExplorer)),
                Err(_) => set_error(state, state.locale.text(Text::RevealInExplorerFailed)),
            }
        }
        Message::RevealCompletedOutput => {
            let Some(path) = state.completed_output.clone() else {
                return Task::none();
            };
            match reveal_in_file_manager(&path) {
                Ok(()) => set_status(state, state.locale.text(Text::OutputRevealed)),
                Err(_) => set_error(state, state.locale.text(Text::RevealInExplorerFailed)),
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
            let status = if selected {
                match state.locale {
                    Locale::En => format!("Selected all {} archive files", state.selected.len()),
                    Locale::ZhCn => format!("已选择全部 {} 个归档文件", state.selected.len()),
                }
            } else {
                choose(state.locale, "Selection cleared", "已清除选择").to_owned()
            };
            set_status(state, status);
        }
        Message::InvertSelection => {
            if let Some(archive) = &state.archive {
                let selected = invert_archive_file_selection(archive, &mut state.selected);
                set_status(
                    state,
                    match state.locale {
                        Locale::En => format!("Selection inverted; {selected} files selected"),
                        Locale::ZhCn => format!("已反选；当前选择 {selected} 个文件"),
                    },
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
        Message::ClearArchiveFilter => {
            state.entry_filter.clear();
            state.entry_page = 0;
        }
        Message::CopyChecksum(checksum) => {
            set_status(state, state.locale.text(Text::ChecksumCopied));
            return iced::clipboard::write(checksum);
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
            return extraction_dialog_task(state, false);
        }
        Message::ExtractAll => {
            return extraction_dialog_task(state, true);
        }
        Message::ExtractToNamedFolder => {
            return quick_extraction_operation(state)
                .map_or_else(Task::none, |operation| submit_operation(state, operation));
        }
        Message::ExtractDialogFinished(destination, extract_all) => {
            state.dialog_open = false;
            if let Some(destination) = destination {
                return extract_operation(state, destination, extract_all)
                    .map_or_else(Task::none, |operation| submit_operation(state, operation));
            }
        }
        Message::ExtractFinished(id, output, result) => {
            if state.operations.active_id() != Some(id) {
                return Task::none();
            }
            state.busy = false;
            state.cancellation = None;
            state.progress = None;
            let failed = result.is_err();
            state.completed_output = (!failed).then_some(output).flatten();
            let status = match result {
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
                    "{}: {}",
                    choose(state.locale, "Extraction failed", "解压失败"),
                    format_worker_error(state.locale, &error)
                ),
            };
            if failed {
                set_error(state, status);
            } else {
                set_status(state, status);
            }
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
            if state.operations.active_id() != Some(id) {
                return Task::none();
            }
            state.busy = false;
            state.cancellation = None;
            state.progress = None;
            let failed = result.is_err();
            let status = match result {
                Ok(info) => {
                    let checksum_count = info
                        .entries
                        .iter()
                        .filter(|entry| entry.checksum.is_some())
                        .count();
                    let status = if state.locale == Locale::ZhCn {
                        format!(
                            "压缩文件完好 · {} 个项目 · {} 个校验和 · {}",
                            info.entries.len(),
                            checksum_count,
                            format_bytes(info.total_size)
                        )
                    } else {
                        format!(
                            "Archive is healthy · {} entries · {} SHA-256 checksums · {}",
                            info.entries.len(),
                            checksum_count,
                            format_bytes(info.total_size)
                        )
                    };
                    state.archive = Some(info);
                    status
                }
                Err(error) => format!(
                    "{}: {}",
                    choose(state.locale, "Integrity test failed", "完整性校验失败"),
                    format_worker_error(state.locale, &error)
                ),
            };
            if failed {
                set_error(state, status);
            } else {
                set_status(state, status);
            }
            return continue_queue(state, id);
        }
        Message::AddFiles => {
            if state.dialog_open {
                return Task::none();
            }
            let single_file_format =
                state.create_format.create_input() == Some(CreateInputKind::SingleFile);
            state.dialog_open = true;
            let dialog = FileDialog::new().set_title(state.locale.text(Text::AddFilesDialog));
            return Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || {
                        if single_file_format {
                            dialog.pick_file().map(|path| vec![path])
                        } else {
                            dialog.pick_files()
                        }
                    })
                    .await
                    .unwrap_or(None)
                },
                Message::AddFilesDialogFinished,
            );
        }
        Message::AddFilesDialogFinished(paths) => {
            state.dialog_open = false;
            if let Some(paths) = paths {
                let added = append_unique(&mut state.create_sources, paths);
                set_status(
                    state,
                    create_sources_added_status(state.locale, added, state.create_sources.len()),
                );
                state.page = Page::Create;
            }
        }
        Message::AddFolder => {
            if state.create_format.create_input() == Some(CreateInputKind::SingleFile) {
                set_error(state, state.locale.text(Text::SingleFileRequired));
            } else if !state.dialog_open {
                state.dialog_open = true;
                let dialog = FileDialog::new().set_title(state.locale.text(Text::AddFolderDialog));
                return Task::perform(
                    async move {
                        tokio::task::spawn_blocking(move || dialog.pick_folder())
                            .await
                            .unwrap_or(None)
                    },
                    Message::AddFolderDialogFinished,
                );
            }
        }
        Message::AddFolderDialogFinished(path) => {
            state.dialog_open = false;
            if let Some(path) = path {
                let added = append_unique(&mut state.create_sources, vec![path]);
                set_status(
                    state,
                    create_sources_added_status(state.locale, added, state.create_sources.len()),
                );
                state.page = Page::Create;
            }
        }
        Message::RemoveSource(index) => {
            if index < state.create_sources.len() {
                let path = state.create_sources.remove(index);
                set_status(
                    state,
                    create_source_removed_status(
                        state.locale,
                        &path.to_string_lossy(),
                        state.create_sources.len(),
                    ),
                );
            }
        }
        Message::ClearSources => {
            let cleared = state.create_sources.len();
            state.create_sources.clear();
            set_status(state, create_sources_cleared_status(state.locale, cleared));
        }
        Message::CreateFormatChanged(format) => {
            apply_create_format(state, format);
        }
        Message::CreatePasswordChanged(password) => {
            state.create_password = password;
            if state.create_password.is_empty() {
                state.create_password_visible = false;
            }
        }
        Message::CreatePasswordConfirmationChanged(password) => {
            state.create_password_confirmation = password;
            state.create_password_confirmation_touched = true;
        }
        Message::CreatePasswordVisibilityChanged(visible) => {
            state.create_password_visible = visible;
        }
        Message::CompressionLevelChanged(level) => {
            state.compression_level = state.create_format.clamp_compression_level(level);
        }
        Message::Create => {
            if state.dialog_open {
                return Task::none();
            }
            if let Some(issue) = create_source_issue(state.create_format, &state.create_sources) {
                set_error(state, create_source_issue_text(state.locale, issue));
                return Task::none();
            }
            if !create_passwords_match(&state.create_password, &state.create_password_confirmation)
            {
                set_error(state, state.locale.text(Text::PasswordMismatch));
                return Task::none();
            }
            let extension = state.create_format.canonical_extension();
            state.dialog_open = true;
            let dialog = FileDialog::new()
                .set_title(state.locale.text(Text::CreateDialog))
                .add_filter(state.create_format.to_string(), &[extension])
                .set_file_name(format!("archive.{extension}"));
            return Task::perform(
                async move {
                    tokio::task::spawn_blocking(move || dialog.save_file())
                        .await
                        .unwrap_or(None)
                },
                Message::CreateDialogFinished,
            );
        }
        Message::CreateDialogFinished(destination) => {
            state.dialog_open = false;
            let Some(destination) = destination else {
                return Task::none();
            };
            let destination = ensure_archive_extension(destination, state.create_format);
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
        Message::CreateFinished(id, output, result) => {
            if state.operations.active_id() != Some(id) {
                return Task::none();
            }
            state.busy = false;
            state.cancellation = None;
            state.progress = None;
            let failed = result.is_err();
            state.completed_output = (!failed).then_some(output).flatten();
            let status = match result {
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
                    "{}: {}",
                    choose(state.locale, "Creation failed", "创建失败"),
                    format_worker_error(state.locale, &error)
                ),
            };
            if failed {
                set_error(state, status);
            } else {
                set_status(state, status);
            }
            return continue_queue(state, id);
        }
        Message::ClearQueued => {
            let cleared = state.operations.clear_pending().len();
            set_status(
                state,
                match state.locale {
                    Locale::En => format!("Cleared {cleared} queued operations"),
                    Locale::ZhCn => format!("已清除 {cleared} 个排队操作"),
                },
            );
        }
        Message::Cancel => {
            if let Some(cancellation) = &state.cancellation {
                cancellation.cancel();
                set_status(
                    state,
                    choose(
                        state.locale,
                        "Cancelling safely after the current block…",
                        "正在当前数据块结束后安全取消…",
                    ),
                );
            }
        }
        Message::ProgressTick => {}
        Message::FileDropped(path) => {
            let probe_path = path.clone();
            return Task::perform(
                async move { (path, is_openable_archive_path(&probe_path)) },
                |(path, openable)| Message::FileDropClassified(path, openable),
            );
        }
        Message::FileDropClassified(path, openable) => {
            if openable {
                clear_archive_password(state);
                state.automatic_extract_destination = None;
                return begin_load(state, path);
            } else if path.exists() {
                let added = append_unique(&mut state.create_sources, vec![path]);
                state.page = Page::Create;
                set_status(
                    state,
                    create_sources_added_status(state.locale, added, state.create_sources.len()),
                );
            }
        }
        Message::KeyboardShortcut(shortcut) => match shortcut {
            Shortcut::Open => return update(state, Message::OpenArchiveDialog),
            Shortcut::Create => state.page = Page::Create,
            Shortcut::Reload => return update(state, Message::ReloadArchive),
            Shortcut::Search if state.archive.is_some() => {
                state.page = Page::Archive;
                return iced::widget::operation::focus(ARCHIVE_SEARCH_ID);
            }
            Shortcut::Close if state.archive.is_some() && !state.busy => {
                return update(state, Message::CloseArchive);
            }
            Shortcut::About => state.page = Page::About,
            Shortcut::SelectAll if state.page == Page::Archive => {
                return update(state, Message::SelectAll(true));
            }
            Shortcut::InvertSelection if state.page == Page::Archive && state.archive.is_some() => {
                return update(state, Message::InvertSelection);
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
            default_shortcut(&key, physical_key, modifiers).map(Message::KeyboardShortcut)
        }
        _ => None,
    }
}

fn default_shortcut(
    key: &iced::keyboard::Key,
    physical_key: iced::keyboard::key::Physical,
    modifiers: iced::keyboard::Modifiers,
) -> Option<Shortcut> {
    if modifiers == iced::keyboard::Modifiers::NONE {
        if *key == iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape) {
            return Some(Shortcut::Cancel);
        }
        if *key == iced::keyboard::Key::Named(iced::keyboard::key::Named::F1) {
            return Some(Shortcut::About);
        }
        return None;
    }
    if modifiers != iced::keyboard::Modifiers::CTRL {
        return None;
    }
    match key
        .to_latin(physical_key)
        .map(|value| value.to_ascii_lowercase())
    {
        Some('o') => Some(Shortcut::Open),
        Some('n') => Some(Shortcut::Create),
        Some('r') => Some(Shortcut::Reload),
        Some('f') => Some(Shortcut::Search),
        Some('w') => Some(Shortcut::Close),
        Some('a') => Some(Shortcut::SelectAll),
        Some('i') => Some(Shortcut::InvertSelection),
        _ => None,
    }
}

fn close_archive(state: &mut ZiFile) {
    state.archive = None;
    state.pending_archive = None;
    state.pending_archive_requires_password = false;
    state.automatic_extract_destination = None;
    state.selected.clear();
    state.entry_directory.clear();
    state.entry_filter.clear();
    state.entry_page = 0;
    state.entry_sort = EntrySort::default();
    state.entry_sort_direction = SortDirection::default();
    clear_archive_password(state);
    state.page = Page::Home;
    set_status(state, state.locale.text(Text::ArchiveClosed));
}

fn save_settings(state: &mut ZiFile) {
    if let Err(error) = (AppSettings {
        locale: state.locale,
        dark: state.dark,
        recent_archives: state.recent_archives.clone(),
    }
    .save())
    {
        set_error(
            state,
            format!(
                "{}: {error}",
                state.locale.text(Text::PreferencesSaveFailed)
            ),
        );
    }
}

fn record_recent_archive(state: &mut ZiFile, path: PathBuf) {
    let mut settings = AppSettings {
        locale: state.locale,
        dark: state.dark,
        recent_archives: std::mem::take(&mut state.recent_archives),
    };
    settings.record_recent_archive(path);
    state.recent_archives = settings.recent_archives;
    save_settings(state);
}

fn begin_load(state: &mut ZiFile, path: PathBuf) -> Task<Message> {
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

fn extraction_dialog_task(state: &mut ZiFile, extract_all: bool) -> Task<Message> {
    if state.dialog_open {
        return Task::none();
    }
    let Some(archive) = state.archive.as_ref() else {
        return Task::none();
    };
    let default_folder = archive
        .path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(archive.path.file_stem().unwrap_or_default());
    state.dialog_open = true;
    let dialog = FileDialog::new()
        .set_title(state.locale.text(Text::ChooseExtractionFolder))
        .set_directory(default_folder.parent().unwrap_or_else(|| Path::new(".")));
    Task::perform(
        async move {
            tokio::task::spawn_blocking(move || dialog.pick_folder())
                .await
                .unwrap_or(None)
        },
        move |destination| Message::ExtractDialogFinished(destination, extract_all),
    )
}

fn extract_operation(
    state: &ZiFile,
    destination: PathBuf,
    extract_all: bool,
) -> Option<QueuedOperation> {
    let archive = state.archive.as_ref()?;
    let file_count = archive
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .count();
    let selected_paths = if extract_all || state.selected.len() == file_count {
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

fn quick_extraction_operation(state: &ZiFile) -> Option<QueuedOperation> {
    let destination = startup::extraction_destination(&state.archive.as_ref()?.path);
    extract_operation(state, destination, true)
}

fn submit_operation(state: &mut ZiFile, operation: QueuedOperation) -> Task<Message> {
    let kind = operation.kind;
    match state.operations.submit(operation) {
        Ok(Submission::Start(job)) => {
            clear_submitted_create_password(
                &mut state.create_password,
                &mut state.create_password_confirmation,
                &mut state.create_password_confirmation_touched,
                &mut state.create_password_visible,
                kind,
                true,
            );
            start_operation(state, job)
        }
        Ok(Submission::Queued { position, .. }) => {
            clear_submitted_create_password(
                &mut state.create_password,
                &mut state.create_password_confirmation,
                &mut state.create_password_confirmation_touched,
                &mut state.create_password_visible,
                kind,
                true,
            );
            set_status(
                state,
                match state.locale {
                    Locale::En => format!(
                        "Queued operation at position {position}; the current operation continues"
                    ),
                    Locale::ZhCn => format!("操作已排队，位置 {position}；当前操作继续运行"),
                },
            );
            Task::none()
        }
        Err(error) => {
            clear_submitted_create_password(
                &mut state.create_password,
                &mut state.create_password_confirmation,
                &mut state.create_password_confirmation_touched,
                &mut state.create_password_visible,
                kind,
                false,
            );
            set_error(
                state,
                match state.locale {
                    Locale::En => format!("Operation queue is full (maximum {})", error.capacity),
                    Locale::ZhCn => format!("操作队列已满（最多 {} 个）", error.capacity),
                },
            );
            Task::none()
        }
    }
}

fn clear_submitted_create_password(
    password: &mut String,
    confirmation: &mut String,
    confirmation_touched: &mut bool,
    visible: &mut bool,
    kind: OperationKind,
    accepted: bool,
) {
    if accepted && kind == OperationKind::Create {
        password.clear();
        confirmation.clear();
        *confirmation_touched = false;
        *visible = false;
    }
}

fn clear_archive_password(state: &mut ZiFile) {
    state.password.clear();
    state.password_visible = false;
}

fn start_operation(state: &mut ZiFile, job: Job<QueuedOperation>) -> Task<Message> {
    let Job { id, payload } = job;
    let QueuedOperation {
        kind,
        request,
        status,
        archive_path,
    } = payload;
    let completed_output = operation_output_path(&request);
    if let Some(path) = archive_path {
        state.archive = None;
        state.pending_archive = Some(path);
        state.pending_archive_requires_password = false;
        state.selected.clear();
        state.page = Page::Archive;
    }
    let cancellation = CancellationToken::default();
    let progress = OperationProgress::default();
    state.busy = true;
    state.completed_output = None;
    state.cancellation = Some(cancellation.clone());
    state.progress = Some(progress.clone());
    set_status(state, status);
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
            move |result| Message::ExtractFinished(id, completed_output, result),
        ),
        OperationKind::Create => Task::perform(
            async move { run_worker(request, progress, cancellation).and_then(expect_summary) },
            move |result| Message::CreateFinished(id, completed_output, result),
        ),
    }
}

fn operation_output_path(request: &WorkerRequest) -> Option<PathBuf> {
    match request {
        WorkerRequest::Extract { destination, .. } | WorkerRequest::Create { destination, .. } => {
            Some(destination.clone())
        }
        WorkerRequest::List { .. } | WorkerRequest::Test { .. } => None,
    }
}

fn continue_queue(state: &mut ZiFile, completed_id: u64) -> Task<Message> {
    let next = match state.operations.complete(completed_id) {
        Ok(next) => next,
        Err(error) => {
            set_error(state, format!("Internal operation queue error: {error}"));
            return Task::none();
        }
    };
    match next {
        Some(next) => start_operation(state, next),
        None => {
            state.busy = false;
            state.cancellation = None;
            state.progress = None;
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
            nav_button(state.locale.text(Text::About), Page::About, state.page),
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
        .padding(24),
    )
    .width(232)
    .height(Fill)
    .style(container::secondary);

    let page = match state.page {
        Page::Home => home_view(state),
        Page::Archive => archive_view(state),
        Page::Create => create_view(state),
        Page::About => about_view(state),
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
    let status_kind = state.status_kind;
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
                button(state.locale.text(Text::RevealOutput))
                    .style(button::secondary)
                    .on_press_maybe(
                        (!state.busy && state.completed_output.is_some())
                            .then_some(Message::RevealCompletedOutput)
                    ),
            ]
            .spacing(12),
            progress,
        ]
        .spacing(6),
    )
    .padding([12, 20])
    .width(Fill)
    .style(move |theme| status_container_style(theme, status_kind));

    row![navigation, column![page, status].spacing(16).padding(24)]
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
    let recent_archives: Element<'_, Message> = if state.recent_archives.is_empty() {
        container(text(choose(
            state.locale,
            "Archives you successfully open will appear here.",
            "成功打开的压缩文件会显示在这里。",
        )))
        .padding(18)
        .width(Fill)
        .style(container::rounded_box)
        .into()
    } else {
        let entries = state
            .recent_archives
            .iter()
            .fold(column![].spacing(8), |entries, path| {
                entries.push(
                    row![
                        button(text(recent_archive_label(path)).width(Fill))
                            .style(button::secondary)
                            .width(Fill)
                            .on_press_maybe(
                                (!state.busy).then_some(Message::OpenRecentArchive(path.clone())),
                            ),
                        button(choose(state.locale, "Remove", "移除"))
                            .style(button::secondary)
                            .on_press_maybe(
                                (!state.busy).then_some(Message::RemoveRecentArchive(path.clone())),
                            ),
                    ]
                    .spacing(8),
                )
            });
        container(
            column![
                row![
                    text(choose(state.locale, "Recent archives", "最近打开"))
                        .size(20)
                        .width(Fill),
                    button(choose(state.locale, "Clear", "清空"))
                        .style(button::secondary)
                        .on_press_maybe((!state.busy).then_some(Message::ClearRecentArchives)),
                ]
                .align_y(iced::Alignment::Center),
                entries,
            ]
            .spacing(12),
        )
        .padding(18)
        .width(Fill)
        .style(container::rounded_box)
        .into()
    };
    column![
        text(state.locale.text(Text::Hero)).size(42),
        text(state.locale.text(Text::HeroSub)).size(18),
        row![open, create].spacing(16),
        recent_archives,
        container(
            column![
                text(state.locale.text(Text::Privacy)).size(20),
                text(state.locale.text(Text::PrivacyDescription)),
            ]
            .spacing(8)
        )
        .padding(22)
        .width(Fill)
        .style(container::rounded_box),
    ]
    .spacing(26)
    .width(Fill)
    .into()
}

fn recent_archive_label(path: &Path) -> String {
    let name = path.file_name().map_or_else(
        || path.as_os_str().to_string_lossy(),
        |name| name.to_string_lossy(),
    );
    let parent = path
        .parent()
        .map(|parent| parent.to_string_lossy())
        .unwrap_or_default();
    if parent.is_empty() {
        name.into_owned()
    } else {
        format!("{name}  ·  {parent}")
    }
}

fn about_view(state: &ZiFile) -> Element<'_, Message> {
    let locale = state.locale;
    let detail = |label: &'static str, value: String| {
        container(
            column![text(label).size(13), text(value).size(18)]
                .spacing(4)
                .width(Fill),
        )
        .padding(18)
        .width(Fill)
        .style(container::rounded_box)
    };
    let shortcut = |keys: &'static str, description: &'static str| {
        row![
            container(text(keys).size(14))
                .padding([6, 10])
                .style(container::bordered_box),
            text(description).size(15).width(Fill),
        ]
        .spacing(12)
        .align_y(iced::Alignment::Center)
    };
    column![
        text(locale.text(Text::AboutHeading)).size(32),
        text(locale.text(Text::AboutDescription)).size(17),
        row![
            detail(
                locale.text(Text::Version),
                env!("CARGO_PKG_VERSION").to_owned()
            ),
            detail(locale.text(Text::License), "MIT".to_owned()),
            detail(
                locale.text(Text::SupportedFormatFamilies),
                ArchiveFormat::ALL.len().to_string()
            ),
        ]
        .spacing(14),
        detail(
            locale.text(Text::ProjectWebsite),
            "https://github.com/ax2/zifile".to_owned()
        ),
        container(
            column![
                text(locale.text(Text::KeyboardShortcuts)).size(20),
                shortcut("Ctrl+O", locale.text(Text::ShortcutOpen)),
                shortcut("Ctrl+N", locale.text(Text::ShortcutCreate)),
                shortcut("Ctrl+R", locale.text(Text::ShortcutReload)),
                shortcut("Ctrl+F", locale.text(Text::ShortcutSearch)),
                shortcut("Ctrl+W", locale.text(Text::ShortcutClose)),
                shortcut("Ctrl+A", locale.text(Text::ShortcutSelectAll)),
                shortcut("Ctrl+I", locale.text(Text::ShortcutInvertSelection)),
                shortcut("F1", locale.text(Text::ShortcutAbout)),
                shortcut("Esc", locale.text(Text::ShortcutCancel)),
            ]
            .spacing(10)
        )
        .padding(18)
        .width(Fill)
        .style(container::rounded_box),
        container(text(locale.text(Text::PrivacyDescription)))
            .padding(18)
            .width(Fill)
            .style(container::rounded_box),
    ]
    .spacing(18)
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
        let requires_password = state.pending_archive_requires_password;
        let heading = pending.and_then(|path| path.file_name()).map_or_else(
            || state.locale.text(Text::NoArchive).to_owned(),
            |name| name.to_string_lossy().into_owned(),
        );
        let description = archive_empty_state_description(
            state.locale,
            state.busy,
            pending.is_some(),
            requires_password,
        );
        let retry_controls: Element<'_, Message> = if requires_password {
            column![
                text_input(state.locale.text(Text::PasswordEncrypted), &state.password)
                    .secure(!state.password_visible)
                    .on_input(Message::PasswordChanged)
                    .on_submit_maybe(
                        (pending.is_some() && !state.busy).then_some(Message::ReloadArchive),
                    )
                    .width(280),
                checkbox(state.password_visible)
                    .label(state.locale.text(Text::ShowPassword))
                    .on_toggle(Message::ArchivePasswordVisibilityChanged),
                button(state.locale.text(Text::UnlockArchive))
                    .style(button::primary)
                    .on_press_maybe(
                        (pending.is_some() && !state.busy).then_some(Message::ReloadArchive),
                    ),
            ]
            .spacing(12)
            .into()
        } else {
            space().height(0).into()
        };
        return container(
            column![
                text(heading).size(30),
                text(description),
                retry_controls,
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

    if state.busy {
        let archive_name = archive
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy();
        return container(
            column![
                column![
                    text(archive_name).size(28),
                    text(format!(
                        "{} · {} entries · {}",
                        archive.format,
                        archive.entries.len(),
                        format_bytes(archive.total_size)
                    )),
                ]
                .spacing(5),
                row![
                    button(state.locale.text(Text::OpenAnother))
                        .style(button::secondary)
                        .on_press(Message::OpenArchiveDialog),
                    button(state.locale.text(Text::RevealInExplorer))
                        .style(button::secondary)
                        .on_press(Message::RevealArchive),
                    button(state.locale.text(Text::TestArchive))
                        .style(button::secondary)
                        .on_press(Message::TestArchive),
                    button(state.locale.text(Text::CloseArchive)).style(button::secondary),
                ]
                .spacing(10),
                container(text(state.locale.text(Text::BusyArchiveDescription))).padding(24),
            ]
            .spacing(20),
        )
        .padding(24)
        .width(Fill)
        .height(Fill)
        .into();
    }

    let all_files = archive
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .count();
    let all_selected = all_files > 0 && state.selected.len() == all_files;
    let header = column![
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
        .spacing(5),
        row![
            button(state.locale.text(Text::OpenAnother))
                .style(button::secondary)
                .on_press(Message::OpenArchiveDialog),
            button(state.locale.text(Text::RevealInExplorer))
                .style(button::secondary)
                .on_press(Message::RevealArchive),
            button(state.locale.text(Text::TestArchive))
                .style(button::secondary)
                .on_press(Message::TestArchive),
            button(state.locale.text(Text::CloseArchive))
                .style(button::secondary)
                .on_press(Message::CloseArchive),
        ]
        .spacing(10),
    ]
    .spacing(12);

    let selection_controls = row![
        checkbox(all_selected)
            .label(format!(
                "{} {}",
                state.selected.len(),
                state.locale.text(Text::Selected)
            ))
            .on_toggle(Message::SelectAll),
        button(state.locale.text(Text::SelectAll))
            .style(button::secondary)
            .on_press(Message::SelectAll(true)),
        button(state.locale.text(Text::SelectNone))
            .style(button::secondary)
            .on_press_maybe((!state.selected.is_empty()).then_some(Message::SelectAll(false))),
        button(state.locale.text(Text::InvertSelection))
            .style(button::secondary)
            .on_press(Message::InvertSelection),
        space().width(Fill),
        column![
            text_input(state.locale.text(Text::PasswordEncrypted), &state.password)
                .secure(!state.password_visible)
                .on_input(Message::PasswordChanged)
                .width(220),
            checkbox(state.password_visible)
                .label(state.locale.text(Text::ShowPassword))
                .on_toggle(Message::ArchivePasswordVisibilityChanged),
        ]
        .spacing(6),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(10);

    let operation_controls = row![
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
            .style(button::secondary)
            .on_press_maybe((!state.selected.is_empty()).then_some(Message::Extract)),
        button(state.locale.text(Text::ExtractToNamedFolder))
            .style(button::secondary)
            .on_press(Message::ExtractToNamedFolder),
        button(state.locale.text(Text::ExtractAll))
            .style(button::primary)
            .on_press(Message::ExtractAll),
    ]
    .spacing(10);

    let controls = column![selection_controls, operation_controls].spacing(10);

    let filtered_count = browser_entry_count(archive, &state.entry_directory, &state.entry_filter);
    let last_page = filtered_count.saturating_sub(1) / ENTRIES_PER_PAGE;
    let current_page = state.entry_page.min(last_page);
    let filter_summary = archive_filter_summary(
        state.locale,
        &state.entry_filter,
        filtered_count,
        archive.entries.len(),
    );
    let search_controls = row![
        text_input(state.locale.text(Text::Search), &state.entry_filter)
            .id(ARCHIVE_SEARCH_ID)
            .on_input(Message::EntryFilterChanged)
            .width(Fill),
        button(state.locale.text(Text::ClearSearch))
            .style(button::secondary)
            .on_press_maybe(
                (!state.entry_filter.is_empty()).then_some(Message::ClearArchiveFilter),
            ),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);
    let pagination_controls = row![
        text(filter_summary).size(12),
        space().width(Fill),
        text(if filtered_count == 0 {
            "—".to_owned()
        } else {
            format!(
                "{} {} / {}",
                state.locale.text(Text::Page),
                current_page + 1,
                last_page + 1
            )
        }),
        button(state.locale.text(Text::Previous))
            .style(button::secondary)
            .on_press_maybe((current_page > 0).then_some(Message::PreviousEntryPage)),
        button(state.locale.text(Text::Next))
            .style(button::secondary)
            .on_press_maybe((current_page < last_page).then_some(Message::NextEntryPage)),
    ]
    .spacing(10)
    .align_y(iced::Alignment::Center);
    let browser_controls = column![search_controls, pagination_controls].spacing(8);

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
            text(state.locale.text(Text::Checksum)).width(300),
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
        let checksum_cell: Element<'_, Message> = match entry.checksum.clone() {
            Some(checksum) => row![
                text(checksum.clone()).width(Fill),
                button(state.locale.text(Text::CopyChecksum))
                    .style(button::text)
                    .on_press(Message::CopyChecksum(checksum)),
            ]
            .spacing(4)
            .width(300)
            .into(),
            None => text("—").width(300).into(),
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
                checksum_cell,
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .padding([7, 4]),
        );
    }
    if filtered_count == 0 {
        entries = entries.push(
            container(text(archive_no_matches(state.locale, &state.entry_filter))).padding(24),
        );
    }

    column![
        header,
        controls,
        breadcrumbs,
        browser_controls,
        container(
            scrollable(container(entries).width(Length::Fixed(ARCHIVE_TABLE_MIN_WIDTH)))
                .height(Fill)
                .direction(scrollable::Direction::Both {
                    vertical: scrollable::Scrollbar::default(),
                    horizontal: scrollable::Scrollbar::default(),
                }),
        )
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
    let password_mismatch =
        !create_passwords_match(&state.create_password, &state.create_password_confirmation);
    let show_password_mismatch = state.create_password_confirmation_touched && password_mismatch;
    let single_file_format =
        state.create_format.create_input() == Some(CreateInputKind::SingleFile);
    let validation_notice: Element<'_, Message> = match source_issue {
        Some(issue) if !state.create_sources.is_empty() => {
            container(text(create_source_issue_text(state.locale, issue)))
                .padding(12)
                .width(Fill)
                .style(container::danger)
                .into()
        }
        _ => space().height(0).into(),
    };
    let compression_control: Element<'_, Message> =
        if let Some((minimum, maximum)) = state.create_format.compression_level_range() {
            column![
                text(format!(
                    "{} · {}",
                    state.locale.text(Text::CompressionLevel),
                    state.compression_level
                ))
                .size(13),
                slider(
                    minimum..=maximum,
                    state.compression_level,
                    Message::CompressionLevelChanged
                )
            ]
            .spacing(8)
            .width(Fill)
            .into()
        } else {
            text(state.locale.text(Text::CompressionFixed))
                .size(13)
                .width(Fill)
                .into()
        };

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
        text(create_source_summary(
            state.locale,
            state.create_sources.len()
        ))
        .size(13),
        validation_notice,
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
                    compression_control,
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
                    .secure(!state.create_password_visible)
                    .on_input_maybe(encryption_supported.then_some(Message::CreatePasswordChanged)),
                    text(state.locale.text(Text::ConfirmPassword)).size(13),
                    text_input(
                        state.locale.text(Text::ConfirmPassword),
                        &state.create_password_confirmation
                    )
                    .secure(!state.create_password_visible)
                    .on_input_maybe(
                        encryption_supported.then_some(Message::CreatePasswordConfirmationChanged),
                    ),
                    if show_password_mismatch {
                        text(state.locale.text(Text::PasswordMismatch))
                    } else {
                        text("")
                    },
                    checkbox(state.create_password_visible)
                        .label(state.locale.text(Text::ShowPassword))
                        .on_toggle_maybe(
                            encryption_supported
                                .then_some(Message::CreatePasswordVisibilityChanged),
                        ),
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
                .on_press_maybe(
                    (source_issue.is_none() && !password_mismatch).then_some(Message::Create),
                ),
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

fn apply_create_format(state: &mut ZiFile, format: ArchiveFormat) {
    state.create_format = format;
    state.compression_level = format.clamp_compression_level(state.compression_level);
    if !format.capabilities().encryption {
        state.create_password.clear();
        state.create_password_confirmation.clear();
        state.create_password_confirmation_touched = false;
        state.create_password_visible = false;
    }
    set_status(state, create_input_help(state.locale, format));
}

fn create_source_issue_text(locale: Locale, issue: CreateSourceIssue) -> &'static str {
    match issue {
        CreateSourceIssue::MissingSources => locale.text(Text::NoSources),
        CreateSourceIssue::MissingSource => locale.text(Text::MissingSource),
        CreateSourceIssue::LinkSource => locale.text(Text::LinkSource),
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
            text(heading).size(23),
            text(description),
            button(action)
                .style(button::primary)
                .on_press_maybe((!busy).then_some(message)),
        ]
        .spacing(14),
    )
    .padding(24)
    .width(Fill)
    .height(210)
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
        let unit_label = UNITS.get(unit).copied().unwrap_or("TB");
        format!("{value:.1} {unit_label}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use zifile_core::ArchiveEntryInfo;
    use zifile_desktop::entry_view::{filtered_entry_count, sorted_filtered_entry_page};

    #[test]
    fn default_window_settings_keep_the_layout_readable() {
        let settings = window_settings();

        assert_eq!(settings.size, iced::Size::new(1_180.0, 780.0));
        assert_eq!(settings.min_size, Some(iced::Size::new(920.0, 620.0)));
        assert!(matches!(
            settings.position,
            iced::window::Position::Centered
        ));
    }

    #[test]
    fn closing_archive_releases_session_state_and_returns_home() {
        let mut state = ZiFile {
            page: Page::Archive,
            archive: Some(ArchiveInfo {
                path: PathBuf::from("private.7z"),
                format: ArchiveFormat::SevenZip,
                entries: Vec::new(),
                total_size: 0,
                compressed_size: 0,
            }),
            pending_archive: Some(PathBuf::from("pending.zip")),
            pending_archive_requires_password: true,
            automatic_extract_destination: Some(PathBuf::from("output")),
            selected: HashSet::from([PathBuf::from("secret.txt")]),
            entry_directory: PathBuf::from("folder"),
            entry_filter: "secret".to_owned(),
            entry_page: 3,
            password: "not-retained".to_owned(),
            locale: Locale::En,
            ..ZiFile::default()
        };

        drop(update(&mut state, Message::CloseArchive));

        assert_eq!(state.page, Page::Home);
        assert!(state.archive.is_none());
        assert!(state.pending_archive.is_none());
        assert!(!state.pending_archive_requires_password);
        assert!(state.automatic_extract_destination.is_none());
        assert!(state.selected.is_empty());
        assert!(state.entry_directory.as_os_str().is_empty());
        assert!(state.entry_filter.is_empty());
        assert_eq!(state.entry_page, 0);
        assert!(state.password.is_empty());
        assert_eq!(state.status, "Archive closed");
    }

    #[test]
    fn closing_archive_is_rejected_while_work_is_running() {
        let mut state = ZiFile {
            archive: Some(ArchiveInfo {
                path: PathBuf::from("busy.zip"),
                format: ArchiveFormat::Zip,
                entries: Vec::new(),
                total_size: 0,
                compressed_size: 0,
            }),
            busy: true,
            ..ZiFile::default()
        };

        drop(update(&mut state, Message::CloseArchive));

        assert!(state.archive.is_some());
    }

    #[test]
    fn archive_layout_keeps_dense_columns_scrollable_at_the_minimum_width() {
        const {
            assert!(ARCHIVE_TABLE_MIN_WIDTH >= 1_000.0);
        }
        let source = include_str!("main.rs");
        assert!(source.contains("let selection_controls ="));
        assert!(source.contains("let pagination_controls ="));
        assert!(source.contains("scrollable::Direction::Both"));
        assert!(source.contains("scrollable::Scrollbar::default()"));
    }

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
        assert!(
            create_source_issue_text(Locale::En, CreateSourceIssue::MissingSource)
                .contains("no longer exist")
        );
    }

    #[test]
    fn changing_create_format_updates_status_and_format_state() {
        let mut state = ZiFile {
            locale: Locale::En,
            compression_level: 22,
            create_password: "secret".to_owned(),
            ..ZiFile::default()
        };

        apply_create_format(&mut state, ArchiveFormat::Bzip2);

        assert_eq!(state.create_format, ArchiveFormat::Bzip2);
        assert_eq!(state.compression_level, 9);
        assert!(state.create_password.is_empty());
        assert_eq!(
            state.status,
            "This stream format requires exactly one file. Use a TAR composition for folders or multiple items."
        );
        assert_eq!(state.status_kind, StatusKind::Informational);
    }

    #[test]
    fn create_form_uses_single_file_picker_for_stream_formats() {
        let source = include_str!("main.rs");
        assert!(source.contains("let single_file_format ="));
        assert!(source.contains(&["dialog.", "pick_file()"].concat()));
        assert!(source.contains(&["map(|path| vec!", "[path])"].concat()));
        assert!(source.contains(&["dialog.", "pick_files()"].concat()));
        assert!(source.contains("CreateInputKind::SingleFile"));
    }

    #[test]
    fn about_page_exposes_release_identity_in_the_default_ui() {
        let source = include_str!("main.rs");
        assert!(source.contains("Page::About => about_view(state)"));
        assert!(source.contains("env!(\"CARGO_PKG_VERSION\")"));
        assert!(source.contains("\"MIT\".to_owned()"));
        assert!(source.contains("ArchiveFormat::ALL.len().to_string()"));
        assert!(source.contains("https://github.com/ax2/zifile"));
        assert!(source.contains("key::Named::F1"));
        assert!(source.contains("KeyboardShortcut(Shortcut::About)"));
    }

    #[test]
    fn default_shortcuts_require_exact_modifiers() {
        use iced::keyboard::Key;
        use iced::keyboard::Modifiers;
        use iced::keyboard::key::{Code, Named, NativeCode, Physical};

        let key_n = Key::Character("n".into());
        let physical_n = Physical::Code(Code::KeyN);
        assert_eq!(
            default_shortcut(&key_n, physical_n, Modifiers::CTRL),
            Some(Shortcut::Create)
        );
        assert_eq!(
            default_shortcut(&key_n, physical_n, Modifiers::CTRL | Modifiers::SHIFT),
            None
        );
        assert_eq!(
            default_shortcut(
                &Key::Character("R".into()),
                Physical::Code(Code::KeyR),
                Modifiers::CTRL
            ),
            Some(Shortcut::Reload)
        );
        assert_eq!(
            default_shortcut(
                &Key::Character("r".into()),
                Physical::Code(Code::KeyR),
                Modifiers::CTRL | Modifiers::SHIFT
            ),
            None
        );
        assert_eq!(
            default_shortcut(
                &Key::Character("F".into()),
                Physical::Code(Code::KeyF),
                Modifiers::CTRL
            ),
            Some(Shortcut::Search)
        );
        assert_eq!(
            default_shortcut(
                &Key::Character("f".into()),
                Physical::Code(Code::KeyF),
                Modifiers::CTRL | Modifiers::ALT
            ),
            None
        );
        assert_eq!(
            default_shortcut(
                &Key::Character("W".into()),
                Physical::Code(Code::KeyW),
                Modifiers::CTRL
            ),
            Some(Shortcut::Close)
        );
        assert_eq!(
            default_shortcut(
                &Key::Character("w".into()),
                Physical::Code(Code::KeyW),
                Modifiers::CTRL | Modifiers::SHIFT
            ),
            None
        );
        assert_eq!(
            default_shortcut(
                &Key::Character("I".into()),
                Physical::Code(Code::KeyI),
                Modifiers::CTRL
            ),
            Some(Shortcut::InvertSelection)
        );
        assert_eq!(
            default_shortcut(
                &Key::Character("i".into()),
                Physical::Code(Code::KeyI),
                Modifiers::CTRL | Modifiers::SHIFT
            ),
            None
        );
        assert_eq!(
            default_shortcut(
                &Key::Named(Named::F1),
                Physical::Unidentified(NativeCode::Unidentified),
                Modifiers::ALT
            ),
            None
        );
        assert_eq!(
            default_shortcut(
                &Key::Named(Named::Escape),
                Physical::Unidentified(NativeCode::Unidentified),
                Modifiers::NONE
            ),
            Some(Shortcut::Cancel)
        );
    }

    #[test]
    fn shortcut_help_matches_the_default_keyboard_map() {
        let source = include_str!("main.rs");
        for (keys, text_key) in [
            ("Ctrl+O", "Text::ShortcutOpen"),
            ("Ctrl+N", "Text::ShortcutCreate"),
            ("Ctrl+R", "Text::ShortcutReload"),
            ("Ctrl+F", "Text::ShortcutSearch"),
            ("Ctrl+W", "Text::ShortcutClose"),
            ("Ctrl+A", "Text::ShortcutSelectAll"),
            ("Ctrl+I", "Text::ShortcutInvertSelection"),
            ("F1", "Text::ShortcutAbout"),
            ("Esc", "Text::ShortcutCancel"),
        ] {
            assert!(source.contains(&format!("shortcut(\"{keys}\", locale.text({text_key}))")));
        }
    }

    #[test]
    fn accepted_create_submission_releases_the_form_password() {
        let mut password = "not-for-retention".to_owned();
        let mut confirmation = "not-for-retention".to_owned();
        let mut confirmation_touched = true;
        let mut visible = true;
        clear_submitted_create_password(
            &mut password,
            &mut confirmation,
            &mut confirmation_touched,
            &mut visible,
            OperationKind::Create,
            true,
        );
        assert!(password.is_empty());
        assert!(confirmation.is_empty());
        assert!(!confirmation_touched);
        assert!(!visible);

        password.push_str("retry-secret");
        confirmation.push_str("retry-secret");
        confirmation_touched = true;
        visible = true;
        clear_submitted_create_password(
            &mut password,
            &mut confirmation,
            &mut confirmation_touched,
            &mut visible,
            OperationKind::Create,
            false,
        );
        assert_eq!(password, "retry-secret");
        assert_eq!(confirmation, "retry-secret");
        assert!(confirmation_touched);
        assert!(visible);

        clear_submitted_create_password(
            &mut password,
            &mut confirmation,
            &mut confirmation_touched,
            &mut visible,
            OperationKind::Extract,
            true,
        );
        assert_eq!(password, "retry-secret");
        assert_eq!(confirmation, "retry-secret");
        assert!(confirmation_touched);
        assert!(visible);
    }

    #[test]
    fn mismatched_create_passwords_are_rejected_before_the_save_dialog() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let source = temporary.path().join("source.txt");
        std::fs::write(&source, b"source").expect("source fixture");
        let mut state = ZiFile {
            locale: Locale::En,
            create_sources: vec![source],
            create_password: "secret".to_owned(),
            create_password_confirmation: "different".to_owned(),
            create_password_confirmation_touched: true,
            ..ZiFile::default()
        };

        drop(update(&mut state, Message::Create));

        assert!(!state.dialog_open);
        assert_eq!(state.status_kind, StatusKind::Error);
        assert_eq!(state.status, "The password confirmation does not match.");
    }

    #[test]
    fn clearing_an_archive_password_restores_masking() {
        let mut state = ZiFile {
            password: "not-for-retention".to_owned(),
            password_visible: true,
            ..ZiFile::default()
        };

        clear_archive_password(&mut state);

        assert!(state.password.is_empty());
        assert!(!state.password_visible);
    }

    #[test]
    fn password_visibility_controls_cover_archive_and_create_fields() {
        let state = ZiFile::default();
        assert!(!state.password_visible);
        assert!(!state.create_password_visible);

        let source = include_str!("main.rs");
        assert!(source.contains(".secure(!state.password_visible)"));
        assert!(source.contains(".secure(!state.create_password_visible)"));
        assert!(source.contains("Message::ArchivePasswordVisibilityChanged"));
        assert!(source.contains("Message::CreatePasswordVisibilityChanged"));
        assert!(source.contains("Text::ShowPassword"));
        assert!(source.contains(".on_submit_maybe("));
    }

    #[test]
    fn empty_password_input_restores_masking() {
        let mut state = ZiFile {
            password: "secret".to_owned(),
            password_visible: true,
            create_password: "secret".to_owned(),
            create_password_visible: true,
            ..ZiFile::default()
        };

        drop(update(&mut state, Message::PasswordChanged(String::new())));
        assert!(!state.password_visible);

        drop(update(
            &mut state,
            Message::CreatePasswordChanged(String::new()),
        ));
        assert!(!state.create_password_visible);
    }

    #[test]
    fn completed_output_action_is_scoped_to_create_and_extract_requests() {
        let destination = PathBuf::from(r"C:\output\archive.zip");
        let create = WorkerRequest::Create {
            sources: vec![PathBuf::from(r"C:\input\file.txt")],
            destination: destination.clone(),
            format: ArchiveFormat::Zip,
            compression_level: 6,
            password: None,
        };
        let test = WorkerRequest::Test {
            archive: destination.clone(),
            password: None,
        };

        assert_eq!(operation_output_path(&create), Some(destination));
        assert_eq!(operation_output_path(&test), None);
        let source = include_str!("main.rs");
        assert!(source.contains("Message::RevealCompletedOutput"));
        assert!(source.contains("Text::RevealOutput"));
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
                checksum: None,
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
    fn checksum_copy_action_is_wired_to_the_native_clipboard() {
        let source = include_str!("main.rs");
        assert!(source.contains("Message::CopyChecksum(String)"));
        assert!(source.contains("iced::clipboard::write(checksum)"));
        assert!(source.contains("Text::CopyChecksum"));
    }

    #[test]
    fn queue_handoff_keeps_busy_until_the_next_operation_starts() {
        let source = include_str!("main.rs");
        let queue_source = source
            .split("fn continue_queue")
            .nth(1)
            .expect("continue_queue implementation should exist");
        let next_start = queue_source
            .find("Some(next) => start_operation(state, next)")
            .expect("queued work should start directly");
        let idle_transition = queue_source
            .find("state.busy = false;")
            .expect("idle transition should still clear busy state");
        assert!(next_start < idle_transition);
    }

    #[test]
    fn archive_header_can_reveal_the_source_in_file_explorer() {
        let source = include_str!("main.rs");
        assert!(source.contains("Message::RevealArchive"));
        assert!(source.contains("reveal_in_file_manager(&path)"));
        assert!(source.contains("Text::RevealInExplorer"));
    }

    #[test]
    fn home_page_wires_recent_archives_to_open_and_clear_actions() {
        let source = include_str!("main.rs");
        assert!(source.contains("Message::OpenRecentArchive(path.clone())"));
        assert!(source.contains("Message::ClearRecentArchives"));
        assert!(source.contains("Message::RemoveRecentArchive(path.clone())"));
        assert!(source.contains("record_recent_archive(state, recent_path)"));
        assert!(source.contains("(!state.busy).then_some(Message::OpenRecentArchive"));
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
    fn clear_archive_filter_resets_only_filter_state() {
        let mut state = ZiFile {
            entry_directory: PathBuf::from("docs"),
            entry_filter: "readme".to_owned(),
            entry_page: 7,
            ..ZiFile::default()
        };
        drop(update(&mut state, Message::ClearArchiveFilter));
        assert_eq!(state.entry_directory, PathBuf::from("docs"));
        assert!(state.entry_filter.is_empty());
        assert_eq!(state.entry_page, 0);
    }

    #[test]
    fn create_source_changes_update_the_status_region() {
        let mut state = ZiFile {
            locale: Locale::En,
            ..ZiFile::default()
        };
        drop(update(
            &mut state,
            Message::AddFilesDialogFinished(Some(vec![PathBuf::from("a.txt")])),
        ));
        assert_eq!(state.status, "Added 1 archive source; 1 total");

        drop(update(
            &mut state,
            Message::AddFolderDialogFinished(Some(PathBuf::from("folder"))),
        ));
        assert_eq!(state.status, "Added 1 archive source; 2 total");

        drop(update(&mut state, Message::RemoveSource(0)));
        assert_eq!(state.status, "Removed archive source a.txt; 1 remaining");

        drop(update(&mut state, Message::ClearSources));
        assert_eq!(state.status, "Cleared 1 archive sources");
    }

    #[test]
    fn status_kind_distinguishes_errors_from_normal_updates() {
        let mut state = ZiFile {
            locale: Locale::En,
            ..ZiFile::default()
        };
        drop(update(&mut state, Message::Create));
        assert_eq!(state.status_kind, StatusKind::Error);

        drop(update(&mut state, Message::ClearSources));
        assert_eq!(state.status_kind, StatusKind::Informational);
    }

    #[test]
    fn queued_archive_load_keeps_the_visible_archive_until_it_starts() {
        let mut state = ZiFile {
            archive: Some(ArchiveInfo {
                path: PathBuf::from("current.zip"),
                format: ArchiveFormat::Zip,
                entries: Vec::new(),
                total_size: 0,
                compressed_size: 0,
            }),
            page: Page::Archive,
            ..ZiFile::default()
        };
        state
            .operations
            .submit(QueuedOperation {
                kind: OperationKind::Test,
                request: WorkerRequest::Test {
                    archive: PathBuf::from("current.zip"),
                    password: None,
                },
                status: "testing".to_owned(),
                archive_path: None,
            })
            .expect("the first operation should occupy the queue");

        drop(begin_load(&mut state, PathBuf::from("next.zip")));

        assert_eq!(
            state.archive.as_ref().map(|archive| archive.path.as_path()),
            Some(Path::new("current.zip"))
        );
        assert_eq!(state.pending_archive, None);
        assert_eq!(state.operations.pending_count(), 1);
    }

    #[test]
    fn full_queue_keeps_the_visible_archive_when_open_is_rejected() {
        let mut state = ZiFile {
            archive: Some(ArchiveInfo {
                path: PathBuf::from("current.zip"),
                format: ArchiveFormat::Zip,
                entries: Vec::new(),
                total_size: 0,
                compressed_size: 0,
            }),
            ..ZiFile::default()
        };
        for index in 0..32 {
            state
                .operations
                .submit(QueuedOperation {
                    kind: OperationKind::Test,
                    request: WorkerRequest::Test {
                        archive: PathBuf::from(format!("archive-{index}.zip")),
                        password: None,
                    },
                    status: "testing".to_owned(),
                    archive_path: None,
                })
                .expect("the fixture should fill the queue");
        }

        drop(begin_load(&mut state, PathBuf::from("rejected.zip")));

        assert_eq!(
            state.archive.as_ref().map(|archive| archive.path.as_path()),
            Some(Path::new("current.zip"))
        );
        assert_eq!(state.operations.len(), 32);
    }

    #[test]
    fn stale_archive_completion_cannot_clear_the_active_operation() {
        let mut state = ZiFile::default();
        drop(begin_load(&mut state, PathBuf::from("sample.zip")));
        let status = state.status.clone();
        drop(update(
            &mut state,
            Message::ArchiveLoaded(999, Err("archive header is corrupt".to_owned())),
        ));
        assert_eq!(state.operations.active_id(), Some(1));
        assert!(state.busy);
        assert_eq!(state.status, status);
        assert!(!state.pending_archive_requires_password);
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
                    checksum: None,
                    modified: None,
                },
                ArchiveEntryInfo {
                    path: PathBuf::from("other.txt"),
                    size: 1,
                    compressed_size: 1,
                    is_directory: false,
                    encrypted: false,
                    checksum: None,
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
                checksum: None,
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

    #[test]
    fn archive_view_exposes_separate_selected_and_all_extraction_actions() {
        let source = include_str!("main.rs");
        assert!(source.contains("Text::ExtractSelected"));
        assert!(source.contains(".style(button::secondary)"));
        assert!(source.contains("Text::ExtractToNamedFolder"));
        assert!(source.contains("Message::ExtractToNamedFolder"));
        assert!(source.contains("Text::ExtractAll"));
        assert!(source.contains(".style(button::primary)"));
        assert!(source.contains("Message::ExtractAll"));
    }

    #[test]
    fn quick_extraction_targets_the_matching_sibling_folder_and_all_entries() {
        let state = ZiFile {
            archive: Some(ArchiveInfo {
                path: PathBuf::from(r"C:\archives\backup.tar.gz"),
                format: ArchiveFormat::TarGzip,
                entries: Vec::new(),
                total_size: 0,
                compressed_size: 0,
            }),
            selected: HashSet::from([PathBuf::from("partial.txt")]),
            password: "session-secret".to_owned(),
            ..ZiFile::default()
        };

        let operation = quick_extraction_operation(&state).expect("archive should be extractable");
        let WorkerRequest::Extract {
            archive,
            destination,
            password,
            selected_paths,
            ..
        } = operation.request
        else {
            panic!("quick extraction should create an extract request");
        };
        assert_eq!(archive, PathBuf::from(r"C:\archives\backup.tar.gz"));
        assert_eq!(destination, PathBuf::from(r"C:\archives\backup"));
        assert_eq!(password.as_deref(), Some("session-secret"));
        assert!(selected_paths.is_none());
    }

    #[test]
    fn create_view_surfaces_non_empty_source_validation_inline() {
        let source = include_str!("main.rs");
        assert!(source.contains("let validation_notice: Element<'_, Message>"));
        assert!(source.contains("!state.create_sources.is_empty()"));
        assert!(source.contains(".style(container::danger)"));
        assert!(source.contains("validation_notice,"));
    }
}
