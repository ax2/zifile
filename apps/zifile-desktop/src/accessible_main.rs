#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
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
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::Duration;

use dioxus::prelude::*;
use dioxus_html::HasFileData;
use rfd::AsyncFileDialog;
use zifile_core::{
    ArchiveFormat, ArchiveInfo, CancellationToken, ConflictPolicy, CreateInputKind,
    OPEN_ARCHIVE_EXTENSIONS, OperationProgress, OperationSummary, ProgressSnapshot, SafetyLimits,
};
use zifile_worker_protocol::WorkerRequest;

mod i18n;
mod settings;
mod taskbar;
mod worker_client;

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
    BrowserEntry, DirectorySelection, ENTRIES_PER_PAGE, EntrySort, SortDirection,
    browser_entry_count, browser_entry_page, child_directory_selections, descendant_file_paths,
    directory_breadcrumbs, next_sort,
};
use zifile_desktop::operation_queue::{Job, OperationQueue, Submission};
use zifile_desktop::startup::{self, StartupRequest};
use zifile_desktop::{
    append_unique_paths as append_unique, ensure_archive_extension, invert_archive_file_selection,
    is_openable_archive_path, reveal_in_file_manager,
};

const STYLES: &str = include_str!("accessible_ui.css");
const ARCHIVE_FILTER_LIVE: &str = "off";
const OPERATION_PROGRESS_LIVE: &str = "off";
const ARIA_SHORTCUT_OPEN: &str = "Control+O";
const ARIA_SHORTCUT_CREATE: &str = "Control+N";
const ARIA_SHORTCUT_RELOAD: &str = "Control+R";
const ARIA_SHORTCUT_SEARCH: &str = "Control+F";
const ARIA_SHORTCUT_CLOSE: &str = "Control+W";
const ARIA_SHORTCUT_ABOUT: &str = "F1";
const ARIA_SHORTCUT_CANCEL: &str = "Escape";
const ARIA_SHORTCUT_SELECT_ALL: &str = "Control+A";
const ARIA_SHORTCUT_INVERT_SELECTION: &str = "Control+I";
const FOCUS_MAIN_SCRIPT: &str =
    "requestAnimationFrame(() => document.getElementById('main-content')?.focus())";
const FOCUS_ARCHIVE_SEARCH_SCRIPT: &str =
    "requestAnimationFrame(() => document.getElementById('archive-search')?.focus())";
const SECURITY_HEAD: &str = r#"<meta http-equiv="Content-Security-Policy" content="script-src 'unsafe-inline' 'unsafe-eval'; style-src 'unsafe-inline'; img-src data:; connect-src dioxus: ws://127.0.0.1:* http://dioxus.index.html https://dioxus.index.html ipc:; object-src 'none'; base-uri 'none'; form-action 'none'; frame-src 'none'">"#;
const CREATE_FORMATS: [ArchiveFormat; 15] = ArchiveFormat::CREATABLE;

fn main() {
    if std::env::args_os().any(|argument| argument == zifile_worker::WORKER_MODE_ARGUMENT) {
        zifile_worker::run_process();
        return;
    }
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
    About,
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

type SharedOperationQueue = Arc<Mutex<OperationQueue<QueuedOperation>>>;

fn lock_operation_queue(
    queue: &Mutex<OperationQueue<QueuedOperation>>,
) -> MutexGuard<'_, OperationQueue<QueuedOperation>> {
    queue
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AccessibleShortcut {
    Open,
    Create,
    Reload,
    Search,
    Close,
    InvertSelection,
    About,
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
    conflict: ConflictPolicy,
    create_sources: Vec<PathBuf>,
    create_format: ArchiveFormat,
    create_password: String,
    create_password_visible: bool,
    compression_level: u8,
    status: String,
    status_kind: StatusKind,
    busy: bool,
    dialog_open: bool,
    cancellation: Option<CancellationToken>,
    progress: Option<OperationProgress>,
    operations: SharedOperationQueue,
    recent_archives: Vec<PathBuf>,
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
            conflict: ConflictPolicy::Rename,
            create_sources: Vec::new(),
            create_format: ArchiveFormat::Zip,
            create_password: String::new(),
            create_password_visible: false,
            compression_level: 6,
            status: settings.locale.text(Text::Ready).to_owned(),
            status_kind: StatusKind::Informational,
            busy: false,
            dialog_open: false,
            cancellation: None,
            progress: None,
            operations: Arc::new(Mutex::new(OperationQueue::default())),
            recent_archives: settings.recent_archives,
            dark: settings.dark,
            locale: settings.locale,
            revision: 0,
        }
    }
}

impl UiState {
    fn clear_archive_password(&mut self) {
        self.password.clear();
        self.password_visible = false;
    }

    fn clear_create_password(&mut self) {
        self.create_password.clear();
        self.create_password_visible = false;
    }

    fn set_archive_password(&mut self, password: String) {
        self.password = password;
        if self.password.is_empty() {
            self.password_visible = false;
        }
    }

    fn set_create_password(&mut self, password: String) {
        self.create_password = password;
        if self.create_password.is_empty() {
            self.create_password_visible = false;
        }
    }

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
    let mut focused_page = use_signal(|| Page::Home);
    use_effect(move || {
        if !startup_processed() {
            startup_processed.set(true);
            match startup_request.clone() {
                StartupRequest::Home => {}
                StartupRequest::OpenArchive(path) => begin_load(state, path),
                StartupRequest::ExtractHere(path) => {
                    state.write().automatic_extract_destination =
                        Some(startup::extraction_destination(&path));
                    begin_load(state, path);
                }
                StartupRequest::CreateFrom(sources) => {
                    let mut value = state.write();
                    value.page = Page::Create;
                    append_unique(&mut value.create_sources, sources);
                }
            }
        }
    });
    use_effect(move || {
        let page = state.read().page;
        if focused_page() != page {
            focused_page.set(page);
            let _ = dioxus_document::eval(FOCUS_MAIN_SCRIPT);
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

    let (
        locale,
        page,
        dark,
        archive_loaded,
        busy,
        status_kind,
        status,
        progress,
        cancelled,
        queued_count,
        can_cancel,
        completed_output,
    ) = {
        let view = state.read();
        (
            view.locale,
            view.page,
            view.dark,
            view.archive.is_some(),
            view.busy,
            view.status_kind,
            view.status.clone(),
            view.progress.as_ref().map(OperationProgress::snapshot),
            view.cancellation
                .as_ref()
                .is_some_and(CancellationToken::is_cancelled),
            lock_operation_queue(&view.operations).pending_count(),
            view.cancellation.is_some(),
            view.completed_output.clone(),
        )
    };
    let theme = if dark { "dark" } else { "light" };
    let main_title = main_title_id(page, archive_loaded);
    let (status_role, status_live) = status_semantics(status_kind);
    let queue_summary = operation_queue_summary(locale, queued_count);
    let progress_view = progress.map(|snapshot| {
        (
            (snapshot.fraction() * 100.0).round() as u8,
            operation_progress_text(locale, snapshot),
        )
    });
    taskbar::sync(busy, cancelled, progress);

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
                if let Some(shortcut) =
                    accessible_shortcut(
                        &event.key().to_string(),
                        event.modifiers(),
                        can_cancel,
                        archive_loaded,
                        !busy,
                    )
                {
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
                    button { class: if page == Page::Create { "nav-item active" } else { "nav-item" }, "aria-current": if page == Page::Create { "page" } else { "false" }, "aria-keyshortcuts": ARIA_SHORTCUT_CREATE, onclick: move |_| state.write().page = Page::Create, {locale.text(Text::Create)} }
                    button { class: if page == Page::About { "nav-item active" } else { "nav-item" }, "aria-current": if page == Page::About { "page" } else { "false" }, "aria-keyshortcuts": ARIA_SHORTCUT_ABOUT, onclick: move |_| state.write().page = Page::About, {locale.text(Text::About)} }
                }
                div { class: "preferences",
                    button { onclick: move |_| { let mut value = state.write(); value.dark = !value.dark; save_settings(&mut value); },
                        {if dark { locale.text(Text::Light) } else { locale.text(Text::Dark) }} }
                    button { lang: if locale == Locale::ZhCn { "en-US" } else { "zh-CN" },
                        onclick: move |_| { let mut value = state.write(); value.locale = value.locale.toggle(); let status = value.locale.text(Text::Ready).to_owned(); value.set_status(status); save_settings(&mut value); },
                        {locale.text(Text::SwitchLanguage)} }
                }
            }
            main { id: "main-content", tabindex: "-1", "aria-labelledby": main_title,
                match page {
                    Page::Home => rsx! { Home { state } },
                    Page::Archive => rsx! { ArchivePage { state } },
                    Page::Create => rsx! { CreatePage { state } },
                    Page::About => rsx! { AboutPage { state } },
                }
                footer { class: if status_kind == StatusKind::Error { "status-error" } else { "" },
                    div { id: "operation-status", class: "status-copy", role: status_role, "aria-live": status_live, "aria-atomic": "true",
                        span { class: "status-dot", "aria-hidden": "true", "•" } span { {status} }
                    }
                    output { id: "operation-queue-summary", class: "queue-count", role: "status", "aria-live": "polite", "aria-atomic": "true", {queue_summary} }
                    if let Some((percent, progress_text)) = progress_view {
                        progress { max: "100", value: "{percent}", "aria-label": choose(locale, "Operation progress", "操作进度"), "aria-valuetext": progress_text, "aria-describedby": "operation-status", "aria-live": OPERATION_PROGRESS_LIVE }
                    }
                    button { class: "queue-clear", disabled: queued_count == 0, "aria-describedby": "operation-queue-summary", onclick: move |_| clear_queued(state), {choose(locale, "Clear queue", "清空队列")} }
                    button { disabled: !busy, "aria-describedby": "operation-status", "aria-keyshortcuts": ARIA_SHORTCUT_CANCEL, onclick: move |_| cancel_operation(state), {locale.text(Text::Cancel)} }
                    button { disabled: busy || completed_output.is_none(), "aria-describedby": "operation-status", onclick: move |_| reveal_completed_output(state), {locale.text(Text::RevealOutput)} }
                }
            }
        }
    }
}

const fn main_title_id(page: Page, archive_loaded: bool) -> &'static str {
    match page {
        Page::Home => "home-title",
        Page::Archive if archive_loaded => "archive-title",
        Page::Archive => "pending-archive-title",
        Page::Create => "create-title",
        Page::About => "about-title",
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
                button { class: "primary", "aria-keyshortcuts": ARIA_SHORTCUT_OPEN, onclick: move |_| open_archive_dialog(state), {locale.text(Text::OpenAction)} } }
            article { h3 { {locale.text(Text::CreateArchive)} } p { {locale.text(Text::CreateDescription)} }
                button { class: "primary", "aria-keyshortcuts": ARIA_SHORTCUT_CREATE, onclick: move |_| state.write().page = Page::Create, {locale.text(Text::StartCreating)} } }
        }
        section { class: "recent-archives", "aria-labelledby": "recent-archives-title",
            div { class: "page-heading", h3 { id: "recent-archives-title", {choose(locale, "Recent archives", "最近打开")} }
                button { disabled: view.busy || view.recent_archives.is_empty(), onclick: move |_| clear_recent_archives(state), {choose(locale, "Clear", "清空")} } }
            if view.recent_archives.is_empty() {
                p { class: "muted", {choose(locale, "Archives you successfully open will appear here.", "成功打开的压缩文件会显示在这里。")} }
            } else {
                ul { class: "recent-list",
                    for path in view.recent_archives.iter() {
                        li { key: "{path.display()}",
                            button { class: "recent-open", disabled: view.busy, title: "{path.display()}", onclick: { let path = path.clone(); move |_| open_recent_archive(state, path.clone()) }, {recent_archive_label(path)} }
                            button { disabled: view.busy, "aria-label": "{recent_archive_remove_label(locale, path)}", onclick: { let path = path.clone(); move |_| remove_recent_archive(state, path.clone()) }, {choose(locale, "Remove", "移除")} }
                        }
                    }
                }
            }
        }
        section { class: "privacy", "aria-labelledby": "privacy-title", h3 { id: "privacy-title", {locale.text(Text::Privacy)} } p { {locale.text(Text::PrivacyDescription)} } }
    } }
}

#[component]
fn AboutPage(state: Signal<UiState>) -> Element {
    let view = state.read();
    let locale = view.locale;
    rsx! { section { class: "home", "aria-labelledby": "about-title",
        h2 { id: "about-title", {locale.text(Text::AboutHeading)} }
        p { class: "lead", {locale.text(Text::AboutDescription)} }
        dl { class: "about-details",
            div { dt { {locale.text(Text::Version)} } dd { {env!("CARGO_PKG_VERSION")} } }
            div { dt { {locale.text(Text::License)} } dd { "MIT" } }
            div { dt { {locale.text(Text::SupportedFormatFamilies)} } dd { {ArchiveFormat::ALL.len().to_string()} } }
            div { dt { {locale.text(Text::ProjectWebsite)} } dd { "https://github.com/ax2/zifile" } }
        }
        section { class: "shortcut-help", "aria-labelledby": "shortcut-help-title",
            h3 { id: "shortcut-help-title", {locale.text(Text::KeyboardShortcuts)} }
            dl {
                div { dt { kbd { "Ctrl+O" } } dd { {locale.text(Text::ShortcutOpen)} } }
                div { dt { kbd { "Ctrl+N" } } dd { {locale.text(Text::ShortcutCreate)} } }
                div { dt { kbd { "Ctrl+R" } } dd { {locale.text(Text::ShortcutReload)} } }
                div { dt { kbd { "Ctrl+F" } } dd { {locale.text(Text::ShortcutSearch)} } }
                div { dt { kbd { "Ctrl+W" } } dd { {locale.text(Text::ShortcutClose)} } }
                div { dt { kbd { "Ctrl+A" } } dd { {locale.text(Text::ShortcutSelectAll)} } }
                div { dt { kbd { "Ctrl+I" } } dd { {locale.text(Text::ShortcutInvertSelection)} } }
                div { dt { kbd { "F1" } } dd { {locale.text(Text::ShortcutAbout)} } }
                div { dt { kbd { "Esc" } } dd { {locale.text(Text::ShortcutCancel)} } }
            }
        }
        section { class: "privacy", "aria-labelledby": "about-privacy-title",
            h3 { id: "about-privacy-title", {locale.text(Text::Privacy)} }
            p { {locale.text(Text::PrivacyDescription)} }
        }
    } }
}

#[component]
fn ArchivePage(mut state: Signal<UiState>) -> Element {
    let view = state.read();
    let locale = view.locale;
    let Some(archive) = view.archive.as_ref() else {
        let pending = view.pending_archive.clone();
        let heading = pending
            .as_ref()
            .and_then(|path| path.file_name())
            .map_or_else(
                || locale.text(Text::NoArchive).to_owned(),
                |name| name.to_string_lossy().into_owned(),
            );
        let description = archive_empty_state_description(
            locale,
            view.busy,
            pending.is_some(),
            view.pending_archive_requires_password,
        );
        let can_unlock = pending.is_some() && !view.busy;
        return rsx! { section { class: "empty-state", "aria-labelledby": "pending-archive-title",
            h2 { id: "pending-archive-title", {heading} }
            p { {description} }
            div { class: "button-row",
                if view.pending_archive_requires_password {
                    div { class: "password-field",
                        label { r#for: "pending-archive-password", span { {locale.text(Text::PasswordEncrypted)} }
                            input { id: "pending-archive-password", r#type: if view.password_visible { "text" } else { "password" }, autocomplete: "off", spellcheck: "false", value: view.password.clone(),
                                oninput: move |event| state.write().set_archive_password(event.value()),
                                onkeydown: move |event: KeyboardEvent| {
                                    if unlock_submit_key(
                                        &event.key().to_string(),
                                        event.modifiers(),
                                        event.is_composing(),
                                        can_unlock,
                                    ) {
                                        event.prevent_default();
                                        reload_archive(state);
                                    }
                                }
                            }
                        }
                        label { class: "password-toggle",
                            input { r#type: "checkbox", checked: view.password_visible, "aria-controls": "pending-archive-password", onchange: move |event| state.write().password_visible = event.checked() }
                            span { {locale.text(Text::ShowPassword)} }
                        }
                    }
                    button { class: "primary", disabled: !can_unlock, "aria-keyshortcuts": ARIA_SHORTCUT_RELOAD, onclick: move |_| reload_archive(state), {locale.text(Text::UnlockArchive)} }
                }
                button { "aria-keyshortcuts": ARIA_SHORTCUT_OPEN, onclick: move |_| open_archive_dialog(state), {locale.text(Text::OpenAction)} }
            }
        } };
    };
    if view.busy {
        let archive_name = archive
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        return rsx! { section { class: "archive-page", "aria-labelledby": "archive-title",
            div { class: "page-heading", div {
                h2 { id: "archive-title", {archive_name} }
                p { "{archive.format} · {archive.entries.len()} · {format_bytes(archive.total_size)}" }
            }
            div { class: "button-row",
                button { onclick: move |_| open_archive_dialog(state), {locale.text(Text::OpenAnother)} }
                button { onclick: move |_| reveal_archive(state), {locale.text(Text::RevealInExplorer)} }
                button { onclick: move |_| test_archive(state), {locale.text(Text::TestArchive)} }
                button { disabled: true, "aria-keyshortcuts": ARIA_SHORTCUT_CLOSE, {locale.text(Text::CloseArchive)} }
            } }
            section { class: "empty-state busy-archive", "aria-busy": "true",
                p { {locale.text(Text::BusyArchiveDescription)} }
            }
        } };
    }
    let count = browser_entry_count(archive, &view.entry_directory, &view.entry_filter);
    let last_page = count.saturating_sub(1) / ENTRIES_PER_PAGE;
    let current_page = view.entry_page.min(last_page);
    let rows = browser_entry_page(
        archive,
        &view.entry_directory,
        &view.entry_filter,
        current_page,
        view.entry_sort,
        view.entry_sort_direction,
    )
    .into_iter()
    .map(BrowserEntry::into_owned)
    .collect::<Vec<_>>();
    let directory_selections = if view.entry_filter.is_empty() {
        child_directory_selections(archive, &view.entry_directory, &view.selected)
    } else {
        Default::default()
    };
    let all_files = archive
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .count();
    let selected_count = view.selected.len();
    let all_selected = all_files > 0 && selected_count == all_files;
    let selection_summary = archive_selection_summary(locale, selected_count, all_files);
    let filter_summary =
        archive_filter_summary(locale, &view.entry_filter, count, archive.entries.len());
    let select_all_label = archive_select_all_label(locale, all_selected);
    let archive_name = archive
        .path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();

    rsx! { section { class: "archive-page", "aria-labelledby": "archive-title",
        div { class: "page-heading", div { h2 { id: "archive-title", {archive_name} } p { "{archive.format} · {archive.entries.len()} · {format_bytes(archive.total_size)}" } }
            div { class: "button-row",
                button { "aria-keyshortcuts": ARIA_SHORTCUT_OPEN, onclick: move |_| open_archive_dialog(state), {locale.text(Text::OpenAnother)} }
                button { onclick: move |_| reveal_archive(state), {locale.text(Text::RevealInExplorer)} }
                button { onclick: move |_| test_archive(state), {locale.text(Text::TestArchive)} }
                button { "aria-keyshortcuts": ARIA_SHORTCUT_CLOSE, onclick: move |_| close_archive(state), {locale.text(Text::CloseArchive)} }
            }
        }
        div { class: "toolbar",
            div { class: "password-field",
                label { r#for: "archive-password", span { {locale.text(Text::PasswordEncrypted)} }
                    input { id: "archive-password", r#type: if view.password_visible { "text" } else { "password" }, autocomplete: "off", spellcheck: "false", value: view.password.clone(), oninput: move |event| state.write().set_archive_password(event.value()) }
                }
                label { class: "password-toggle",
                    input { r#type: "checkbox", checked: view.password_visible, "aria-controls": "archive-password", onchange: move |event| state.write().password_visible = event.checked() }
                    span { {locale.text(Text::ShowPassword)} }
                }
            }
            button { "aria-keyshortcuts": ARIA_SHORTCUT_RELOAD, onclick: move |_| reload_archive(state), {locale.text(Text::Reload)} }
            div { class: "search-field",
                div { class: "search-row",
                    label { span { {locale.text(Text::Search)} }
                        input { id: "archive-search", r#type: "search", value: view.entry_filter.clone(), "aria-describedby": "archive-filter-summary", "aria-controls": "archive-entry-table", "aria-keyshortcuts": ARIA_SHORTCUT_SEARCH,
                            oninput: move |event| { let mut value = state.write(); value.entry_filter = event.value(); value.entry_page = 0; },
                            onkeydown: move |event: KeyboardEvent| {
                                if !event.is_composing() && event.key().to_string().eq_ignore_ascii_case("Enter") {
                                    announce_archive_filter(state);
                                }
                            }
                        }
                    }
                    button { disabled: view.entry_filter.is_empty(), "aria-describedby": "archive-filter-summary", onclick: move |_| clear_archive_filter(state), {locale.text(Text::ClearSearch)} }
                }
                output { id: "archive-filter-summary", class: "filter-summary", "aria-live": ARCHIVE_FILTER_LIVE, "aria-atomic": "true", {filter_summary} }
            }
        }
        div { class: "selection-bar",
            label { input { r#type: "checkbox", checked: all_selected, "aria-label": select_all_label, "aria-describedby": "archive-selection-summary", "aria-keyshortcuts": ARIA_SHORTCUT_SELECT_ALL, onchange: move |event| select_all(state, event.checked()) }
                output { id: "archive-selection-summary", role: "status", "aria-live": "polite", "aria-atomic": "true", {selection_summary.clone()} }
            }
            div { class: "button-row",
                button { "aria-describedby": "archive-selection-summary", onclick: move |_| select_all(state, true), {locale.text(Text::SelectAll)} }
                button { disabled: selected_count == 0, "aria-describedby": "archive-selection-summary", onclick: move |_| select_all(state, false), {locale.text(Text::SelectNone)} }
                button { "aria-describedby": "archive-selection-summary", "aria-keyshortcuts": ARIA_SHORTCUT_INVERT_SELECTION, onclick: move |_| invert_selection(state), {locale.text(Text::InvertSelection)} }
                select { value: conflict_value(view.conflict), "aria-label": choose(locale, "Conflict policy", "文件冲突策略"), onchange: move |event| state.write().conflict = parse_conflict(&event.value()),
                    for policy in [ConflictPolicy::Rename, ConflictPolicy::Overwrite, ConflictPolicy::Skip, ConflictPolicy::Error] { option { value: conflict_value(policy), {conflict_label(locale, policy)} } } }
                button { disabled: selected_count == 0, "aria-describedby": "archive-selection-summary", onclick: move |_| extract_selected(state), {locale.text(Text::ExtractSelected)} }
                button { "aria-describedby": "archive-selection-summary", onclick: move |_| extract_to_named_folder(state), {locale.text(Text::ExtractToNamedFolder)} }
                button { class: "primary", "aria-describedby": "archive-selection-summary", onclick: move |_| extract_all(state), {locale.text(Text::ExtractAll)} }
            }
        }
        nav { class: "breadcrumbs", "aria-label": choose(locale, "Archive folder", "归档文件夹"),
            button { "aria-current": if view.entry_directory.as_os_str().is_empty() { "page" } else { "false" }, onclick: move |_| navigate_archive_directory(state, PathBuf::new()), {choose(locale, "Archive root", "归档根目录")} }
            for (label, path) in directory_breadcrumbs(&view.entry_directory) {
                span { "›" }
                button { "aria-current": if path == view.entry_directory { "page" } else { "false" }, onclick: move |_| navigate_archive_directory(state, path.clone()), {label} }
            }
        }
        div { class: "table-wrap", tabindex: "0", role: "region", "aria-label": choose(locale, "Archive entries", "压缩文件项目"), "aria-describedby": "archive-selection-summary archive-filter-summary", "aria-keyshortcuts": ARIA_SHORTCUT_SELECT_ALL,
            onkeydown: move |event: KeyboardEvent| {
                if !event.is_composing()
                    && is_select_all_shortcut(&event.key().to_string(), event.modifiers())
                {
                    event.prevent_default();
                    select_all(state, true);
                }
            },
            table { id: "archive-entry-table",
                caption { class: "sr-only", {choose(locale, "Archive entry details and selection", "压缩文件项目详情与选择")} }
                thead { tr {
                    th { scope: "col", span { class: "sr-only", {choose(locale, "Select", "选择")} } }
                    th { scope: "col", "aria-sort": sort_aria(EntrySort::Name, view.entry_sort, view.entry_sort_direction),
                        button { class: "sort-header", onclick: move |_| set_entry_sort(state, EntrySort::Name), {sort_header_label(locale.text(Text::Name), EntrySort::Name, view.entry_sort, view.entry_sort_direction)} } }
                    th { scope: "col", "aria-sort": sort_aria(EntrySort::Size, view.entry_sort, view.entry_sort_direction),
                        button { class: "sort-header", onclick: move |_| set_entry_sort(state, EntrySort::Size), {sort_header_label(locale.text(Text::Original), EntrySort::Size, view.entry_sort, view.entry_sort_direction)} } }
                    th { scope: "col", "aria-sort": sort_aria(EntrySort::Packed, view.entry_sort, view.entry_sort_direction),
                        button { class: "sort-header", onclick: move |_| set_entry_sort(state, EntrySort::Packed), {sort_header_label(locale.text(Text::Packed), EntrySort::Packed, view.entry_sort, view.entry_sort_direction)} } }
                     th { scope: "col", "aria-sort": sort_aria(EntrySort::Modified, view.entry_sort, view.entry_sort_direction),
                         button { class: "sort-header", onclick: move |_| set_entry_sort(state, EntrySort::Modified), {sort_header_label(locale.text(Text::Modified), EntrySort::Modified, view.entry_sort, view.entry_sort_direction)} } }
                     th { scope: "col", {locale.text(Text::Flags)} }
                     th { scope: "col", {locale.text(Text::Checksum)} }
                } }
                tbody { for (index, entry) in rows.into_iter().enumerate() {
                    ArchiveRow { key: "{current_page}-{index}-{entry.path.display()}", state, directory_selection: directory_selections.get(entry.path.as_ref()).copied().unwrap_or_default(), entry, locale, show_full_path: !view.entry_filter.is_empty() }
                } }
            }
            if count == 0 { p { class: "empty-filter", {archive_no_matches(locale, &view.entry_filter)} } }
        }
        if count > 0 {
            nav { class: "pagination", "aria-label": choose(locale, "Entry pages", "项目分页"),
                button { disabled: current_page == 0, onclick: move |_| state.write().entry_page = current_page.saturating_sub(1), {locale.text(Text::Previous)} }
                span { "{locale.text(Text::Page)} {current_page + 1} / {last_page + 1}" }
                button { disabled: current_page >= last_page, onclick: move |_| state.write().entry_page = (current_page + 1).min(last_page), {locale.text(Text::Next)} }
            }
        }
    } }
}

fn set_entry_sort(mut state: Signal<UiState>, sort: EntrySort) {
    let mut value = state.write();
    (value.entry_sort, value.entry_sort_direction) =
        next_sort(value.entry_sort, value.entry_sort_direction, sort);
    value.entry_page = 0;
}

fn navigate_archive_directory(mut state: Signal<UiState>, directory: PathBuf) {
    let mut value = state.write();
    value.entry_directory = directory;
    value.entry_filter.clear();
    value.entry_page = 0;
}

fn sort_aria(column: EntrySort, active: EntrySort, direction: SortDirection) -> &'static str {
    if column != active {
        return "none";
    }
    match direction {
        SortDirection::Ascending => "ascending",
        SortDirection::Descending => "descending",
    }
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

#[component]
fn ArchiveRow(
    mut state: Signal<UiState>,
    entry: BrowserEntry<'static>,
    directory_selection: DirectorySelection,
    locale: Locale,
    show_full_path: bool,
) -> Element {
    let selected = state.read().selected.contains(entry.path.as_ref());
    let path = entry.path.as_ref().to_path_buf();
    let selection_path = path.clone();
    let directory_selection_path = path.clone();
    let path_display = if show_full_path {
        entry.path.to_string_lossy().into_owned()
    } else {
        entry
            .path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned()
    };
    let selection_label = if entry.is_directory {
        directory_selection_label(locale, &path_display, directory_selection)
    } else {
        format!("{} {path_display}", choose(locale, "Select", "选择"))
    };
    let open_folder_label = format!(
        "{} {path_display}",
        choose(locale, "Open folder", "打开文件夹")
    );
    rsx! { tr {
        td {
            if entry.is_directory {
                input { r#type: "checkbox", checked: directory_selection.all_selected(), disabled: directory_selection.total == 0,
                    "aria-checked": if directory_selection.partially_selected() { "mixed" } else if directory_selection.all_selected() { "true" } else { "false" },
                    "aria-label": selection_label,
                    onchange: move |event| toggle_archive_directory(state, directory_selection_path.clone(), event.checked()) }
            } else {
                input { r#type: "checkbox", checked: selected, "aria-label": selection_label,
                    onchange: move |event| update_archive_selection(state, selection_path.clone(), event.checked()) }
            }
        }
        td { class: "path-cell",
            if entry.is_directory {
                button { class: "folder-link", "aria-label": open_folder_label, onclick: move |_| navigate_archive_directory(state, path.clone()), "▸ {path_display}" }
            } else {
                "{path_display}"
            }
        } td { {format_bytes(entry.size)} } td { {format_bytes(entry.compressed_size)} }
        td { {format_archive_modified(locale, entry.modified.as_ref())} }
        td { if entry.encrypted { {locale.text(Text::Locked)} } else if entry.is_directory { {folder_selection_summary(locale, directory_selection)} } else { "—" } }
        td { class: "checksum-cell",
            if let Some(checksum) = entry.checksum.clone() {
                span { title: checksum.clone(), {checksum.clone()} }
                button { class: "copy-checksum", "aria-label": format!("{} {}", locale.text(Text::CopyChecksum), checksum), onclick: move |_| copy_checksum(state, checksum.clone()), {locale.text(Text::CopyChecksum)} }
            } else {
                "—"
            }
        }
    } }
}

fn copy_checksum(mut state: Signal<UiState>, checksum: String) {
    let locale = state.read().locale;
    let Ok(js_literal) = serde_json::to_string(&checksum) else {
        state
            .write()
            .set_error(choose(locale, "Could not copy checksum", "无法复制校验和").to_owned());
        return;
    };
    let script = format!(
        "return navigator.clipboard ? navigator.clipboard.writeText({js_literal}).then(() => true).catch(() => false) : false;"
    );
    spawn(async move {
        let copied = dioxus_document::eval(&script)
            .join::<bool>()
            .await
            .unwrap_or(false);
        let mut value = state.write();
        let status = if copied {
            value.locale.text(Text::ChecksumCopied).to_owned()
        } else {
            choose(value.locale, "Could not copy checksum", "无法复制校验和").to_owned()
        };
        if copied {
            value.set_status(status);
        } else {
            value.set_error(status);
        }
    });
}

#[component]
fn CreatePage(mut state: Signal<UiState>) -> Element {
    let view = state.read().clone();
    let locale = view.locale;
    let encrypted = view.create_format.capabilities().encryption;
    let source_summary = create_source_summary(locale, view.create_sources.len());
    let source_issue = create_source_issue(view.create_format, &view.create_sources);
    let single_file_format = view.create_format.create_input() == Some(CreateInputKind::SingleFile);
    let input_help = create_input_help(locale, view.create_format);
    let compression_range = view.create_format.compression_level_range();
    rsx! { section { class: "create-page", "aria-labelledby": "create-title",
        div { class: "page-heading", div { h2 { id: "create-title", {locale.text(Text::CreateHeading)} } p { {locale.text(Text::CreateHelp)} } }
            div { class: "button-row", button { onclick: move |_| add_files(state), {locale.text(Text::AddFiles)} }
                button { disabled: single_file_format, "aria-describedby": "create-format-help", onclick: move |_| add_folder(state), {locale.text(Text::AddFolder)} }
                button { disabled: view.create_sources.is_empty(), "aria-describedby": "create-source-summary", onclick: move |_| clear_create_sources(state), {locale.text(Text::Clear)} } } }
        section { class: "source-list", "aria-label": choose(locale, "Archive sources", "压缩来源"), "aria-describedby": "create-source-summary",
            output { id: "create-source-summary", class: "source-summary", role: "status", "aria-live": "polite", "aria-atomic": "true", {source_summary} }
            if view.create_sources.is_empty() { p { class: "muted", {locale.text(Text::NoSources)} } }
            ul { for source in view.create_sources.iter() { li { key: "{source.display()}", span { {source.to_string_lossy().to_string()} }
                button { "aria-label": create_source_remove_label(locale, &source.to_string_lossy()), onclick: { let source = source.clone(); move |_| remove_create_source(state, source.clone()) }, {locale.text(Text::Remove)} } } } }
        }
        div { class: "form-grid",
            label { span { {locale.text(Text::Format)} } select { value: format_value(view.create_format), "aria-describedby": "create-format-help",
                onchange: move |event| { let mut value = state.write(); set_create_format(&mut value, parse_format(&event.value())); },
                for format in CREATE_FORMATS { option { value: format_value(format), "{format}" } } } }
            if let Some((minimum, maximum)) = compression_range {
                label { span { "{locale.text(Text::CompressionLevel)} · {view.compression_level}" } input { r#type: "range", min: "{minimum}", max: "{maximum}", value: "{view.compression_level}",
                    oninput: move |event| { let mut value = state.write(); value.compression_level = value.create_format.clamp_compression_level(event.value().parse().unwrap_or(6)); } } }
            } else {
                p { class: "muted", {locale.text(Text::CompressionFixed)} }
            }
            div { class: "password-field",
                label { r#for: "create-password", span { if encrypted { {locale.text(Text::PasswordOptional)} } else { {locale.text(Text::PasswordUnavailable)} } }
                    input { id: "create-password", r#type: if view.create_password_visible { "text" } else { "password" }, autocomplete: "off", spellcheck: "false", placeholder: locale.text(Text::NoEncryption), value: view.create_password.clone(), disabled: !encrypted, oninput: move |event| state.write().set_create_password(event.value()) }
                }
                label { class: "password-toggle",
                    input { r#type: "checkbox", checked: view.create_password_visible, disabled: !encrypted, "aria-controls": "create-password", onchange: move |event| state.write().create_password_visible = event.checked() }
                    span { {locale.text(Text::ShowPassword)} }
                }
            }
        }
        output { id: "create-format-help", class: "muted", role: "status", "aria-live": "off", {input_help} }
        div { class: "create-actions", button { class: "primary", disabled: source_issue.is_some(), "aria-describedby": "create-source-summary create-format-help", onclick: move |_| create_archive(state), {locale.text(Text::CreateAction)} } }
    } }
}

fn open_archive_dialog(mut state: Signal<UiState>) {
    if state.read().dialog_open {
        return;
    }
    let locale = state.read().locale;
    state.write().dialog_open = true;
    spawn(async move {
        let path = archive_dialog(locale)
            .pick_file()
            .await
            .map(|file| file.path().to_path_buf());
        let mut value = state.write();
        value.dialog_open = false;
        if let Some(path) = path {
            value.clear_archive_password();
            value.automatic_extract_destination = None;
            drop(value);
            begin_load(state, path);
        }
    });
}

fn accessible_shortcut(
    key: &str,
    modifiers: Modifiers,
    cancellation_available: bool,
    archive_search_available: bool,
    archive_close_available: bool,
) -> Option<AccessibleShortcut> {
    let modifiers = shortcut_modifiers(modifiers);
    if key.eq_ignore_ascii_case("escape") {
        return (cancellation_available && modifiers.is_empty())
            .then_some(AccessibleShortcut::Cancel);
    }
    if key.eq_ignore_ascii_case("f1") {
        return modifiers.is_empty().then_some(AccessibleShortcut::About);
    }
    if modifiers != Modifiers::CONTROL {
        return None;
    }
    match key.to_ascii_lowercase().as_str() {
        "o" => Some(AccessibleShortcut::Open),
        "n" => Some(AccessibleShortcut::Create),
        "r" => Some(AccessibleShortcut::Reload),
        "f" if archive_search_available => Some(AccessibleShortcut::Search),
        "i" if archive_search_available => Some(AccessibleShortcut::InvertSelection),
        "w" if archive_search_available && archive_close_available => {
            Some(AccessibleShortcut::Close)
        }
        _ => None,
    }
}

fn unlock_submit_key(key: &str, modifiers: Modifiers, composing: bool, enabled: bool) -> bool {
    enabled
        && !composing
        && shortcut_modifiers(modifiers).is_empty()
        && key.eq_ignore_ascii_case("Enter")
}

fn shortcut_modifiers(modifiers: Modifiers) -> Modifiers {
    let lock_modifiers = Modifiers::CAPS_LOCK
        | Modifiers::FN_LOCK
        | Modifiers::NUM_LOCK
        | Modifiers::SCROLL_LOCK
        | Modifiers::SYMBOL_LOCK;
    modifiers & !lock_modifiers
}

fn is_select_all_shortcut(key: &str, modifiers: Modifiers) -> bool {
    shortcut_modifiers(modifiers) == Modifiers::CONTROL && key.eq_ignore_ascii_case("a")
}

fn apply_accessible_shortcut(mut state: Signal<UiState>, shortcut: AccessibleShortcut) {
    match shortcut {
        AccessibleShortcut::Open => open_archive_dialog(state),
        AccessibleShortcut::Create => state.write().page = Page::Create,
        AccessibleShortcut::Reload => reload_archive(state),
        AccessibleShortcut::Search => focus_archive_search(state),
        AccessibleShortcut::Close => close_archive(state),
        AccessibleShortcut::InvertSelection => invert_selection(state),
        AccessibleShortcut::About => state.write().page = Page::About,
        AccessibleShortcut::Cancel => cancel_operation(state),
    }
}

fn close_archive(mut state: Signal<UiState>) {
    if state.read().busy || state.read().archive.is_none() {
        return;
    }
    let locale = state.read().locale;
    let mut value = state.write();
    clear_archive_session(&mut value);
    value.set_status(locale.text(Text::ArchiveClosed).to_owned());
}

fn clear_archive_session(value: &mut UiState) {
    value.archive = None;
    value.pending_archive = None;
    value.pending_archive_requires_password = false;
    value.automatic_extract_destination = None;
    value.selected.clear();
    value.entry_directory.clear();
    value.entry_filter.clear();
    value.entry_page = 0;
    value.entry_sort = EntrySort::default();
    value.entry_sort_direction = SortDirection::default();
    value.clear_archive_password();
    value.page = Page::Home;
}

fn focus_archive_search(mut state: Signal<UiState>) {
    if state.read().archive.is_none() {
        return;
    }
    state.write().page = Page::Archive;
    let _ = dioxus_document::eval(FOCUS_ARCHIVE_SEARCH_SCRIPT);
}

fn single_openable_archive(paths: &[PathBuf], openable: bool) -> Option<&Path> {
    let [path] = paths else {
        return None;
    };
    openable.then_some(path.as_path())
}

fn handle_dropped_paths(state: Signal<UiState>, paths: Vec<PathBuf>) {
    if paths.is_empty() {
        return;
    }
    if paths.len() == 1 {
        let Some(path) = paths.first().cloned() else {
            return;
        };
        spawn(async move {
            let probe_path = path.clone();
            let openable =
                tokio::task::spawn_blocking(move || is_openable_archive_path(&probe_path))
                    .await
                    .unwrap_or(false);
            handle_classified_drop(state, vec![path], openable);
        });
        return;
    }
    handle_classified_drop(state, paths, false);
}

fn handle_classified_drop(mut state: Signal<UiState>, paths: Vec<PathBuf>, openable: bool) {
    if let Some(path) = single_openable_archive(&paths, openable).map(Path::to_path_buf) {
        let mut value = state.write();
        value.clear_archive_password();
        value.automatic_extract_destination = None;
        drop(value);
        begin_load(state, path);
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
        value
            .archive
            .as_ref()
            .map(|archive| archive.path.clone())
            .or_else(|| value.pending_archive.clone())
    };
    if let Some(path) = path {
        begin_load(state, path);
    }
}

fn reveal_archive(mut state: Signal<UiState>) {
    let value = state.read();
    let Some(path) = value.archive.as_ref().map(|archive| archive.path.clone()) else {
        return;
    };
    let locale = value.locale;
    drop(value);
    let result = reveal_in_file_manager(&path);
    let mut value = state.write();
    match result {
        Ok(()) => value.set_status(locale.text(Text::RevealedInExplorer).to_owned()),
        Err(_) => value.set_error(locale.text(Text::RevealInExplorerFailed).to_owned()),
    }
}

fn reveal_completed_output(mut state: Signal<UiState>) {
    let value = state.read();
    let Some(path) = value.completed_output.clone() else {
        return;
    };
    let locale = value.locale;
    drop(value);
    let result = reveal_in_file_manager(&path);
    let mut value = state.write();
    match result {
        Ok(()) => value.set_status(locale.text(Text::OutputRevealed).to_owned()),
        Err(_) => value.set_error(locale.text(Text::RevealInExplorerFailed).to_owned()),
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
    extract_with_scope(state, false);
}

fn extract_all(state: Signal<UiState>) {
    extract_with_scope(state, true);
}

fn extract_to_named_folder(state: Signal<UiState>) {
    let destination = {
        let value = state.read();
        let Some(archive) = value.archive.as_ref() else {
            return;
        };
        startup::extraction_destination(&archive.path)
    };
    extract_to(state, destination, true);
}

fn extract_with_scope(mut state: Signal<UiState>, extract_all: bool) {
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
    drop(value);
    if state.read().dialog_open {
        return;
    }
    state.write().dialog_open = true;
    let dialog = AsyncFileDialog::new()
        .set_title(locale.text(Text::ChooseExtractionFolder))
        .set_directory(default_folder.parent().unwrap_or_else(|| Path::new(".")));
    spawn(async move {
        let destination = dialog
            .pick_folder()
            .await
            .map(|folder| folder.path().to_path_buf());
        state.write().dialog_open = false;
        if let Some(destination) = destination {
            extract_to(state, destination, extract_all);
        }
    });
}

fn extract_to(state: Signal<UiState>, destination: PathBuf, extract_all: bool) {
    let value = state.read();
    let Some(archive) = value.archive.clone() else {
        return;
    };
    let selected = value.selected.clone();
    let conflict = value.conflict;
    let password = non_empty(&value.password);
    let locale = value.locale;
    drop(value);
    let file_count = archive
        .entries
        .iter()
        .filter(|entry| !entry.is_directory)
        .count();
    let selected_paths =
        (!extract_all && selected.len() != file_count).then(|| selected.into_iter().collect());
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
    if state.read().dialog_open {
        return;
    }
    let locale = state.read().locale;
    let single_file_format =
        state.read().create_format.create_input() == Some(CreateInputKind::SingleFile);
    state.write().dialog_open = true;
    let dialog = AsyncFileDialog::new().set_title(locale.text(Text::AddFilesDialog));
    spawn(async move {
        let paths = if single_file_format {
            dialog
                .pick_file()
                .await
                .map(|file| vec![file.path().to_path_buf()])
        } else {
            dialog.pick_files().await.map(|files| {
                files
                    .into_iter()
                    .map(|file| file.path().to_path_buf())
                    .collect()
            })
        };
        let mut value = state.write();
        value.dialog_open = false;
        if let Some(paths) = paths {
            let added = append_unique(&mut value.create_sources, paths);
            let status = create_sources_added_status(locale, added, value.create_sources.len());
            value.set_status(status);
        }
    });
}

fn add_folder(mut state: Signal<UiState>) {
    if state.read().dialog_open {
        return;
    }
    if state.read().create_format.create_input() == Some(CreateInputKind::SingleFile) {
        let locale = state.read().locale;
        state
            .write()
            .set_error(locale.text(Text::SingleFileRequired).to_owned());
        return;
    }
    let locale = state.read().locale;
    state.write().dialog_open = true;
    let dialog = AsyncFileDialog::new().set_title(locale.text(Text::AddFolderDialog));
    spawn(async move {
        let path = dialog
            .pick_folder()
            .await
            .map(|folder| folder.path().to_path_buf());
        let mut value = state.write();
        value.dialog_open = false;
        if let Some(path) = path {
            let added = append_unique(&mut value.create_sources, vec![path]);
            let status = create_sources_added_status(locale, added, value.create_sources.len());
            value.set_status(status);
        }
    });
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
    let locale = value.locale;
    value.create_sources.clear();
    value.set_status(create_sources_cleared_status(locale, cleared));
}

fn create_archive(mut state: Signal<UiState>) {
    let value = state.read();
    if let Some(issue) = create_source_issue(value.create_format, &value.create_sources) {
        let message = create_source_issue_text(value.locale, issue).to_owned();
        drop(value);
        state.write().set_error(message);
        return;
    }
    let locale = value.locale;
    let format = value.create_format;
    let sources = value.create_sources.clone();
    let compression_level = value.compression_level;
    let password = non_empty(&value.create_password);
    drop(value);
    if state.read().dialog_open {
        return;
    }
    let extension = format.canonical_extension();
    state.write().dialog_open = true;
    let dialog = AsyncFileDialog::new()
        .set_title(locale.text(Text::CreateDialog))
        .add_filter(format.to_string(), &[extension])
        .set_file_name(format!("archive.{extension}"));
    spawn(async move {
        let destination = dialog
            .save_file()
            .await
            .map(|file| file.path().to_path_buf());
        state.write().dialog_open = false;
        let Some(destination) = destination else {
            return;
        };
        let destination = ensure_archive_extension(destination, format);
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
    });
}

fn create_input_help(locale: Locale, format: ArchiveFormat) -> &'static str {
    match format.create_input() {
        Some(CreateInputKind::FilesAndDirectories) => locale.text(Text::FilesAndFoldersSupported),
        Some(CreateInputKind::SingleFile) => locale.text(Text::SingleFileRequired),
        None => locale.text(Text::FormatCannotCreate),
    }
}

fn set_create_format(state: &mut UiState, format: ArchiveFormat) {
    state.create_format = format;
    state.compression_level = format.clamp_compression_level(state.compression_level);
    if !format.capabilities().encryption {
        state.clear_create_password();
    }
    state.set_status(create_input_help(state.locale, format).to_owned());
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

fn launch_worker(
    mut state: Signal<UiState>,
    request: WorkerRequest,
    kind: OperationKind,
    status: String,
) {
    let submitted_kind = kind;
    let archive_path = match &request {
        WorkerRequest::List { archive, .. } => Some(archive.clone()),
        _ => None,
    };
    let operation = QueuedOperation {
        kind,
        request,
        status,
        archive_path,
    };
    let operations = state.read().operations.clone();
    let submission = lock_operation_queue(&operations).submit(operation);
    match submission {
        Ok(Submission::Start(job)) => {
            clear_submitted_create_password(&mut state.write(), submitted_kind, true);
            start_worker(state, job);
        }
        Ok(Submission::Queued { position, .. }) => {
            clear_submitted_create_password(&mut state.write(), submitted_kind, true);
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
            clear_submitted_create_password(&mut state.write(), submitted_kind, false);
            let locale = state.read().locale;
            let status = match locale {
                Locale::En => format!("Operation queue is full (maximum {})", error.capacity),
                Locale::ZhCn => format!("操作队列已满（最多 {} 个）", error.capacity),
            };
            state.write().set_error(status);
        }
    }
}

fn clear_submitted_create_password(state: &mut UiState, kind: OperationKind, accepted: bool) {
    if accepted && kind == OperationKind::Create {
        state.clear_create_password();
    }
}

fn start_worker(mut state: Signal<UiState>, job: Job<QueuedOperation>) {
    let Job { id, payload } = job;
    let QueuedOperation {
        kind,
        request,
        status,
        archive_path,
    } = payload;
    let completed_output = operation_output_path(&request);
    let progress = OperationProgress::default();
    let cancellation = CancellationToken::default();
    {
        let mut value = state.write();
        if let Some(path) = archive_path {
            value.archive = None;
            value.pending_archive = Some(path);
            value.pending_archive_requires_password = false;
            value.selected.clear();
            value.page = Page::Archive;
        }
        value.busy = true;
        value.completed_output = None;
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
        finish_worker(state, id, kind, completed_output, result);
    });
}

fn finish_worker(
    mut state: Signal<UiState>,
    id: u64,
    kind: OperationKind,
    completed_output: Option<PathBuf>,
    result: Result<WorkerOutput, String>,
) {
    let operations = state.read().operations.clone();
    if lock_operation_queue(&operations).active_id() != Some(id) {
        return;
    }
    let locale = state.read().locale;
    let requires_password = kind == OperationKind::List
        && result
            .as_ref()
            .err()
            .is_some_and(|error| worker_error_may_require_password(error));
    if kind == OperationKind::List {
        state.write().pending_archive_requires_password = requires_password;
    }
    let succeeded = result.is_ok();
    let mut automatic_extract = None;
    let (status, status_kind) = match (kind, result) {
        (OperationKind::List, Ok(WorkerOutput::Archive(archive))) => {
            let recent_path = archive.path.clone();
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
            record_recent_archive(&mut value, recent_path);
            value.pending_archive = None;
            value.pending_archive_requires_password = false;
            value.selected = selected;
            value.entry_filter.clear();
            value.entry_directory.clear();
            value.entry_page = 0;
            value.entry_sort = EntrySort::Name;
            value.entry_sort_direction = SortDirection::Ascending;
            value.page = Page::Archive;
            automatic_extract = value.automatic_extract_destination.take();
            (status, StatusKind::Informational)
        }
        (OperationKind::Test, Ok(WorkerOutput::Archive(info))) => {
            let checksum_count = info
                .entries
                .iter()
                .filter(|entry| entry.checksum.is_some())
                .count();
            let status = if locale == Locale::ZhCn {
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
            state.write().archive = Some(info);
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
                "{}: {}",
                match kind {
                    OperationKind::List => choose(locale, "Open failed", "打开失败"),
                    OperationKind::Test => {
                        choose(locale, "Integrity test failed", "完整性校验失败")
                    }
                    OperationKind::Extract => choose(locale, "Extraction failed", "解压失败"),
                    OperationKind::Create => choose(locale, "Creation failed", "创建失败"),
                },
                format_worker_error(locale, &error)
            ),
            StatusKind::Error,
        ),
    };
    if matches!(kind, OperationKind::Extract | OperationKind::Create)
        && succeeded
        && status_kind == StatusKind::Informational
    {
        state.write().completed_output = completed_output;
    }
    if let Some(destination) = automatic_extract {
        extract_to(state, destination, true);
    }
    let next = lock_operation_queue(&operations).complete(id);
    match next {
        Ok(Some(job)) => start_worker(state, job),
        Ok(None) => {
            let mut value = state.write();
            value.busy = false;
            value.cancellation = None;
            value.progress = None;
            value.status = status;
            value.status_kind = status_kind;
        }
        Err(error) => {
            let mut value = state.write();
            value.busy = false;
            value.cancellation = None;
            value.progress = None;
            value.set_error(format!("Internal operation queue error: {error}"));
        }
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

fn clear_queued(mut state: Signal<UiState>) {
    let operations = state.read().operations.clone();
    let cleared = lock_operation_queue(&operations).clear_pending().len();
    let locale = state.read().locale;
    let status = match locale {
        Locale::En => format!("Cleared {cleared} queued operations"),
        Locale::ZhCn => format!("已清除 {cleared} 个排队操作"),
    };
    state.write().set_status(status);
}

fn cancel_operation(mut state: Signal<UiState>) {
    let value = state.read();
    let Some(cancellation) = &value.cancellation else {
        return;
    };
    cancellation.cancel();
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

fn invert_selection(mut state: Signal<UiState>) {
    let mut value = state.write();
    let UiState {
        archive, selected, ..
    } = &mut *value;
    let Some(archive) = archive.as_ref() else {
        return;
    };
    let selected = invert_archive_file_selection(archive, selected);
    let status = match value.locale {
        Locale::En => format!("Selection inverted; {selected} files selected"),
        Locale::ZhCn => format!("已反选；当前选择 {selected} 个文件"),
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

fn toggle_archive_directory(mut state: Signal<UiState>, directory: PathBuf, selected: bool) {
    let descendants = state
        .read()
        .archive
        .as_ref()
        .map_or_else(Vec::new, |archive| {
            descendant_file_paths(archive, &directory)
        });
    let mut value = state.write();
    if selected {
        value.selected.extend(descendants.iter().cloned());
    } else {
        for path in &descendants {
            value.selected.remove(path);
        }
    }
    let status = match value.locale {
        Locale::En => format!(
            "{} {} files in {}",
            if selected { "Selected" } else { "Cleared" },
            descendants.len(),
            directory.display()
        ),
        Locale::ZhCn => format!(
            "{} {} 中的 {} 个文件",
            if selected { "已选择" } else { "已清除" },
            directory.display(),
            descendants.len()
        ),
    };
    value.set_status(status);
}

fn directory_selection_label(locale: Locale, path: &str, selection: DirectorySelection) -> String {
    match locale {
        Locale::En => format!(
            "{} folder {path}; {} of {} files selected",
            if selection.all_selected() {
                "Clear"
            } else {
                "Select"
            },
            selection.selected,
            selection.total
        ),
        Locale::ZhCn => format!(
            "{}文件夹 {path}；已选择 {}/{} 个文件",
            if selection.all_selected() {
                "清除"
            } else {
                "选择"
            },
            selection.selected,
            selection.total
        ),
    }
}

fn folder_selection_summary(locale: Locale, selection: DirectorySelection) -> String {
    match locale {
        Locale::En => format!("{}/{} selected", selection.selected, selection.total),
        Locale::ZhCn => format!("已选 {}/{}", selection.selected, selection.total),
    }
}

fn announce_archive_filter(mut state: Signal<UiState>) {
    let value = state.read();
    let Some(archive) = value.archive.as_ref() else {
        return;
    };
    let status = archive_filter_summary(
        value.locale,
        &value.entry_filter,
        browser_entry_count(archive, &value.entry_directory, &value.entry_filter),
        archive.entries.len(),
    );
    drop(value);
    state.write().set_status(status);
}

fn clear_archive_filter(mut state: Signal<UiState>) {
    let mut value = state.write();
    value.entry_filter.clear();
    value.entry_page = 0;
    let (visible, total) = value.archive.as_ref().map_or((0, 0), |archive| {
        (
            browser_entry_count(archive, &value.entry_directory, ""),
            archive.entries.len(),
        )
    });
    let status = archive_filter_summary(value.locale, "", visible, total);
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

fn create_source_remove_label(locale: Locale, path: &str) -> String {
    match locale {
        Locale::En => format!("Remove archive source {path}"),
        Locale::ZhCn => format!("移除压缩来源 {path}"),
    }
}

fn archive_dialog(locale: Locale) -> AsyncFileDialog {
    AsyncFileDialog::new()
        .set_title(locale.text(Text::OpenDialog))
        .add_filter(
            locale.text(Text::SupportedArchives),
            OPEN_ARCHIVE_EXTENSIONS,
        )
        .add_filter(locale.text(Text::AllFiles), &["*"])
}

fn save_settings(state: &mut UiState) {
    if let Err(error) = (AppSettings {
        locale: state.locale,
        dark: state.dark,
        recent_archives: state.recent_archives.clone(),
    }
    .save())
    {
        state.set_error(format!(
            "{}: {error}",
            state.locale.text(Text::PreferencesSaveFailed)
        ));
    }
}

fn record_recent_archive(state: &mut UiState, path: PathBuf) {
    let mut settings = AppSettings {
        locale: state.locale,
        dark: state.dark,
        recent_archives: std::mem::take(&mut state.recent_archives),
    };
    settings.record_recent_archive(path);
    state.recent_archives = settings.recent_archives;
    save_settings(state);
}

fn open_recent_archive(mut state: Signal<UiState>, path: PathBuf) {
    if state.read().busy {
        return;
    }
    let mut value = state.write();
    value.clear_archive_password();
    value.automatic_extract_destination = None;
    drop(value);
    begin_load(state, path);
}

fn clear_recent_archives(mut state: Signal<UiState>) {
    if state.read().busy {
        return;
    }
    let mut value = state.write();
    value.recent_archives.clear();
    save_settings(&mut value);
    let status = choose(
        value.locale,
        "Recent archives cleared",
        "最近打开记录已清空",
    )
    .to_owned();
    value.set_status(status);
}

fn remove_recent_archive(mut state: Signal<UiState>, path: PathBuf) {
    if state.read().busy {
        return;
    }
    let mut value = state.write();
    let mut settings = AppSettings {
        locale: value.locale,
        dark: value.dark,
        recent_archives: std::mem::take(&mut value.recent_archives),
    };
    settings.remove_recent_archive(&path);
    value.recent_archives = settings.recent_archives;
    save_settings(&mut value);
    let status = choose(value.locale, "Recent archive removed", "已移除最近打开记录").to_owned();
    value.set_status(status);
}

fn recent_archive_remove_label(locale: Locale, path: &Path) -> String {
    match locale {
        Locale::En => format!("Remove recent archive {}", path.display()),
        Locale::ZhCn => format!("移除最近打开记录 {}", path.display()),
    }
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
    if snapshot.total_bytes == 0 && snapshot.processed_bytes > 0 {
        return match locale {
            Locale::En => format!("Scanning · {} read", format_bytes(snapshot.processed_bytes)),
            Locale::ZhCn => format!(
                "正在扫描 · 已读取 {}",
                format_bytes(snapshot.processed_bytes)
            ),
        };
    }
    if snapshot.total_entries == 0 && snapshot.processed_entries > 0 {
        return match locale {
            Locale::En => format!("Scanning · {} entries found", snapshot.processed_entries),
            Locale::ZhCn => format!("正在扫描 · 已发现 {} 项", snapshot.processed_entries),
        };
    }
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

    #[test]
    fn recent_archive_controls_are_semantic_and_busy_safe() {
        let source = include_str!("accessible_main.rs");
        assert!(source.contains("\"aria-labelledby\": \"recent-archives-title\""));
        assert!(source.contains("disabled: view.busy || view.recent_archives.is_empty()"));
        assert!(source.contains("disabled: view.busy, title: \"{path.display()}\""));
        assert!(source.contains("record_recent_archive(&mut value, recent_path)"));
        assert!(source.contains("remove_recent_archive(state, path.clone())"));
    }

    #[test]
    fn clearing_archive_session_releases_sensitive_and_navigation_state() {
        let mut state = UiState {
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
            password_visible: true,
            locale: Locale::En,
            ..UiState::default()
        };

        clear_archive_session(&mut state);

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
        assert!(!state.password_visible);
    }

    #[test]
    fn operation_queue_recovers_after_a_poisoned_lock() {
        let queue: SharedOperationQueue = Arc::new(Mutex::new(OperationQueue::default()));
        let poisoned = queue.clone();
        let _ = std::thread::spawn(move || {
            let _guard = poisoned.lock().expect("test lock should start healthy");
            panic!("poison operation queue for recovery coverage");
        })
        .join();

        assert_eq!(lock_operation_queue(&queue).pending_count(), 0);
    }

    #[test]
    fn only_one_existing_supported_archive_opens_from_a_drop() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let archive = temporary.path().join("sample.zip");
        let plain_file = temporary.path().join("notes.txt");
        let archive_named_directory = temporary.path().join("folder.zip");
        std::fs::write(&archive, b"not parsed by this classification test")
            .expect("archive fixture");
        std::fs::write(&plain_file, b"plain fixture").expect("plain fixture");
        std::fs::create_dir(&archive_named_directory).expect("directory fixture");

        assert_eq!(
            single_openable_archive(std::slice::from_ref(&archive), true),
            Some(archive.as_path())
        );
        assert_eq!(single_openable_archive(&[], true), None);
        assert_eq!(
            single_openable_archive(&[archive.clone(), plain_file.clone()], true),
            None
        );
        assert_eq!(single_openable_archive(&[plain_file], false), None);
        assert_eq!(
            single_openable_archive(&[archive_named_directory], false),
            None
        );
    }

    #[test]
    fn format_values_round_trip() {
        for format in CREATE_FORMATS {
            assert_eq!(parse_format(format_value(format)), format);
        }
    }

    #[test]
    fn create_input_guidance_is_bilingual_and_matches_capabilities() {
        assert_eq!(
            create_input_help(Locale::En, ArchiveFormat::Zip),
            "This format accepts files and folders."
        );
        assert!(create_input_help(Locale::ZhCn, ArchiveFormat::Gzip).contains("只能压缩一个文件"));
        assert_eq!(
            create_source_issue_text(Locale::En, CreateSourceIssue::UnsupportedFormat),
            "This format cannot be created."
        );
        assert!(
            create_source_issue_text(Locale::ZhCn, CreateSourceIssue::MissingSource)
                .contains("来源已不存在")
        );
    }

    #[test]
    fn changing_create_format_updates_status_and_format_state() {
        let mut state = UiState {
            locale: Locale::En,
            compression_level: 22,
            create_password: "secret".to_owned(),
            ..UiState::default()
        };

        set_create_format(&mut state, ArchiveFormat::Bzip2);

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
    fn accessible_create_form_uses_single_file_picker_and_error_semantics() {
        let source = include_str!("accessible_main.rs");
        assert!(source.contains("let single_file_format ="));
        assert!(source.contains(&["dialog", "\n                .pick_file()"].concat()));
        assert!(source.contains(&["dialog.", "pick_files().await"].concat()));
        assert!(source.contains("CreateInputKind::SingleFile"));
        assert!(source.contains(&["state.write().", "set_error(message)"].concat()));
    }

    #[test]
    fn about_page_exposes_semantic_release_identity() {
        let source = include_str!("accessible_main.rs");
        assert!(source.contains("Page::About => rsx! { AboutPage { state } }"));
        assert!(source.contains("dl { class: \"about-details\""));
        assert!(source.contains("dt { {locale.text(Text::Version)} }"));
        assert!(source.contains("dd { {env!(\"CARGO_PKG_VERSION\")} }"));
        assert!(source.contains("https://github.com/ax2/zifile"));
        assert!(source.contains("\"aria-keyshortcuts\": ARIA_SHORTCUT_ABOUT"));
        assert!(STYLES.contains(".about-details"));
        assert!(source.contains("class: \"shortcut-help\""));
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
            assert!(source.contains(&format!(
                "dt {{ kbd {{ \"{keys}\" }} }} dd {{ {{locale.text({text_key})}} }}"
            )));
        }
        assert!(STYLES.contains(".shortcut-help kbd"));
    }

    #[test]
    fn accepted_create_submission_releases_the_form_password() {
        let mut state = UiState {
            create_password: "not-for-retention".to_owned(),
            create_password_visible: true,
            ..UiState::default()
        };
        clear_submitted_create_password(&mut state, OperationKind::Create, true);
        assert!(state.create_password.is_empty());
        assert!(!state.create_password_visible);

        state.create_password.push_str("retry-secret");
        state.create_password_visible = true;
        clear_submitted_create_password(&mut state, OperationKind::Create, false);
        assert_eq!(state.create_password, "retry-secret");
        assert!(state.create_password_visible);

        clear_submitted_create_password(&mut state, OperationKind::Extract, true);
        assert_eq!(state.create_password, "retry-secret");
        assert!(state.create_password_visible);
    }

    #[test]
    fn password_visibility_controls_are_labeled_and_scoped() {
        let source = include_str!("accessible_main.rs");
        for id in [
            "pending-archive-password",
            "archive-password",
            "create-password",
        ] {
            assert!(source.contains(&format!("id: \"{id}\"")));
            assert!(source.contains(&format!("\"aria-controls\": \"{id}\"")));
        }
        assert!(source.contains("locale.text(Text::ShowPassword)"));
        assert!(source.contains("if view.password_visible { \"text\" } else { \"password\" }"));
        assert!(
            source.contains("if view.create_password_visible { \"text\" } else { \"password\" }")
        );
        assert!(STYLES.contains(".password-toggle"));
    }

    #[test]
    fn empty_password_input_restores_masking() {
        let mut state = UiState {
            password: "secret".to_owned(),
            password_visible: true,
            create_password: "secret".to_owned(),
            create_password_visible: true,
            ..UiState::default()
        };

        state.set_archive_password(String::new());
        assert!(!state.password_visible);

        state.set_create_password(String::new());
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
        let list = WorkerRequest::List {
            archive: destination.clone(),
            password: None,
        };

        assert_eq!(operation_output_path(&create), Some(destination));
        assert_eq!(operation_output_path(&list), None);
        let source = include_str!("accessible_main.rs");
        assert!(source.contains("onclick: move |_| reveal_completed_output(state)"));
        assert!(source.contains("locale.text(Text::RevealOutput)"));
    }

    #[test]
    fn focus_indicators_are_two_tone_and_theme_aware() {
        assert!(STYLES.contains("outline: 3px solid var(--focus-ring);"));
        assert!(STYLES.contains("box-shadow: 0 0 0 2px var(--focus-ring-contrast);"));
        assert!(STYLES.contains("--focus-ring: #ffffff;"));
        assert!(STYLES.contains("--focus-ring-contrast: #000000;"));
        assert!(STYLES.contains("--focus-ring: #06324a;"));
        assert!(STYLES.contains("--focus-ring-contrast: #ffffff;"));
        assert!(STYLES.contains("--focus-ring: Highlight;"));
        assert!(STYLES.contains("--focus-ring-contrast: Canvas;"));
    }

    #[test]
    fn page_changes_focus_a_labelled_main_region() {
        assert_eq!(main_title_id(Page::Home, false), "home-title");
        assert_eq!(main_title_id(Page::Archive, false), "pending-archive-title");
        assert_eq!(main_title_id(Page::Archive, true), "archive-title");
        assert_eq!(main_title_id(Page::Create, false), "create-title");
        assert_eq!(main_title_id(Page::About, false), "about-title");

        let source = include_str!("accessible_main.rs");
        assert!(source.contains("if focused_page() != page"));
        assert!(source.contains("dioxus_document::eval(FOCUS_MAIN_SCRIPT)"));
        assert!(
            source.contains(
                "id: \"main-content\", tabindex: \"-1\", \"aria-labelledby\": main_title"
            )
        );
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
    fn archive_filter_copy_is_visible_bilingual_and_not_live() {
        assert_eq!(ARCHIVE_FILTER_LIVE, "off");
        assert_eq!(
            archive_filter_summary(Locale::En, "", 1, 1),
            "Showing 1 entry in this folder"
        );
        assert_eq!(
            archive_filter_summary(Locale::En, "beta", 1, 2),
            "Showing 1 of 2 entries for “beta”"
        );
        assert_eq!(
            archive_filter_summary(Locale::ZhCn, "文档", 0, 3),
            "3 个项目中显示 0 个匹配“文档”的结果"
        );
        assert_eq!(
            archive_filter_summary(Locale::ZhCn, "", 2, 30),
            "此文件夹显示 2 个项目"
        );
        assert_eq!(
            archive_no_matches(Locale::En, "missing"),
            "No archive entries match “missing”"
        );
        assert_eq!(archive_no_matches(Locale::ZhCn, ""), "此文件夹没有项目");
    }

    #[test]
    fn checksum_copy_action_has_clipboard_fallback_and_accessible_name() {
        let source = include_str!("accessible_main.rs");
        assert!(source.contains("class: \"copy-checksum\""));
        assert!(source.contains("navigator.clipboard ?"));
        assert!(source.contains("Text::CopyChecksum"));
    }

    #[test]
    fn queue_handoff_keeps_busy_until_the_next_worker_starts() {
        let source = include_str!("accessible_main.rs");
        let finish_source = source
            .split("fn finish_worker")
            .nth(1)
            .expect("finish_worker implementation should exist");
        let next_start = finish_source
            .find("Ok(Some(job)) => start_worker(state, job)")
            .expect("queued work should start directly");
        let idle_transition = finish_source
            .find("value.busy = false;")
            .expect("idle transition should still clear busy state");
        assert!(next_start < idle_transition);
    }

    #[test]
    fn archive_header_exposes_file_explorer_reveal_action() {
        let source = include_str!("accessible_main.rs");
        assert!(source.contains("onclick: move |_| reveal_archive(state)"));
        assert!(source.contains("reveal_in_file_manager(&path)"));
        assert!(source.contains("Text::RevealInExplorer"));
    }

    #[test]
    fn archive_actions_expose_selected_and_all_extraction_scopes() {
        let source = include_str!("accessible_main.rs");
        assert!(source.contains("onclick: move |_| extract_selected(state)"));
        assert!(source.contains("onclick: move |_| extract_to_named_folder(state)"));
        assert!(source.contains("onclick: move |_| extract_all(state)"));
        assert!(source.contains("startup::extraction_destination(&archive.path)"));
        assert!(source.contains("fn extract_with_scope"));
        assert!(source.contains("Text::ExtractSelected"));
        assert!(source.contains("Text::ExtractToNamedFolder"));
        assert!(source.contains("Text::ExtractAll"));
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
        assert_eq!(
            operation_progress_text(
                Locale::En,
                ProgressSnapshot {
                    processed_entries: 17,
                    ..ProgressSnapshot::default()
                }
            ),
            "Scanning · 17 entries found"
        );
        assert_eq!(
            operation_progress_text(
                Locale::ZhCn,
                ProgressSnapshot {
                    processed_bytes: 2048,
                    total_entries: 1,
                    ..ProgressSnapshot::default()
                }
            ),
            "正在扫描 · 已读取 2.0 KB"
        );
    }

    #[test]
    fn folder_navigation_has_semantic_breadcrumbs_and_native_buttons() {
        let source = include_str!("accessible_main.rs");
        assert!(source.contains("aria-label\": choose(locale, \"Archive folder\""));
        assert!(source.contains("\"aria-current\""));
        assert!(source.contains("choose(locale, \"Open folder\", \"打开文件夹\")"));
        assert!(source.contains("navigate_archive_directory(state, PathBuf::new())"));
        assert!(source.contains("class: \"folder-link\""));
        assert!(STYLES.contains(".breadcrumbs"));
        assert!(STYLES.contains(".folder-link"));
    }

    #[test]
    fn folder_selection_exposes_mixed_state_and_bilingual_counts() {
        let partial = DirectorySelection {
            selected: 2,
            total: 5,
        };
        assert_eq!(
            directory_selection_label(Locale::En, "docs", partial),
            "Select folder docs; 2 of 5 files selected"
        );
        assert_eq!(
            directory_selection_label(
                Locale::ZhCn,
                "文档",
                DirectorySelection {
                    selected: 5,
                    total: 5
                }
            ),
            "清除文件夹 文档；已选择 5/5 个文件"
        );
        assert_eq!(
            folder_selection_summary(Locale::En, partial),
            "2/5 selected"
        );
        let source = include_str!("accessible_main.rs");
        assert!(source.contains("\"aria-checked\""));
        assert!(source.contains("\"mixed\""));
        assert!(source.contains("{current_page}-{index}-{entry.path.display()}"));
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
            accessible_shortcut("o", Modifiers::CONTROL, false, false, false),
            Some(AccessibleShortcut::Open)
        );
        assert_eq!(
            accessible_shortcut("N", Modifiers::CONTROL, false, false, false),
            Some(AccessibleShortcut::Create)
        );
        assert_eq!(
            accessible_shortcut("r", Modifiers::CONTROL, false, false, false),
            Some(AccessibleShortcut::Reload)
        );
        assert_eq!(
            accessible_shortcut(
                "r",
                Modifiers::CONTROL | Modifiers::SHIFT,
                false,
                false,
                false,
            ),
            None
        );
        assert_eq!(
            accessible_shortcut("f", Modifiers::CONTROL, false, true, true),
            Some(AccessibleShortcut::Search)
        );
        assert_eq!(
            accessible_shortcut("f", Modifiers::CONTROL, false, false, false),
            None
        );
        assert_eq!(
            accessible_shortcut(
                "f",
                Modifiers::CONTROL | Modifiers::SHIFT,
                false,
                true,
                true,
            ),
            None
        );
        assert_eq!(
            accessible_shortcut("w", Modifiers::CONTROL, false, true, true),
            Some(AccessibleShortcut::Close)
        );
        assert_eq!(
            accessible_shortcut("w", Modifiers::CONTROL, false, true, false),
            None
        );
        assert_eq!(
            accessible_shortcut("w", Modifiers::CONTROL, false, false, true),
            None
        );
        assert_eq!(
            accessible_shortcut("i", Modifiers::CONTROL, false, true, false),
            Some(AccessibleShortcut::InvertSelection)
        );
        assert_eq!(
            accessible_shortcut("i", Modifiers::CONTROL, false, false, false),
            None
        );
        assert_eq!(
            accessible_shortcut(
                "i",
                Modifiers::CONTROL | Modifiers::SHIFT,
                true,
                false,
                false,
            ),
            None
        );
        assert_eq!(
            accessible_shortcut("Escape", Modifiers::empty(), true, false, false),
            Some(AccessibleShortcut::Cancel)
        );
        assert_eq!(
            accessible_shortcut("F1", Modifiers::empty(), false, false, false),
            Some(AccessibleShortcut::About)
        );
        assert_eq!(
            accessible_shortcut("Escape", Modifiers::empty(), false, false, false),
            None
        );
        assert_eq!(
            accessible_shortcut(
                "N",
                Modifiers::CONTROL | Modifiers::SHIFT,
                false,
                false,
                false,
            ),
            None
        );
        assert_eq!(
            accessible_shortcut("F1", Modifiers::ALT, false, false, false),
            None
        );
        assert_eq!(
            accessible_shortcut("a", Modifiers::CONTROL, true, false, false),
            None
        );
        assert_eq!(
            accessible_shortcut("o", Modifiers::empty(), true, false, false),
            None
        );
        assert!(is_select_all_shortcut("a", Modifiers::CONTROL));
        assert!(is_select_all_shortcut(
            "A",
            Modifiers::CONTROL | Modifiers::CAPS_LOCK
        ));
        assert!(!is_select_all_shortcut("a", Modifiers::empty()));
        assert!(!is_select_all_shortcut(
            "a",
            Modifiers::CONTROL | Modifiers::SHIFT
        ));
        assert!(!is_select_all_shortcut("o", Modifiers::CONTROL));
    }

    #[test]
    fn unlock_enter_requires_an_enabled_idle_non_composing_input() {
        assert!(unlock_submit_key("Enter", Modifiers::empty(), false, true));
        assert!(unlock_submit_key(
            "enter",
            Modifiers::CAPS_LOCK,
            false,
            true
        ));
        assert!(!unlock_submit_key("Enter", Modifiers::CONTROL, false, true));
        assert!(!unlock_submit_key("Enter", Modifiers::empty(), true, true));
        assert!(!unlock_submit_key(
            "Enter",
            Modifiers::empty(),
            false,
            false
        ));
        assert!(!unlock_submit_key("Space", Modifiers::empty(), false, true));
    }

    #[test]
    fn handled_shortcuts_are_exposed_to_assistive_technology() {
        assert_eq!(ARIA_SHORTCUT_OPEN, "Control+O");
        assert_eq!(ARIA_SHORTCUT_CREATE, "Control+N");
        assert_eq!(ARIA_SHORTCUT_RELOAD, "Control+R");
        assert_eq!(ARIA_SHORTCUT_SEARCH, "Control+F");
        assert_eq!(ARIA_SHORTCUT_CLOSE, "Control+W");
        assert_eq!(ARIA_SHORTCUT_ABOUT, "F1");
        assert_eq!(ARIA_SHORTCUT_CANCEL, "Escape");
        assert_eq!(ARIA_SHORTCUT_SELECT_ALL, "Control+A");
        assert_eq!(ARIA_SHORTCUT_INVERT_SELECTION, "Control+I");

        let source = include_str!("accessible_main.rs");
        for shortcut in [
            "ARIA_SHORTCUT_OPEN",
            "ARIA_SHORTCUT_CREATE",
            "ARIA_SHORTCUT_RELOAD",
            "ARIA_SHORTCUT_SEARCH",
            "ARIA_SHORTCUT_CLOSE",
            "ARIA_SHORTCUT_ABOUT",
            "ARIA_SHORTCUT_CANCEL",
            "ARIA_SHORTCUT_SELECT_ALL",
            "ARIA_SHORTCUT_INVERT_SELECTION",
        ] {
            assert!(
                source.contains(&format!("\"aria-keyshortcuts\": {shortcut}")),
                "missing semantic shortcut metadata for {shortcut}"
            );
        }
        assert!(source.contains("id: \"archive-search\""));
        assert!(source.contains("dioxus_document::eval(FOCUS_ARCHIVE_SEARCH_SCRIPT)"));
    }

    #[test]
    fn sortable_headers_expose_accessible_direction() {
        assert_eq!(
            sort_aria(
                EntrySort::Modified,
                EntrySort::Modified,
                SortDirection::Ascending,
            ),
            "ascending"
        );
        assert_eq!(
            sort_aria(
                EntrySort::Packed,
                EntrySort::Modified,
                SortDirection::Descending,
            ),
            "none"
        );
        assert_eq!(
            sort_header_label(
                "修改时间",
                EntrySort::Modified,
                EntrySort::Modified,
                SortDirection::Descending,
            ),
            "修改时间 ↓"
        );
    }
}
