use std::collections::HashSet;
use std::fmt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use iced::widget::{
    button, checkbox, column, container, pick_list, progress_bar, row, rule, scrollable, slider,
    space, text, text_input,
};
use iced::{Element, Fill, Length, Subscription, Task, Theme};
use rfd::FileDialog;
use zifile_core::{
    ArchiveFormat, ArchiveInfo, CancellationToken, ConflictPolicy, CreateOptions, ExtractOptions,
    OperationProgress, OperationSummary, create_archive, detect_format_from_path, extract_archive,
    list_archive, test_archive,
};

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
    let archive = std::env::args_os().nth(1).map(PathBuf::from);
    let task = archive.map_or_else(Task::none, |path| begin_load(&mut state, path));
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
    selected: HashSet<PathBuf>,
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
    dark: bool,
}

impl Default for ZiFile {
    fn default() -> Self {
        Self {
            page: Page::Home,
            archive: None,
            selected: HashSet::new(),
            password: String::new(),
            conflict: ConflictChoice::Rename,
            create_sources: Vec::new(),
            create_format: ArchiveFormat::Zip,
            create_password: String::new(),
            compression_level: 6,
            status: "Ready".to_owned(),
            busy: false,
            cancellation: None,
            progress: None,
            dark: true,
        }
    }
}

#[derive(Debug, Clone)]
enum Message {
    Navigate(Page),
    ToggleTheme,
    OpenArchiveDialog,
    ArchiveLoaded(Result<ArchiveInfo, String>),
    PasswordChanged(String),
    ReloadArchive,
    ToggleEntry(PathBuf, bool),
    SelectAll(bool),
    ConflictChanged(ConflictChoice),
    Extract,
    ExtractFinished(Result<OperationSummary, String>),
    TestArchive,
    TestFinished(Result<ArchiveInfo, String>),
    AddFiles,
    AddFolder,
    RemoveSource(usize),
    ClearSources,
    CreateFormatChanged(ArchiveFormat),
    CreatePasswordChanged(String),
    CompressionLevelChanged(u8),
    Create,
    CreateFinished(Result<OperationSummary, String>),
    Cancel,
    ProgressTick,
    FileDropped(PathBuf),
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
        Message::ToggleTheme => state.dark = !state.dark,
        Message::OpenArchiveDialog => {
            if let Some(path) = archive_dialog().pick_file() {
                return begin_load(state, path);
            }
        }
        Message::ArchiveLoaded(result) => {
            state.busy = false;
            match result {
                Ok(archive) => {
                    state.selected = archive
                        .entries
                        .iter()
                        .filter(|entry| !entry.is_directory)
                        .map(|entry| entry.path.clone())
                        .collect();
                    state.status = format!(
                        "Opened {} entries · {} expanded",
                        archive.entries.len(),
                        format_bytes(archive.total_size)
                    );
                    state.archive = Some(archive);
                    state.page = Page::Archive;
                }
                Err(error) => state.status = format!("Open failed: {error}"),
            }
        }
        Message::PasswordChanged(password) => state.password = password,
        Message::ReloadArchive => {
            if let Some(path) = state.archive.as_ref().map(|archive| archive.path.clone()) {
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
                .set_title("Choose extraction folder")
                .set_directory(default_folder.parent().unwrap_or_else(|| Path::new(".")))
                .pick_folder()
            else {
                return Task::none();
            };
            let path = archive.path.clone();
            let cancellation = CancellationToken::default();
            let progress = OperationProgress::default();
            let options = ExtractOptions {
                conflict: state.conflict.into(),
                password: non_empty(&state.password),
                selected_paths: Some(state.selected.clone()),
                cancellation: cancellation.clone(),
                progress: progress.clone(),
                ..ExtractOptions::default()
            };
            state.busy = true;
            state.cancellation = Some(cancellation);
            state.progress = Some(progress);
            state.status = format!("Extracting to {}…", destination.display());
            return Task::perform(
                async move {
                    extract_archive(path, destination, &options).map_err(|error| error.to_string())
                },
                Message::ExtractFinished,
            );
        }
        Message::ExtractFinished(result) => {
            state.busy = false;
            state.cancellation = None;
            state.progress = None;
            state.status = match result {
                Ok(summary) => format!(
                    "Extracted {} files · {} · skipped {}",
                    summary.files,
                    format_bytes(summary.bytes),
                    summary.skipped
                ),
                Err(error) => format!("Extraction failed: {error}"),
            };
        }
        Message::TestArchive => {
            let Some(path) = state.archive.as_ref().map(|archive| archive.path.clone()) else {
                return Task::none();
            };
            let password = non_empty(&state.password);
            state.busy = true;
            state.status = "Testing every entry and checksum…".to_owned();
            return Task::perform(
                async move {
                    test_archive(path, password.as_deref()).map_err(|error| error.to_string())
                },
                Message::TestFinished,
            );
        }
        Message::TestFinished(result) => {
            state.busy = false;
            state.status = match result {
                Ok(info) => format!(
                    "Archive is healthy · {} entries · {}",
                    info.entries.len(),
                    format_bytes(info.total_size)
                ),
                Err(error) => format!("Integrity test failed: {error}"),
            };
        }
        Message::AddFiles => {
            if let Some(paths) = FileDialog::new()
                .set_title("Add files to archive")
                .pick_files()
            {
                append_unique(&mut state.create_sources, paths);
                state.page = Page::Create;
            }
        }
        Message::AddFolder => {
            if let Some(path) = FileDialog::new()
                .set_title("Add folder to archive")
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
        Message::CreateFormatChanged(format) => state.create_format = format,
        Message::CreatePasswordChanged(password) => state.create_password = password,
        Message::CompressionLevelChanged(level) => state.compression_level = level,
        Message::Create => {
            if state.create_sources.is_empty() {
                state.status = "Add at least one file or folder first".to_owned();
                return Task::none();
            }
            let extension = state.create_format.canonical_extension();
            let Some(destination) = FileDialog::new()
                .set_title("Create archive")
                .add_filter(state.create_format.to_string(), &[extension])
                .set_file_name(format!("archive.{extension}"))
                .save_file()
            else {
                return Task::none();
            };
            let sources = state.create_sources.clone();
            let format = state.create_format;
            let options = CreateOptions {
                compression_level: state.compression_level,
                password: non_empty(&state.create_password),
                cancellation: CancellationToken::default(),
                progress: OperationProgress::default(),
            };
            state.busy = true;
            state.cancellation = Some(options.cancellation.clone());
            state.progress = Some(options.progress.clone());
            state.status = format!("Creating {}…", destination.display());
            return Task::perform(
                async move {
                    create_archive(&sources, destination, format, &options)
                        .map_err(|error| error.to_string())
                },
                Message::CreateFinished,
            );
        }
        Message::CreateFinished(result) => {
            state.busy = false;
            state.cancellation = None;
            state.progress = None;
            state.status = match result {
                Ok(summary) => format!(
                    "Archive created · {} files · {} input",
                    summary.files,
                    format_bytes(summary.bytes)
                ),
                Err(error) => format!("Creation failed: {error}"),
            };
        }
        Message::Cancel => {
            if let Some(cancellation) = &state.cancellation {
                cancellation.cancel();
                state.status = "Cancelling safely after the current block…".to_owned();
            }
        }
        Message::ProgressTick => {}
        Message::FileDropped(path) => {
            if state.busy {
                state.status =
                    "Wait for the current operation before dropping more files".to_owned();
            } else if path.is_file()
                && detect_format_from_path(&path).is_some_and(|format| format.capabilities().list)
            {
                return begin_load(state, path);
            } else if path.exists() {
                append_unique(&mut state.create_sources, vec![path]);
                state.page = Page::Create;
                state.status = "Added dropped source".to_owned();
            }
        }
    }
    Task::none()
}

fn subscription(state: &ZiFile) -> Subscription<Message> {
    let progress = if state.busy && state.progress.is_some() {
        iced::time::every(Duration::from_millis(100)).map(|_| Message::ProgressTick)
    } else {
        Subscription::none()
    };
    Subscription::batch([progress, iced::event::listen_with(drop_event)])
}

fn drop_event(
    event: iced::Event,
    _status: iced::event::Status,
    _window: iced::window::Id,
) -> Option<Message> {
    match event {
        iced::Event::Window(iced::window::Event::FileDropped(path)) => {
            Some(Message::FileDropped(path))
        }
        _ => None,
    }
}

fn begin_load(state: &mut ZiFile, path: PathBuf) -> Task<Message> {
    let password = non_empty(&state.password);
    state.busy = true;
    state.status = format!("Opening {}…", path.display());
    Task::perform(
        async move { list_archive(path, password.as_deref()).map_err(|error| error.to_string()) },
        Message::ArchiveLoaded,
    )
}

fn view(state: &ZiFile) -> Element<'_, Message> {
    let navigation = container(
        column![
            text("ZiFile").size(30),
            text("Archive Studio").size(13),
            rule::horizontal(1),
            nav_button("⌂  Home", Page::Home, state.page),
            nav_button("▣  Archive", Page::Archive, state.page),
            nav_button("＋  Create", Page::Create, state.page),
            space().height(Fill),
            button(if state.dark {
                "☀  Light"
            } else {
                "☾  Dark"
            })
            .style(button::secondary)
            .width(Fill)
            .on_press(Message::ToggleTheme),
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
            row![
                container(progress_bar(0.0..=1.0, snapshot.fraction())).width(Fill),
                text(format!(
                    "{} / {} · {} / {}",
                    snapshot.processed_entries,
                    snapshot.total_entries,
                    format_bytes(snapshot.processed_bytes),
                    format_bytes(snapshot.total_bytes)
                ))
                .size(12),
            ]
            .spacing(12)
            .align_y(iced::Alignment::Center)
            .into()
        },
    );
    let status = container(
        column![
            row![
                if state.busy {
                    "● Working"
                } else {
                    "● Ready"
                },
                text(&state.status).size(13).width(Fill),
                button("Cancel")
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
        "Open an archive",
        "Browse, verify and safely extract ZIP, 7z and TAR-family archives.",
        "Open archive",
        Message::OpenArchiveDialog,
        state.busy,
    );
    let create = action_card(
        "Create an archive",
        "Package files and folders with compression level and optional encryption.",
        "Start creating",
        Message::Navigate(Page::Create),
        state.busy,
    );
    column![
        text("Files, packed beautifully.").size(38),
        text("A fast, local-first archive manager built in Rust for Windows.").size(17),
        row![open, create].spacing(18),
        container(column![
            text("Privacy by default").size(20),
            text("Archive work stays on this device. ZiFile does not upload filenames, passwords or file contents."),
        ].spacing(8)).padding(20).width(Fill).style(container::rounded_box),
    ]
    .spacing(24)
    .width(Fill)
    .into()
}

fn archive_view(state: &ZiFile) -> Element<'_, Message> {
    let Some(archive) = &state.archive else {
        return container(
            column![
                text("No archive open").size(30),
                text("Open an archive to inspect its contents and extract files."),
                button("Open archive")
                    .style(button::primary)
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
            text(format!(
                "{} · {} entries · {} expanded",
                archive.format,
                archive.entries.len(),
                format_bytes(archive.total_size)
            )),
        ]
        .spacing(5)
        .width(Fill),
        button("Open another")
            .style(button::secondary)
            .on_press_maybe((!state.busy).then_some(Message::OpenArchiveDialog)),
        button("Test archive")
            .style(button::secondary)
            .on_press_maybe((!state.busy).then_some(Message::TestArchive)),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(10);

    let controls = row![
        checkbox(all_selected)
            .label(format!("{} selected", state.selected.len()))
            .on_toggle(Message::SelectAll),
        space().width(Fill),
        text_input("Password (if encrypted)", &state.password)
            .secure(true)
            .on_input(Message::PasswordChanged)
            .width(220),
        button("Reload").on_press_maybe((!state.busy).then_some(Message::ReloadArchive)),
        pick_list(
            ConflictChoice::ALL,
            Some(state.conflict),
            Message::ConflictChanged
        ),
        button("Extract selected")
            .style(button::primary)
            .on_press_maybe(
                (!state.busy && !state.selected.is_empty()).then_some(Message::Extract)
            ),
    ]
    .align_y(iced::Alignment::Center)
    .spacing(10);

    let mut entries = column![
        row![
            text(" ").width(34),
            text("Name").width(Fill),
            text("Original").width(110),
            text("Packed").width(110),
            text("Flags").width(90),
        ]
        .spacing(10),
        rule::horizontal(1),
    ]
    .spacing(4);
    for entry in &archive.entries {
        let path = entry.path.clone();
        let selected = state.selected.contains(&path);
        entries = entries.push(
            row![
                if entry.is_directory {
                    checkbox(false)
                } else {
                    checkbox(selected)
                        .on_toggle(move |value| Message::ToggleEntry(path.clone(), value))
                }
                .width(34),
                text(format!(
                    "{} {}",
                    if entry.is_directory { "▸" } else { "•" },
                    entry.path.display()
                ))
                .width(Fill),
                text(format_bytes(entry.size)).width(110),
                text(format_bytes(entry.compressed_size)).width(110),
                text(if entry.encrypted { "Locked" } else { "—" }).width(90),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center)
            .padding([7, 4]),
        );
    }

    column![
        header,
        controls,
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
        sources = sources.push(text("No sources yet. Add files or one or more folders."));
    }
    for (index, source) in state.create_sources.iter().enumerate() {
        sources = sources.push(
            row![
                text(if source.is_dir() { "▣" } else { "•" }),
                text(source.display().to_string()).width(Fill),
                button("Remove")
                    .style(button::secondary)
                    .on_press(Message::RemoveSource(index)),
            ]
            .spacing(10)
            .align_y(iced::Alignment::Center),
        );
    }
    let encryption_supported = state.create_format.capabilities().encryption;

    column![
        text("Create archive").size(32),
        text("Choose sources, format and compression. The archive is written through a temporary file before replacing its destination."),
        row![
            button("Add files").on_press_maybe((!state.busy).then_some(Message::AddFiles)),
            button("Add folder").on_press_maybe((!state.busy).then_some(Message::AddFolder)),
            button("Clear").style(button::secondary).on_press_maybe((!state.create_sources.is_empty()).then_some(Message::ClearSources)),
        ].spacing(10),
        container(scrollable(sources).height(Length::Fixed(240.0)))
            .padding(16)
            .width(Fill)
            .style(container::rounded_box),
        container(column![
            row![
                column![text("Format").size(13), pick_list(CREATE_FORMATS, Some(state.create_format), Message::CreateFormatChanged)].spacing(6).width(240),
                column![text(format!("Compression level · {}", state.compression_level)).size(13), slider(0..=9, state.compression_level, Message::CompressionLevelChanged)].spacing(8).width(Fill),
            ].spacing(20),
            column![
                text(if encryption_supported { "Password · optional" } else { "Password · unavailable for this format" }).size(13),
                text_input("Leave empty for no encryption", &state.create_password)
                    .secure(true)
                    .on_input_maybe(encryption_supported.then_some(Message::CreatePasswordChanged)),
            ].spacing(6),
        ].spacing(16)).padding(18).width(Fill).style(container::rounded_box),
        row![
            space().width(Fill),
            button("Create archive")
                .style(button::primary)
                .on_press_maybe((!state.busy && !state.create_sources.is_empty()).then_some(Message::Create)),
        ],
    ]
    .spacing(14)
    .height(Fill)
    .into()
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

fn archive_dialog() -> FileDialog {
    FileDialog::new()
        .set_title("Open archive")
        .add_filter(
            "Supported archives",
            &[
                "zip", "7z", "tar", "gz", "tgz", "zst", "xz", "bz2", "lz4", "br",
            ],
        )
        .add_filter("All files", &["*"])
}

fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
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
