#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::Duration;

use dioxus::prelude::*;
use dioxus_html::HasFileData;
use rfd::FileDialog;
use zifile_core::{
    ArchiveEntryInfo, ArchiveFormat, ArchiveInfo, CancellationToken, ConflictPolicy,
    OperationProgress, OperationSummary, SafetyLimits, detect_format_from_path,
};
use zifile_worker_protocol::WorkerRequest;

mod i18n;
mod settings;
mod taskbar;
mod worker_client;

use i18n::{Locale, Text};
use settings::AppSettings;
use worker_client::{WorkerOutput, run_worker};

const STYLES: &str = include_str!("accessible_ui.css");
const SECURITY_HEAD: &str = r#"<meta http-equiv="Content-Security-Policy" content="script-src 'unsafe-inline' 'unsafe-eval'; style-src 'unsafe-inline'; img-src data:; connect-src dioxus: ws://127.0.0.1:* http://dioxus.index.html https://dioxus.index.html ipc:; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'">"#;
const ENTRIES_PER_PAGE: usize = 500;
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccessibleShortcut {
    Open,
    Create,
    Cancel,
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
    busy: bool,
    cancellation: Option<CancellationToken>,
    progress: Option<OperationProgress>,
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
            busy: false,
            cancellation: None,
            progress: None,
            dark: settings.dark,
            locale: settings.locale,
            revision: 0,
        }
    }
}

#[component]
fn App() -> Element {
    let mut state = use_signal(UiState::default);
    let startup_archive = use_hook(|| std::env::args_os().nth(1).map(PathBuf::from));
    let mut startup_processed = use_signal(|| false);
    use_effect(move || {
        if !startup_processed() {
            startup_processed.set(true);
            if let Some(path) = startup_archive.clone() {
                begin_load(state, path);
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
                        onclick: move |_| { let mut value = state.write(); value.locale = value.locale.toggle(); value.status = value.locale.text(Text::Ready).to_owned(); save_settings(&value); },
                        {locale.text(Text::SwitchLanguage)} }
                }
            }
            main { id: "main-content", tabindex: "-1",
                match page {
                    Page::Home => rsx! { Home { state } },
                    Page::Archive => rsx! { ArchivePage { state } },
                    Page::Create => rsx! { CreatePage { state } },
                }
                footer { role: "status", "aria-live": "polite",
                    div { class: "status-copy", span { class: "status-dot", "aria-hidden": "true", "•" } span { {view.status.clone()} } }
                    if let Some(snapshot) = progress {
                        progress { max: "100", value: "{(snapshot.fraction() * 100.0).round() as u8}", "aria-label": choose(locale, "Operation progress", "操作进度") }
                    }
                    button { disabled: !view.busy, onclick: move |_| cancel_operation(state), {locale.text(Text::Cancel)} }
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
                button { class: "primary", disabled: view.busy, onclick: move |_| open_archive_dialog(state), {locale.text(Text::OpenAction)} } }
            article { h3 { {locale.text(Text::CreateArchive)} } p { {locale.text(Text::CreateDescription)} }
                button { class: "primary", disabled: view.busy, onclick: move |_| state.write().page = Page::Create, {locale.text(Text::StartCreating)} } }
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
        button { class: "primary", disabled: view.busy, onclick: move |_| open_archive_dialog(state), {locale.text(Text::OpenAction)} } } };
    };
    let filter = view.entry_filter.to_lowercase();
    let filtered = archive
        .entries
        .iter()
        .filter(|entry| entry_matches_filter(entry, &filter));
    let count = filtered.clone().count();
    let last_page = count.saturating_sub(1) / ENTRIES_PER_PAGE;
    let current_page = view.entry_page.min(last_page);
    let rows = filtered
        .skip(current_page * ENTRIES_PER_PAGE)
        .take(ENTRIES_PER_PAGE)
        .cloned()
        .collect::<Vec<_>>();
    let all_files = archive
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .count();
    let selected_count = view.selected.len();
    let archive_name = archive
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    rsx! { section { class: "archive-page", "aria-labelledby": "archive-title",
        div { class: "page-heading", div { h2 { id: "archive-title", {archive_name} } p { "{archive.format} · {archive.entries.len()} · {format_bytes(archive.total_size)}" } }
            div { class: "button-row", button { disabled: view.busy, onclick: move |_| open_archive_dialog(state), {locale.text(Text::OpenAnother)} }
                button { disabled: view.busy, onclick: move |_| test_archive(state), {locale.text(Text::TestArchive)} } } }
        div { class: "toolbar",
            label { span { {locale.text(Text::PasswordEncrypted)} } input { r#type: "password", value: view.password.clone(), disabled: view.busy, oninput: move |event| state.write().password = event.value() } }
            button { disabled: view.busy, onclick: move |_| reload_archive(state), {locale.text(Text::Reload)} }
            label { span { {locale.text(Text::Search)} } input { r#type: "search", value: view.entry_filter.clone(), oninput: move |event| { let mut value = state.write(); value.entry_filter = event.value(); value.entry_page = 0; } } }
        }
        div { class: "selection-bar",
            label { input { r#type: "checkbox", checked: selected_count == all_files && all_files > 0, onchange: move |event| select_all(state, event.checked()) } " {selected_count} {locale.text(Text::Selected)}" }
            div { class: "button-row",
                select { value: conflict_value(view.conflict), disabled: view.busy, "aria-label": choose(locale, "Conflict policy", "文件冲突策略"), onchange: move |event| state.write().conflict = parse_conflict(&event.value()),
                    for policy in [ConflictPolicy::Rename, ConflictPolicy::Overwrite, ConflictPolicy::Skip, ConflictPolicy::Error] { option { value: conflict_value(policy), {conflict_label(locale, policy)} } } }
                button { class: "primary", disabled: view.busy || selected_count == 0, onclick: move |_| extract_selected(state), {locale.text(Text::ExtractSelected)} }
            }
        }
        div { class: "table-wrap", tabindex: "0", role: "region", "aria-label": choose(locale, "Archive entries", "压缩文件项目"),
            table { thead { tr { th { "" } th { {locale.text(Text::Name)} } th { {locale.text(Text::Original)} } th { {locale.text(Text::Packed)} } th { {locale.text(Text::Flags)} } } }
                tbody { for entry in rows { ArchiveRow { key: "{entry.path.display()}", state, entry, locale } } } }
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
            onchange: move |event| { let mut value = state.write(); if event.checked() { value.selected.insert(path.clone()); } else { value.selected.remove(&path); } } } }
        td { class: "path-cell", {path_display} } td { {format_bytes(entry.size)} } td { {format_bytes(entry.compressed_size)} }
        td { if entry.encrypted { {locale.text(Text::Locked)} } else if entry.is_directory { {choose(locale, "Folder", "文件夹")} } else { "—" } }
    } }
}

#[component]
fn CreatePage(mut state: Signal<UiState>) -> Element {
    let view = state.read().clone();
    let locale = view.locale;
    let encrypted = view.create_format.capabilities().encryption;
    rsx! { section { class: "create-page", "aria-labelledby": "create-title",
        div { class: "page-heading", div { h2 { id: "create-title", {locale.text(Text::CreateHeading)} } p { {locale.text(Text::CreateHelp)} } }
            div { class: "button-row", button { disabled: view.busy, onclick: move |_| add_files(state), {locale.text(Text::AddFiles)} }
                button { disabled: view.busy, onclick: move |_| add_folder(state), {locale.text(Text::AddFolder)} }
                button { disabled: view.busy || view.create_sources.is_empty(), onclick: move |_| state.write().create_sources.clear(), {locale.text(Text::Clear)} } } }
        section { class: "source-list", "aria-label": choose(locale, "Archive sources", "压缩来源"),
            if view.create_sources.is_empty() { p { class: "muted", {locale.text(Text::NoSources)} } }
            ul { for (index, source) in view.create_sources.iter().enumerate() { li { key: "{source.display()}", span { {source.to_string_lossy().to_string()} }
                button { disabled: view.busy, onclick: move |_| { if index < state.read().create_sources.len() { state.write().create_sources.remove(index); } }, {locale.text(Text::Remove)} } } } }
        }
        div { class: "form-grid",
            label { span { {locale.text(Text::Format)} } select { value: format_value(view.create_format), disabled: view.busy,
                onchange: move |event| { let mut value = state.write(); value.create_format = parse_format(&event.value()); if !value.create_format.capabilities().encryption { value.create_password.clear(); } },
                for format in CREATE_FORMATS { option { value: format_value(format), "{format}" } } } }
            label { span { "{locale.text(Text::CompressionLevel)} · {view.compression_level}" } input { r#type: "range", min: "0", max: "9", value: "{view.compression_level}", disabled: view.busy,
                oninput: move |event| state.write().compression_level = event.value().parse().unwrap_or(6) } }
            label { span { if encrypted { {locale.text(Text::PasswordOptional)} } else { {locale.text(Text::PasswordUnavailable)} } }
                input { r#type: "password", placeholder: locale.text(Text::NoEncryption), value: view.create_password.clone(), disabled: view.busy || !encrypted, oninput: move |event| state.write().create_password = event.value() } }
        }
        div { class: "create-actions", button { class: "primary", disabled: view.busy || view.create_sources.is_empty(), onclick: move |_| create_archive(state), {locale.text(Text::CreateAction)} } }
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

fn apply_accessible_shortcut(mut state: Signal<UiState>, shortcut: AccessibleShortcut) {
    match shortcut {
        AccessibleShortcut::Open if !state.read().busy => open_archive_dialog(state),
        AccessibleShortcut::Create if !state.read().busy => state.write().page = Page::Create,
        AccessibleShortcut::Cancel => cancel_operation(state),
        _ => {}
    }
}

fn handle_dropped_paths(mut state: Signal<UiState>, paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    if state.read().busy {
        let locale = state.read().locale;
        state.write().status = choose(
            locale,
            "Wait for the current operation before dropping more files",
            "请等待当前操作完成后再拖入文件",
        )
        .to_owned();
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
    append_unique(&mut value.create_sources, existing);
    value.page = Page::Create;
    value.status = choose(locale, "Added dropped sources", "已添加拖入的来源").to_owned();
}

fn reload_archive(state: Signal<UiState>) {
    if let Some(path) = state
        .read()
        .archive
        .as_ref()
        .map(|archive| archive.path.clone())
    {
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
        append_unique(&mut state.write().create_sources, paths);
    }
}

fn add_folder(mut state: Signal<UiState>) {
    let locale = state.read().locale;
    if let Some(path) = FileDialog::new()
        .set_title(locale.text(Text::AddFolderDialog))
        .pick_folder()
    {
        append_unique(&mut state.write().create_sources, vec![path]);
    }
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
    if state.read().busy {
        return;
    }
    let progress = OperationProgress::default();
    let cancellation = CancellationToken::default();
    {
        let mut value = state.write();
        value.busy = true;
        value.status = status;
        value.progress = Some(progress.clone());
        value.cancellation = Some(cancellation.clone());
    }
    spawn(async move {
        let result =
            tokio::task::spawn_blocking(move || run_worker(request, progress, cancellation))
                .await
                .map_err(|error| format!("worker task failed: {error}"))
                .and_then(|result| result);
        finish_worker(state, kind, result);
    });
}

fn finish_worker(
    mut state: Signal<UiState>,
    kind: OperationKind,
    result: Result<WorkerOutput, String>,
) {
    let locale = state.read().locale;
    let status = match (kind, result) {
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
            status
        }
        (OperationKind::Test, Ok(WorkerOutput::Archive(info))) => {
            if locale == Locale::ZhCn {
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
            }
        }
        (OperationKind::Extract, Ok(WorkerOutput::Summary(summary))) => {
            summary_status(locale, summary, true)
        }
        (OperationKind::Create, Ok(WorkerOutput::Summary(summary))) => {
            summary_status(locale, summary, false)
        }
        (_, Ok(_)) => choose(
            locale,
            "Worker returned an unexpected result",
            "Worker 返回了意外结果",
        )
        .to_owned(),
        (kind, Err(error)) => format!(
            "{}: {error}",
            match kind {
                OperationKind::List => choose(locale, "Open failed", "打开失败"),
                OperationKind::Test => choose(locale, "Integrity test failed", "完整性校验失败"),
                OperationKind::Extract => choose(locale, "Extraction failed", "解压失败"),
                OperationKind::Create => choose(locale, "Creation failed", "创建失败"),
            }
        ),
    };
    let mut value = state.write();
    value.busy = false;
    value.cancellation = None;
    value.progress = None;
    value.status = status;
}

fn cancel_operation(mut state: Signal<UiState>) {
    let value = state.read();
    if let Some(cancellation) = &value.cancellation {
        cancellation.cancel();
    }
    let locale = value.locale;
    drop(value);
    state.write().status = choose(
        locale,
        "Cancelling safely after the current block…",
        "正在当前数据块结束后安全取消…",
    )
    .to_owned();
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
    state.write().selected = if selected { paths } else { HashSet::new() };
}

fn archive_dialog(locale: Locale) -> FileDialog {
    FileDialog::new()
        .set_title(locale.text(Text::OpenDialog))
        .add_filter(
            locale.text(Text::SupportedArchives),
            &[
                "zip", "7z", "tar", "gz", "tgz", "zst", "xz", "bz2", "lz4", "br",
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
fn entry_matches_filter(entry: &ArchiveEntryInfo, filter_lower: &str) -> bool {
    filter_lower.is_empty()
        || entry
            .path
            .to_string_lossy()
            .to_lowercase()
            .contains(filter_lower)
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
    }
}
