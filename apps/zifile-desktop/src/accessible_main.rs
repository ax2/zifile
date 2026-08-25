#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use dioxus::prelude::*;
use dioxus_html::HasFileData;
use rfd::FileDialog;
use zifile_core::{
    ArchiveEntryInfo, ArchiveFormat, ArchiveInfo, CancellationToken, ConflictPolicy,
    OperationProgress, OperationSummary, ProgressSnapshot, SafetyLimits, detect_format_from_path,
};
use zifile_worker_protocol::WorkerRequest;

mod i18n;
mod settings;
mod taskbar;
mod worker_client;

use i18n::{Locale, Text};
use settings::AppSettings;
use worker_client::{WorkerOutput, run_worker};
use zifile_desktop::entry_view::{ENTRIES_PER_PAGE, filtered_entry_count, filtered_entry_page};
use zifile_desktop::operation_queue::{Job, OperationQueue, Submission};
use zifile_desktop::startup::{self, StartupRequest};

const STYLES: &str = include_str!("accessible_ui.css");
const OPERATION_PROGRESS_LIVE: &str = "off";
const SECURITY_HEAD: &str = r#"<meta http-equiv="Content-Security-Policy" content="script-src 'unsafe-inline' 'unsafe-eval'; style-src 'unsafe-inline'; img-src data:; connect-src dioxus: ws://127.0.0.1:* http://dioxus.index.html https://dioxus.index.html ipc:; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'">"#;
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

fn main() {
    use dioxus::desktop::{Config, LogicalSize, WindowBuilder};
    let window = WindowBuilder::new()
        .with_title("ZiFile")
        .with_inner_size(LogicalSize::new(1180.0, 760.0))
        .with_min_inner_size(LogicalSize::new(720.0, 560.0));
    dioxus::LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_window(window)
                .with_menu(None)
                .with_disable_context_menu(true)
                .with_custom_head(SECURITY_HEAD.to_owned())
                .with_navigation_handler(|_| false),
        )
        .launch(App);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    Home,
    Archive,
    Create,
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccessibleShortcut {
    Open,
    Create,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StatusKind {
    Informational,
    Error,
}

#[derive(Debug, Clone)]
struct UiState {
    page: Page,
    archive: Option<ArchiveInfo>,
    selected: HashSet<PathBuf>,
    entry_filter: String,
    entry_page: usize,
    password: String,
    conflict: ConflictPolicy,
    create_sources: Vec<PathBuf>,
    create_format: ArchiveFormat,
    create_password: String,
    compression_level: u8,
    status: String,
    status_kind: StatusKind,
    busy: bool,
    cancellation: Option<CancellationToken>,
    progress: Option<OperationProgress>,
    operations: Arc<Mutex<OperationQueue<QueuedOperation>>>,
    dark: bool,
    locale: Locale,
    revision: u64,
}

impl Default for UiState {
    fn default() -> Self {
        let settings = AppSettings::load();
        Self {
            page: Page::Home,
            archive: None,
            selected: HashSet::new(),
            entry_filter: String::new(),
            entry_page: 0,
            password: String::new(),
            conflict: ConflictPolicy::Rename,
            create_sources: Vec::new(),
            create_format: ArchiveFormat::Zip,
            create_password: String::new(),
            compression_level: 6,
            status: settings.locale.text(Text::Ready).to_owned(),
            status_kind: StatusKind::Informational,
            busy: false,
            cancellation: None,
            progress: None,
            operations: Arc::new(Mutex::new(OperationQueue::default())),
            dark: settings.dark,
            locale: settings.locale,
            revision: 0,
        }
    }
}

impl UiState {
    fn set_status(&mut self, status: String) {
        self.status = status;
        self.status_kind = StatusKind::Informational;
    }

    fn set_error(&mut self, status: String) {
        self.status = status;
        self.status_kind = StatusKind::Error;
    }
}

#[component]
fn App() -> Element {
    let mut state = use_signal(UiState::default);
    let startup_request = use_hook(|| startup::parse(std::env::args_os().skip(1)));
    let mut startup_processed = use_signal(|| false);
    use_effect(move || {
        if !startup_processed() {
            startup_processed.set(true);
            match startup_request.clone() {
                StartupRequest::Home => {}
                StartupRequest::OpenArchive(path) => begin_load(state, path),
                StartupRequest::CreateFrom(sources) => {
                    let mut value = state.write();
                    value.page = Page::Create;
                    append_unique(&mut value.create_sources, sources);
                }
            }
        }
    });
    use_future(move || async move {
        loop {
            tokio::time::sleep(Duration::from_millis(100)).await;
            if state.read().busy {
                let next = state.read().revision.wrapping_add(1);
                state.write().revision = next;
            }
        }
    });

    let view = state.read().clone();
    let locale = view.locale;
    let page = view.page;
    let theme = if view.dark { "dark" } else { "light" };
    let progress = view.progress.as_ref().map(OperationProgress::snapshot);
    let (status_role, status_live) = status_semantics(view.status_kind);
    let queued_count = view
        .operations
        .lock()
        .expect("operation queue lock must not be poisoned")
        .pending_count();
    let queue_summary = operation_queue_summary(locale, queued_count);
    let progress_view = progress.map(|snapshot| {
        (
            (snapshot.fraction() * 100.0).round() as u8,
            operation_progress_text(locale, snapshot),
        )
    });
    taskbar::sync(
        view.busy,
        view.cancellation
            .as_ref()
            .is_some_and(CancellationToken::is_cancelled),
        progress,
    );

    rsx! {
        style { {STYLES} }
        div {
            class: "app-shell {theme}",
            lang: locale.code(),
            tabindex: "-1",
            autofocus: true,
            onkeydown: move |event: KeyboardEvent| {
                if event.is_composing() {
                    return;
                }
                let control = event.modifiers().contains(Modifiers::CONTROL);
                if let Some(shortcut) = accessible_shortcut(&event.key().to_string(), control) {
                    event.prevent_default();
                    apply_accessible_shortcut(state, shortcut);
                }
            },
            ondragover: move |event: DragEvent| event.prevent_default(),
            ondrop: move |event: DragEvent| {
                event.prevent_default();
                let paths = event.files().into_iter().map(|file| file.path()).collect();
                handle_dropped_paths(state, paths);
            },
            aside { class: "sidebar", "aria-label": choose(locale, "Primary", "主导航"),
                header { h1 { "ZiFile" } p { {locale.text(Text::ArchiveStudio)} } }
                nav {
                    NavButton { label: locale.text(Text::Home), active: page == Page::Home, onclick: move |_| state.write().page = Page::Home }
                    NavButton { label: locale.text(Text::Archive), active: page == Page::Archive, onclick: move |_| state.write().page = Page::Archive }
                    NavButton { label: locale.text(Text::Create), active: page == Page::Create, onclick: move |_| state.write().page = Page::Create }
                }
                div { class: "preferences",
                    button { onclick: move |_| { let mut value = state.write(); value.dark = !value.dark; save_settings(&value); },
                        {if view.dark { locale.text(Text::Light) } else { locale.text(Text::Dark) }} }
                    button { lang: if locale == Locale::ZhCn { "en-US" } else { "zh-CN" },
                        onclick: move |_| { let mut value = state.write(); value.locale = value.locale.toggle(); let status = value.locale.text(Text::Ready).to_owned(); value.set_status(status); save_settings(&value); },
                        {locale.text(Text::SwitchLanguage)} }
                }
            }
            main { id: "main-content", tabindex: "-1",
                match page {
                    Page::Home => rsx! { Home { state } },
                    Page::Archive => rsx! { ArchivePage { state } },
                    Page::Create => rsx! { CreatePage { state } },
                }
                footer { class: if view.status_kind == StatusKind::Error { "status-error" } else { "" },
                    div { id: "operation-status", class: "status-copy", role: status_role, "aria-live": status_live, "aria-atomic": "true",
                        span { class: "status-dot", "aria-hidden": "true", "•" } span { {view.status.clone()} }
                    }
                    output { id: "operation-queue-summary", class: "queue-count", role: "status", "aria-live": "polite", "aria-atomic": "true", {queue_summary} }
                    if let Some((percent, progress_text)) = progress_view {
                        progress { max: "100", value: "{percent}", "aria-label": choose(locale, "Operation progress", "操作进度"), "aria-valuetext": progress_text, "aria-describedby": "operation-status", "aria-live": OPERATION_PROGRESS_LIVE }
                    }
                    button { class: "queue-clear", disabled: queued_count == 0, "aria-describedby": "operation-queue-summary", onclick: move |_| clear_queued(state), {choose(locale, "Clear queue", "清空队列")} }
                    button { disabled: !view.busy, "aria-describedby": "operation-status", "aria-keyshortcuts": "Escape", onclick: move |_| cancel_operation(state), {locale.text(Text::Cancel)} }
                }
            }
        }
    }
}

#[component]
fn NavButton(label: &'static str, active: bool, onclick: EventHandler<MouseEvent>) -> Element {
    rsx! { button { class: if active { "nav-item active" } else { "nav-item" }, "aria-current": if active { "page" } else { "false" }, onclick: move |event| onclick.call(event), {label} } }
}

#[component]
fn Home(mut state: Signal<UiState>) -> Element {
    let view = state.read().clone();
    let locale = view.locale;
    rsx! { section { class: "home", "aria-labelledby": "home-title",
        h2 { id: "home-title", {locale.text(Text::Hero)} } p { class: "lead", {locale.text(Text::HeroSub)} }
        div { class: "actions",
            article { h3 { {locale.text(Text::OpenArchive)} } p { {locale.text(Text::OpenDescription)} }
                button { class: "primary", onclick: move |_| open_archive_dialog(state), {locale.text(Text::OpenAction)} } }
            article { h3 { {locale.text(Text::CreateArchive)} } p { {locale.text(Text::CreateDescription)} }
                button { class: "primary", onclick: move |_| state.write().page = Page::Create, {locale.text(Text::StartCreating)} } }
        }
        section { class: "privacy", "aria-labelledby": "privacy-title", h3 { id: "privacy-title", {locale.text(Text::Privacy)} } p { {locale.text(Text::PrivacyDescription)} } }
    } }
}

#[component]
fn ArchivePage(mut state: Signal<UiState>) -> Element {
    let view = state.read().clone();
    let locale = view.locale;
    let Some(archive) = view.archive.clone() else {
        return rsx! { section { class: "empty-state", h2 { {locale.text(Text::NoArchive)} } p { {locale.text(Text::NoArchiveDescription)} }
        button { class: "primary", onclick: move |_| open_archive_dialog(state), {locale.text(Text::OpenAction)} } } };
    };
    let count = filtered_entry_count(&archive, &view.entry_filter);
    let last_page = count.saturating_sub(1) / ENTRIES_PER_PAGE;
    let current_page = view.entry_page.min(last_page);
    let rows = filtered_entry_page(&archive, &view.entry_filter, current_page)
        .into_iter()
        .cloned()
        .collect::<Vec<_>>();
    let all_files = archive
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .count();
    let selected_count = view.selected.len();
    let all_selected = all_files > 0 && selected_count == all_files;
    let selection_summary = archive_selection_summary(locale, selected_count, all_files);
    let select_all_label = archive_select_all_label(locale, all_selected);
    let archive_name = archive
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    rsx! { section { class: "archive-page", "aria-labelledby": "archive-title",
        div { class: "page-heading", div { h2 { id: "archive-title", {archive_name} } p { "{archive.format} · {archive.entries.len()} · {format_bytes(archive.total_size)}" } }
            div { class: "button-row", button { onclick: move |_| open_archive_dialog(state), {locale.text(Text::OpenAnother)} }
                button { onclick: move |_| test_archive(state), {locale.text(Text::TestArchive)} } } }
        div { class: "toolbar",
            label { span { {locale.text(Text::PasswordEncrypted)} } input { r#type: "password", autocomplete: "off", spellcheck: "false", value: view.password.clone(), oninput: move |event| state.write().password = event.value() } }
            button { onclick: move |_| reload_archive(state), {locale.text(Text::Reload)} }
            label { span { {locale.text(Text::Search)} } input { r#type: "search", value: view.entry_filter.clone(), oninput: move |event| { let mut value = state.write(); value.entry_filter = event.value(); value.entry_page = 0; } } }
        }
        div { class: "selection-bar",
            label { input { r#type: "checkbox", checked: all_selected, "aria-label": select_all_label, "aria-describedby": "archive-selection-summary", "aria-keyshortcuts": "Control+A", onchange: move |event| select_all(state, event.checked()) }
                output { id: "archive-selection-summary", role: "status", "aria-live": "polite", "aria-atomic": "true", {selection_summary.clone()} }
            }
            div { class: "button-row",
                select { value: conflict_value(view.conflict), "aria-label": choose(locale, "Conflict policy", "文件冲突策略"), onchange: move |event| state.write().conflict = parse_conflict(&event.value()),
                    for policy in [ConflictPolicy::Rename, ConflictPolicy::Overwrite, ConflictPolicy::Skip, ConflictPolicy::Error] { option { value: conflict_value(policy), {conflict_label(locale, policy)} } } }
                button { class: "primary", disabled: selected_count == 0, "aria-describedby": "archive-selection-summary", onclick: move |_| extract_selected(state), {locale.text(Text::ExtractSelected)} }
            }
        }
        div { class: "table-wrap", tabindex: "0", role: "region", "aria-label": choose(locale, "Archive entries", "压缩文件项目"), "aria-describedby": "archive-selection-summary", "aria-keyshortcuts": "Control+A",
            onkeydown: move |event: KeyboardEvent| {
                let control = event.modifiers().contains(Modifiers::CONTROL);
                if !event.is_composing()
                    && is_select_all_shortcut(&event.key().to_string(), control)
                {
                    event.prevent_default();
                    select_all(state, true);
                }
            },
            table {
                caption { class: "sr-only", {choose(locale, "Archive entry details and selection", "压缩文件项目详情与选择")} }
                thead { tr {
                    th { scope: "col", span { class: "sr-only", {choose(locale, "Select", "选择")} } }
                    th { scope: "col", {locale.text(Text::Name)} }
                    th { scope: "col", {locale.text(Text::Original)} }
                    th { scope: "col", {locale.text(Text::Packed)} }
                    th { scope: "col", {locale.text(Text::Flags)} }
                } }
                tbody { for entry in rows { ArchiveRow { key: "{entry.path.display()}", state, entry, locale } } }
            }
        }
        nav { class: "pagination", "aria-label": choose(locale, "Entry pages", "项目分页"),
            button { disabled: current_page == 0, onclick: move |_| state.write().entry_page = current_page.saturating_sub(1), {locale.text(Text::Previous)} }
            span { "{locale.text(Text::Page)} {current_page + 1} / {last_page + 1}" }
            button { disabled: current_page >= last_page, onclick: move |_| state.write().entry_page = (current_page + 1).min(last_page), {locale.text(Text::Next)} }
        }
    } }
}

#[component]
fn ArchiveRow(mut state: Signal<UiState>, entry: ArchiveEntryInfo, locale: Locale) -> Element {
    let selected = state.read().selected.contains(&entry.path);
    let path = entry.path.clone();
    let path_display = entry.path.to_string_lossy().to_string();
    let selection_label = format!("{} {path_display}", choose(locale, "Select", "选择"));
    rsx! { tr {
        td { input { r#type: "checkbox", checked: selected, disabled: entry.is_directory, "aria-label": selection_label,
            onchange: move |event| update_archive_selection(state, path.clone(), event.checked()) } }
        td { class: "path-cell", {path_display} } td { {format_bytes(entry.size)} } td { {format_bytes(entry.compressed_size)} }
        td { if entry.encrypted { {locale.text(Text::Locked)} } else if entry.is_directory { {choose(locale, "Folder", "文件夹")} } else { "—" } }
    } }
}

#[component]
fn CreatePage(mut state: Signal<UiState>) -> Element {
    let view = state.read().clone();
    let locale = view.locale;
    let encrypted = view.create_format.capabilities().encryption;
    let source_summary = create_source_summary(locale, view.create_sources.len());
    rsx! { section { class: "create-page", "aria-labelledby": "create-title",
        div { class: "page-heading", div { h2 { id: "create-title", {locale.text(Text::CreateHeading)} } p { {locale.text(Text::CreateHelp)} } }
            div { class: "button-row", button { onclick: move |_| add_files(state), {locale.text(Text::AddFiles)} }
                button { onclick: move |_| add_folder(state), {locale.text(Text::AddFolder)} }
                button { disabled: view.create_sources.is_empty(), "aria-describedby": "create-source-summary", onclick: move |_| clear_create_sources(state), {locale.text(Text::Clear)} } } }
        section { class: "source-list", "aria-label": choose(locale, "Archive sources", "压缩来源"), "aria-describedby": "create-source-summary",
            output { id: "create-source-summary", class: "source-summary", role: "status", "aria-live": "polite", "aria-atomic": "true", {source_summary} }
            if view.create_sources.is_empty() { p { class: "muted", {locale.text(Text::NoSources)} } }
            ul { for source in view.create_sources.iter() { li { key: "{source.display()}", span { {source.to_string_lossy().to_string()} }
                button { "aria-label": create_source_remove_label(locale, &source.to_string_lossy()), onclick: { let source = source.clone(); move |_| remove_create_source(state, source.clone()) }, {locale.text(Text::Remove)} } } } }
        }
        div { class: "form-grid",
            label { span { {locale.text(Text::Format)} } select { value: format_value(view.create_format),
                onchange: move |event| { let mut value = state.write(); value.create_format = parse_format(&event.value()); if !value.create_format.capabilities().encryption { value.create_password.clear(); } },
                for format in CREATE_FORMATS { option { value: format_value(format), "{format}" } } } }
            label { span { "{locale.text(Text::CompressionLevel)} · {view.compression_level}" } input { r#type: "range", min: "0", max: "9", value: "{view.compression_level}",
                oninput: move |event| state.write().compression_level = event.value().parse().unwrap_or(6) } }
            label { span { if encrypted { {locale.text(Text::PasswordOptional)} } else { {locale.text(Text::PasswordUnavailable)} } }
                input { r#type: "password", autocomplete: "off", spellcheck: "false", placeholder: locale.text(Text::NoEncryption), value: view.create_password.clone(), disabled: !encrypted, oninput: move |event| state.write().create_password = event.value() } }
        }
        div { class: "create-actions", button { class: "primary", disabled: view.create_sources.is_empty(), onclick: move |_| create_archive(state), {locale.text(Text::CreateAction)} } }
    } }
}

fn open_archive_dialog(state: Signal<UiState>) {
    let locale = state.read().locale;
    if let Some(path) = archive_dialog(locale).pick_file() {
        begin_load(state, path);
    }
}

fn accessible_shortcut(key: &str, control: bool) -> Option<AccessibleShortcut> {
    if key.eq_ignore_ascii_case("escape") {
        return Some(AccessibleShortcut::Cancel);
    }
    if !control {
        return None;
    }
    match key.to_ascii_lowercase().as_str() {
        "o" => Some(AccessibleShortcut::Open),
        "n" => Some(AccessibleShortcut::Create),
        _ => None,
    }
}

fn is_select_all_shortcut(key: &str, control: bool) -> bool {
    control && key.eq_ignore_ascii_case("a")
}

fn apply_accessible_shortcut(mut state: Signal<UiState>, shortcut: AccessibleShortcut) {
    match shortcut {
        AccessibleShortcut::Open => open_archive_dialog(state),
        AccessibleShortcut::Create => state.write().page = Page::Create,
        AccessibleShortcut::Cancel => cancel_operation(state),
    }
}

fn handle_dropped_paths(mut state: Signal<UiState>, paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    if paths.len() == 1
        && paths[0].is_file()
        && detect_format_from_path(&paths[0]).is_some_and(|format| format.capabilities().list)
    {
        begin_load(state, paths.into_iter().next().expect("one dropped path"));
        return;
    }
    let existing = paths.into_iter().filter(|path| path.exists()).collect();
    let locale = state.read().locale;
    let mut value = state.write();
    let added = append_unique(&mut value.create_sources, existing);
    let total = value.create_sources.len();
    value.page = Page::Create;
    value.set_status(create_sources_added_status(locale, added, total));
}

fn reload_archive(state: Signal<UiState>) {
    let path = {
        let value = state.read();
        value.archive.as_ref().map(|archive| archive.path.clone())
    };
    if let Some(path) = path {
        begin_load(state, path);
    }
}

fn begin_load(state: Signal<UiState>, path: PathBuf) {
    let locale = state.read().locale;
    let password = non_empty(&state.read().password);
    let status = format!(
        "{} {}…",
        choose(locale, "Opening", "正在打开"),
        path.display()
    );
    launch_worker(
        state,
        WorkerRequest::List {
            archive: path,
            password,
        },
        OperationKind::List,
        status,
    );
}

fn test_archive(state: Signal<UiState>) {
    let value = state.read();
    let Some(path) = value.archive.as_ref().map(|archive| archive.path.clone()) else {
        return;
    };
    let password = non_empty(&value.password);
    let locale = value.locale;
    drop(value);
    launch_worker(
        state,
        WorkerRequest::Test {
            archive: path,
            password,
        },
        OperationKind::Test,
        choose(
            locale,
            "Testing every entry and checksum…",
            "正在校验所有项目与校验和…",
        )
        .to_owned(),
    );
}

fn extract_selected(state: Signal<UiState>) {
    let value = state.read();
    let Some(archive) = value.archive.clone() else {
        return;
    };
    let default_folder = archive
        .path
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(archive.path.file_stem().unwrap_or_default());
    let locale = value.locale;
    let selected = value.selected.clone();
    let conflict = value.conflict;
    let password = non_empty(&value.password);
    drop(value);
    let Some(destination) = FileDialog::new()
        .set_title(locale.text(Text::ChooseExtractionFolder))
        .set_directory(default_folder.parent().unwrap_or_else(|| Path::new(".")))
        .pick_folder()
    else {
        return;
    };
    let file_count = archive
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .count();
    let selected_paths = (selected.len() != file_count).then(|| selected.into_iter().collect());
    let request = WorkerRequest::Extract {
        archive: archive.path,
        destination: destination.clone(),
        conflict,
        limits: SafetyLimits::default(),
        password,
        selected_paths,
    };
    launch_worker(
        state,
        request,
        OperationKind::Extract,
        format!(
            "{} {}…",
            choose(locale, "Extracting to", "正在解压到"),
            destination.display()
        ),
    );
}

fn add_files(mut state: Signal<UiState>) {
    let locale = state.read().locale;
    if let Some(paths) = FileDialog::new()
        .set_title(locale.text(Text::AddFilesDialog))
        .pick_files()
    {
        let mut value = state.write();
        let added = append_unique(&mut value.create_sources, paths);
        let status = create_sources_added_status(locale, added, value.create_sources.len());
        value.set_status(status);
    }
}

fn add_folder(mut state: Signal<UiState>) {
    let locale = state.read().locale;
    if let Some(path) = FileDialog::new()
        .set_title(locale.text(Text::AddFolderDialog))
        .pick_folder()
    {
        let mut value = state.write();
        let added = append_unique(&mut value.create_sources, vec![path]);
        let status = create_sources_added_status(locale, added, value.create_sources.len());
        value.set_status(status);
    }
}

fn remove_create_source(mut state: Signal<UiState>, path: PathBuf) {
    let mut value = state.write();
    let before = value.create_sources.len();
    value.create_sources.retain(|source| source != &path);
    if value.create_sources.len() == before {
        return;
    }
    let status = create_source_removed_status(
        value.locale,
        &path.to_string_lossy(),
        value.create_sources.len(),
    );
    value.set_status(status);
}

fn clear_create_sources(mut state: Signal<UiState>) {
    let mut value = state.write();
    let cleared = value.create_sources.len();
    value.create_sources.clear();
    let status = match value.locale {
        Locale::En => format!("Cleared {cleared} archive sources"),
        Locale::ZhCn => format!("已清除 {cleared} 个压缩来源"),
    };
    value.set_status(status);
}

fn create_archive(state: Signal<UiState>) {
    let value = state.read();
    if value.create_sources.is_empty() {
        return;
    }
    let locale = value.locale;
    let format = value.create_format;
    let sources = value.create_sources.clone();
    let compression_level = value.compression_level;
    let password = non_empty(&value.create_password);
    drop(value);
    let extension = format.canonical_extension();
    let Some(destination) = FileDialog::new()
        .set_title(locale.text(Text::CreateDialog))
        .add_filter(format.to_string(), &[extension])
        .set_file_name(format!("archive.{extension}"))
        .save_file()
    else {
        return;
    };
    let request = WorkerRequest::Create {
        sources,
        destination: destination.clone(),
        format,
        compression_level,
        password,
    };
    launch_worker(
        state,
        request,
        OperationKind::Create,
        format!(
            "{} {}…",
            choose(locale, "Creating", "正在创建"),
            destination.display()
        ),
    );
}

fn launch_worker(
    mut state: Signal<UiState>,
    request: WorkerRequest,
    kind: OperationKind,
    status: String,
) {
    let operation = QueuedOperation {
        kind,
        request,
        status,
    };
    let operations = state.read().operations.clone();
    let submission = operations
        .lock()
        .expect("operation queue lock must not be poisoned")
        .submit(operation);
    match submission {
        Ok(Submission::Start(job)) => start_worker(state, job),
        Ok(Submission::Queued { position, .. }) => {
            let locale = state.read().locale;
            let status = match locale {
                Locale::En => format!(
                    "Queued operation at position {position}; the current operation continues"
                ),
                Locale::ZhCn => format!("操作已排队，位置 {position}；当前操作继续运行"),
            };
            state.write().set_status(status);
        }
        Err(error) => {
            let locale = state.read().locale;
            let status = match locale {
                Locale::En => format!("Operation queue is full (maximum {})", error.capacity),
                Locale::ZhCn => format!("操作队列已满（最多 {} 个）", error.capacity),
            };
            state.write().set_error(status);
        }
    }
}

fn start_worker(mut state: Signal<UiState>, job: Job<QueuedOperation>) {
    let Job { id, payload } = job;
    let QueuedOperation {
        kind,
        request,
        status,
    } = payload;
    let progress = OperationProgress::default();
    let cancellation = CancellationToken::default();
    {
        let mut value = state.write();
        value.busy = true;
        value.set_status(status);
        value.progress = Some(progress.clone());
        value.cancellation = Some(cancellation.clone());
    }
    spawn(async move {
        let result =
            tokio::task::spawn_blocking(move || run_worker(request, progress, cancellation))
                .await
                .map_err(|error| format!("worker task failed: {error}"))
                .and_then(|result| result);
        finish_worker(state, id, kind, result);
    });
}

fn finish_worker(
    mut state: Signal<UiState>,
    id: u64,
    kind: OperationKind,
    result: Result<WorkerOutput, String>,
) {
    let locale = state.read().locale;
    let (status, status_kind) = match (kind, result) {
        (OperationKind::List, Ok(WorkerOutput::Archive(archive))) => {
            let status = if locale == Locale::ZhCn {
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
            let selected = archive
                .entries
                .iter()
                .filter(|entry| !entry.is_directory)
                .map(|entry| entry.path.clone())
                .collect();
            let mut value = state.write();
            value.archive = Some(archive);
            value.selected = selected;
            value.entry_filter.clear();
            value.entry_page = 0;
            value.page = Page::Archive;
            (status, StatusKind::Informational)
        }
        (OperationKind::Test, Ok(WorkerOutput::Archive(info))) => {
            let status = if locale == Locale::ZhCn {
                format!(
                    "压缩文件完好 · {} 个项目 · {}",
                    info.entries.len(),
                    format_bytes(info.total_size)
                )
            } else {
                format!(
                    "Archive is healthy · {} entries · {}",
                    info.entries.len(),
                    format_bytes(info.total_size)
                )
            };
            (status, StatusKind::Informational)
        }
        (OperationKind::Extract, Ok(WorkerOutput::Summary(summary))) => (
            summary_status(locale, summary, true),
            StatusKind::Informational,
        ),
        (OperationKind::Create, Ok(WorkerOutput::Summary(summary))) => (
            summary_status(locale, summary, false),
            StatusKind::Informational,
        ),
        (_, Ok(_)) => (
            choose(
                locale,
                "Worker returned an unexpected result",
                "Worker 返回了意外结果",
            )
            .to_owned(),
            StatusKind::Error,
        ),
        (kind, Err(error)) => (
            format!(
                "{}: {error}",
                match kind {
                    OperationKind::List => choose(locale, "Open failed", "打开失败"),
                    OperationKind::Test => {
                        choose(locale, "Integrity test failed", "完整性校验失败")
                    }
                    OperationKind::Extract => choose(locale, "Extraction failed", "解压失败"),
                    OperationKind::Create => choose(locale, "Creation failed", "创建失败"),
                }
            ),
            StatusKind::Error,
        ),
    };
    let operations = state.read().operations.clone();
    {
        let mut value = state.write();
        value.busy = false;
        value.cancellation = None;
        value.progress = None;
        value.status = status;
        value.status_kind = status_kind;
    }
    let next = operations
        .lock()
        .expect("operation queue lock must not be poisoned")
        .complete(id);
    match next {
        Ok(Some(job)) => start_worker(state, job),
        Ok(None) => {}
        Err(error) => state
            .write()
            .set_error(format!("Internal operation queue error: {error}")),
    }
}

fn clear_queued(mut state: Signal<UiState>) {
    let operations = state.read().operations.clone();
    let cleared = operations
        .lock()
        .expect("operation queue lock must not be poisoned")
        .clear_pending()
        .len();
    let locale = state.read().locale;
    let status = match locale {
        Locale::En => format!("Cleared {cleared} queued operations"),
        Locale::ZhCn => format!("已清除 {cleared} 个排队操作"),
    };
    state.write().set_status(status);
}

fn cancel_operation(mut state: Signal<UiState>) {
    let value = state.read();
    if let Some(cancellation) = &value.cancellation {
        cancellation.cancel();
    }
    let locale = value.locale;
    drop(value);
    state.write().set_status(
        choose(
            locale,
            "Cancelling safely after the current block…",
            "正在当前数据块结束后安全取消…",
        )
        .to_owned(),
    );
}

fn select_all(mut state: Signal<UiState>, selected: bool) {
    let paths = state
        .read()
        .archive
        .as_ref()
        .map(|archive| {
            archive
                .entries
                .iter()
                .filter(|entry| !entry.is_directory)
                .map(|entry| entry.path.clone())
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let count = paths.len();
    let mut value = state.write();
    value.selected = if selected { paths } else { HashSet::new() };
    let status = if selected {
        match value.locale {
            Locale::En => format!("Selected all {count} archive files"),
            Locale::ZhCn => format!("已选择全部 {count} 个归档文件"),
        }
    } else {
        choose(value.locale, "Selection cleared", "已清除选择").to_owned()
    };
    value.set_status(status);
}

fn update_archive_selection(mut state: Signal<UiState>, path: PathBuf, selected: bool) {
    let mut value = state.write();
    if selected {
        value.selected.insert(path.clone());
    } else {
        value.selected.remove(&path);
    }
    let selected_count = value.selected.len();
    let status = archive_selection_change_status(
        value.locale,
        &path.to_string_lossy(),
        selected,
        selected_count,
    );
    value.set_status(status);
}

fn archive_selection_summary(locale: Locale, selected: usize, total: usize) -> String {
    match locale {
        Locale::En => format!(
            "{selected} of {total} files {}",
            locale.text(Text::Selected)
        ),
        Locale::ZhCn => format!("{selected}/{total} {}", locale.text(Text::Selected)),
    }
}

const fn archive_select_all_label(locale: Locale, all_selected: bool) -> &'static str {
    match (locale, all_selected) {
        (Locale::En, false) => "Select all archive files",
        (Locale::En, true) => "Clear all archive selections",
        (Locale::ZhCn, false) => "选择全部归档文件",
        (Locale::ZhCn, true) => "清除全部归档文件选择",
    }
}

fn archive_selection_change_status(
    locale: Locale,
    path: &str,
    selected: bool,
    selected_count: usize,
) -> String {
    let english_unit = if selected_count == 1 { "file" } else { "files" };
    match (locale, selected) {
        (Locale::En, true) => {
            format!("Selected {path}; {selected_count} {english_unit} selected")
        }
        (Locale::En, false) => {
            format!("Cleared {path}; {selected_count} {english_unit} selected")
        }
        (Locale::ZhCn, true) => format!("已选择 {path}；共选择 {selected_count} 个文件"),
        (Locale::ZhCn, false) => format!("已取消选择 {path}；共选择 {selected_count} 个文件"),
    }
}

fn create_source_summary(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!(
            "{count} archive {} added",
            if count == 1 { "source" } else { "sources" }
        ),
        Locale::ZhCn => format!("已添加 {count} 个压缩来源"),
    }
}

fn create_source_remove_label(locale: Locale, path: &str) -> String {
    match locale {
        Locale::En => format!("Remove archive source {path}"),
        Locale::ZhCn => format!("移除压缩来源 {path}"),
    }
}

fn create_sources_added_status(locale: Locale, added: usize, total: usize) -> String {
    match locale {
        Locale::En => format!(
            "Added {added} archive {}; {total} total",
            if added == 1 { "source" } else { "sources" }
        ),
        Locale::ZhCn => format!("已添加 {added} 个压缩来源；共 {total} 个"),
    }
}

fn create_source_removed_status(locale: Locale, path: &str, remaining: usize) -> String {
    match locale {
        Locale::En => format!("Removed archive source {path}; {remaining} remaining"),
        Locale::ZhCn => format!("已移除压缩来源 {path}；剩余 {remaining} 个"),
    }
}

fn archive_dialog(locale: Locale) -> FileDialog {
    FileDialog::new()
        .set_title(locale.text(Text::OpenDialog))
        .add_filter(
            locale.text(Text::SupportedArchives),
            &[
                "zip", "7z", "rar", "tar", "gz", "tgz", "zst", "xz", "bz2", "lz4", "br",
            ],
        )
        .add_filter(locale.text(Text::AllFiles), &["*"])
}

fn save_settings(state: &UiState) {
    AppSettings {
        locale: state.locale,
        dark: state.dark,
    }
    .save();
}

fn summary_status(locale: Locale, summary: OperationSummary, extract: bool) -> String {
    match (locale, extract) {
        (Locale::ZhCn, true) => format!(
            "已解压 {} 个文件 · {} · 跳过 {} 个",
            summary.files,
            format_bytes(summary.bytes),
            summary.skipped
        ),
        (Locale::En, true) => format!(
            "Extracted {} files · {} · skipped {}",
            summary.files,
            format_bytes(summary.bytes),
            summary.skipped
        ),
        (Locale::ZhCn, false) => format!(
            "压缩文件已创建 · {} 个文件 · 输入 {}",
            summary.files,
            format_bytes(summary.bytes)
        ),
        (Locale::En, false) => format!(
            "Archive created · {} files · {} input",
            summary.files,
            format_bytes(summary.bytes)
        ),
    }
}

fn conflict_label(locale: Locale, policy: ConflictPolicy) -> &'static str {
    locale.text(match policy {
        ConflictPolicy::Overwrite => Text::ConflictOverwrite,
        ConflictPolicy::Skip => Text::ConflictSkip,
        ConflictPolicy::Rename | ConflictPolicy::Ask => Text::ConflictRename,
        ConflictPolicy::Error => Text::ConflictError,
    })
}
const fn conflict_value(policy: ConflictPolicy) -> &'static str {
    match policy {
        ConflictPolicy::Overwrite => "overwrite",
        ConflictPolicy::Skip => "skip",
        ConflictPolicy::Rename | ConflictPolicy::Ask => "rename",
        ConflictPolicy::Error => "error",
    }
}
fn parse_conflict(value: &str) -> ConflictPolicy {
    match value {
        "overwrite" => ConflictPolicy::Overwrite,
        "skip" => ConflictPolicy::Skip,
        "error" => ConflictPolicy::Error,
        _ => ConflictPolicy::Rename,
    }
}
const fn format_value(format: ArchiveFormat) -> &'static str {
    format.canonical_extension()
}
fn parse_format(value: &str) -> ArchiveFormat {
    CREATE_FORMATS
        .into_iter()
        .find(|format| format.canonical_extension() == value)
        .unwrap_or(ArchiveFormat::Zip)
}
fn non_empty(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_owned())
}
const fn status_semantics(kind: StatusKind) -> (&'static str, &'static str) {
    match kind {
        StatusKind::Informational => ("status", "polite"),
        StatusKind::Error => ("alert", "assertive"),
    }
}
fn operation_queue_summary(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => match count {
            0 => "No operations queued".to_owned(),
            1 => "1 operation queued".to_owned(),
            _ => format!("{count} operations queued"),
        },
        Locale::ZhCn => format!("{count} 个操作排队"),
    }
}
fn operation_progress_text(locale: Locale, snapshot: ProgressSnapshot) -> String {
    let percent = (snapshot.fraction() * 100.0).round() as u8;
    let processed_entries = snapshot.processed_entries.min(snapshot.total_entries);
    let processed_bytes = snapshot.processed_bytes.min(snapshot.total_bytes);
    match (locale, snapshot.total_bytes > 0, snapshot.total_entries > 0) {
        (Locale::En, true, true) => format!(
            "{percent}% · {} of {} · {processed_entries} of {} {}",
            format_bytes(processed_bytes),
            format_bytes(snapshot.total_bytes),
            snapshot.total_entries,
            if snapshot.total_entries == 1 {
                "entry"
            } else {
                "entries"
            }
        ),
        (Locale::En, true, false) => format!(
            "{percent}% · {} of {}",
            format_bytes(processed_bytes),
            format_bytes(snapshot.total_bytes)
        ),
        (Locale::En, false, true) => format!(
            "{percent}% · {processed_entries} of {} {}",
            snapshot.total_entries,
            if snapshot.total_entries == 1 {
                "entry"
            } else {
                "entries"
            }
        ),
        (Locale::En, false, false) => "Operation starting".to_owned(),
        (Locale::ZhCn, true, true) => format!(
            "{percent}% · {} / {} · {processed_entries} / {} 项",
            format_bytes(processed_bytes),
            format_bytes(snapshot.total_bytes),
            snapshot.total_entries
        ),
        (Locale::ZhCn, true, false) => format!(
            "{percent}% · {} / {}",
            format_bytes(processed_bytes),
            format_bytes(snapshot.total_bytes)
        ),
        (Locale::ZhCn, false, true) => {
            format!(
                "{percent}% · {processed_entries} / {} 项",
                snapshot.total_entries
            )
        }
        (Locale::ZhCn, false, false) => "操作正在启动".to_owned(),
    }
}
const fn choose<'a>(locale: Locale, english: &'a str, chinese: &'a str) -> &'a str {
    match locale {
        Locale::En => english,
        Locale::ZhCn => chinese,
    }
}
fn append_unique(destination: &mut Vec<PathBuf>, paths: Vec<PathBuf>) -> usize {
    let before = destination.len();
    for path in paths {
        if !destination.contains(&path) {
            destination.push(path);
        }
    }
    destination.len() - before
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
    #[test]
    fn format_values_round_trip() {
        for format in CREATE_FORMATS {
            assert_eq!(parse_format(format_value(format)), format);
        }
    }
    #[test]
    fn conflict_values_round_trip() {
        for policy in [
            ConflictPolicy::Overwrite,
            ConflictPolicy::Skip,
            ConflictPolicy::Rename,
            ConflictPolicy::Error,
        ] {
            assert_eq!(parse_conflict(conflict_value(policy)), policy);
        }
    }

    #[test]
    fn archive_selection_accessibility_copy_is_actionable_and_bilingual() {
        assert_eq!(
            archive_selection_summary(Locale::En, 2, 5),
            "2 of 5 files selected"
        );
        assert_eq!(
            archive_selection_summary(Locale::ZhCn, 2, 5),
            "2/5 项已选择"
        );
        assert_eq!(
            archive_select_all_label(Locale::En, false),
            "Select all archive files"
        );
        assert_eq!(
            archive_select_all_label(Locale::ZhCn, true),
            "清除全部归档文件选择"
        );
    }

    #[test]
    fn archive_selection_change_status_identifies_item_and_count() {
        assert_eq!(
            archive_selection_change_status(Locale::En, "docs/readme.txt", true, 1),
            "Selected docs/readme.txt; 1 file selected"
        );
        assert_eq!(
            archive_selection_change_status(Locale::ZhCn, "文档/说明.txt", false, 3),
            "已取消选择 文档/说明.txt；共选择 3 个文件"
        );
    }

    #[test]
    fn status_semantics_interrupt_only_for_errors() {
        assert_eq!(
            status_semantics(StatusKind::Informational),
            ("status", "polite")
        );
        assert_eq!(status_semantics(StatusKind::Error), ("alert", "assertive"));
    }

    #[test]
    fn operation_footer_copy_is_precise_and_bilingual() {
        assert_eq!(OPERATION_PROGRESS_LIVE, "off");
        assert_eq!(
            operation_queue_summary(Locale::En, 0),
            "No operations queued"
        );
        assert_eq!(operation_queue_summary(Locale::En, 1), "1 operation queued");
        assert_eq!(operation_queue_summary(Locale::ZhCn, 3), "3 个操作排队");

        let byte_progress = ProgressSnapshot {
            processed_entries: 2,
            total_entries: 4,
            processed_bytes: 512,
            total_bytes: 1024,
        };
        assert_eq!(
            operation_progress_text(Locale::En, byte_progress),
            "50% · 512 B of 1.0 KB · 2 of 4 entries"
        );
        assert_eq!(
            operation_progress_text(Locale::ZhCn, byte_progress),
            "50% · 512 B / 1.0 KB · 2 / 4 项"
        );
        assert_eq!(
            operation_progress_text(Locale::En, ProgressSnapshot::default()),
            "Operation starting"
        );
    }

    #[test]
    fn create_source_accessibility_copy_identifies_paths_and_counts() {
        assert_eq!(
            create_source_summary(Locale::En, 1),
            "1 archive source added"
        );
        assert_eq!(
            create_source_summary(Locale::ZhCn, 3),
            "已添加 3 个压缩来源"
        );
        assert_eq!(
            create_source_remove_label(Locale::En, "docs/readme.txt"),
            "Remove archive source docs/readme.txt"
        );
        assert_eq!(
            create_source_removed_status(Locale::ZhCn, "文档/说明.txt", 2),
            "已移除压缩来源 文档/说明.txt；剩余 2 个"
        );
        assert_eq!(
            create_sources_added_status(Locale::En, 0, 2),
            "Added 0 archive sources; 2 total"
        );
    }

    #[test]
    fn accessible_shortcuts_are_deliberate_and_ime_safe() {
        assert_eq!(
            accessible_shortcut("o", true),
            Some(AccessibleShortcut::Open)
        );
        assert_eq!(
            accessible_shortcut("N", true),
            Some(AccessibleShortcut::Create)
        );
        assert_eq!(
            accessible_shortcut("Escape", false),
            Some(AccessibleShortcut::Cancel)
        );
        assert_eq!(accessible_shortcut("a", true), None);
        assert_eq!(accessible_shortcut("o", false), None);
        assert!(is_select_all_shortcut("a", true));
        assert!(is_select_all_shortcut("A", true));
        assert!(!is_select_all_shortcut("a", false));
        assert!(!is_select_all_shortcut("o", true));
    }
}
