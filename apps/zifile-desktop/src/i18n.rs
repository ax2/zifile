use zifile_core::{ArchiveTimestamp, ArchiveTimestampOffset, ArchiveTimestampPrecision};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Locale {
    En,
    ZhCn,
}

impl Locale {
    pub fn detect() -> Self {
        if sys_locale::get_locale()
            .is_some_and(|value| value.to_ascii_lowercase().starts_with("zh"))
        {
            Self::ZhCn
        } else {
            Self::En
        }
    }

    pub const fn code(self) -> &'static str {
        match self {
            Self::En => "en-US",
            Self::ZhCn => "zh-CN",
        }
    }

    pub const fn toggle(self) -> Self {
        match self {
            Self::En => Self::ZhCn,
            Self::ZhCn => Self::En,
        }
    }

    pub const fn text(self, key: Text) -> &'static str {
        let (english, chinese) = match key {
            Text::ArchiveStudio => ("Archive Studio", "压缩文件工作室"),
            Text::Home => ("Home", "首页"),
            Text::Archive => ("Archive", "压缩文件"),
            Text::Create => ("Create", "创建"),
            Text::About => ("About", "关于"),
            Text::Light => ("Light", "浅色"),
            Text::Dark => ("Dark", "深色"),
            Text::PreferencesSaveFailed => ("Could not save preferences", "无法保存偏好设置"),
            Text::SwitchLanguage => ("中文", "English"),
            Text::Ready => ("Ready", "就绪"),
            Text::Cancel => ("Cancel", "取消"),
            Text::Hero => ("Files, packed beautifully.", "让文件整理更轻松。"),
            Text::HeroSub => (
                "A fast, local-first archive manager built in Rust for Windows.",
                "以 Rust 构建，快速、本地优先的 Windows 压缩文件管理器。",
            ),
            Text::OpenArchive => ("Open an archive", "打开压缩文件"),
            Text::OpenDescription => (
                "Browse, verify and safely extract ZIP, 7z and TAR-family archives.",
                "浏览、校验并安全解压 ZIP、7z 和 TAR 系列文件。",
            ),
            Text::OpenAction => ("Open archive", "打开文件"),
            Text::CreateArchive => ("Create an archive", "创建压缩文件"),
            Text::CreateDescription => (
                "Package files and folders with compression level and optional encryption.",
                "选择压缩等级和可选加密，打包文件与文件夹。",
            ),
            Text::StartCreating => ("Start creating", "开始创建"),
            Text::Privacy => ("Privacy by default", "默认保护隐私"),
            Text::PrivacyDescription => (
                "Archive work stays on this device. ZiFile does not upload filenames, passwords or file contents.",
                "所有操作都在本机完成。ZiFile 不上传文件名、密码或文件内容。",
            ),
            Text::AboutHeading => ("About ZiFile", "关于 ZiFile"),
            Text::AboutDescription => (
                "A local-first, open-source archive manager built in Rust for Windows.",
                "以 Rust 构建、面向 Windows 的本地优先开源压缩文件管理器。",
            ),
            Text::Version => ("Version", "版本"),
            Text::License => ("License", "许可证"),
            Text::SupportedFormatFamilies => ("Supported format families", "支持的格式系列"),
            Text::ProjectWebsite => ("Project website", "项目网站"),
            Text::KeyboardShortcuts => ("Keyboard shortcuts", "键盘快捷键"),
            Text::ShortcutOpen => ("Open an archive", "打开压缩文件"),
            Text::ShortcutCreate => ("Create an archive", "创建压缩文件"),
            Text::ShortcutReload => ("Reload the current archive", "重新加载当前压缩文件"),
            Text::ShortcutSearch => ("Focus archive search", "聚焦压缩文件搜索"),
            Text::ShortcutClose => ("Close the current archive", "关闭当前压缩文件"),
            Text::ShortcutSelectAll => ("Select all archive entries", "选择全部压缩项目"),
            Text::ShortcutInvertSelection => ("Invert archive selection", "反选压缩项目"),
            Text::ShortcutAbout => ("Open About and shortcut help", "打开关于与快捷键帮助"),
            Text::ShortcutCancel => ("Cancel the current operation", "取消当前操作"),
            Text::NoArchive => ("No archive open", "尚未打开压缩文件"),
            Text::SelectAll => ("Select all", "全选"),
            Text::SelectNone => ("Select none", "全不选"),
            Text::InvertSelection => ("Invert selection", "反选"),
            Text::NoArchiveDescription => (
                "Open an archive to inspect its contents and extract files.",
                "打开压缩文件以查看内容并解压所需文件。",
            ),
            Text::OpeningArchiveDescription => ("Opening this archive…", "正在打开此压缩文件…"),
            Text::BusyArchiveDescription => (
                "Archive work is running; the entry browser will refresh when it finishes.",
                "压缩文件操作正在运行；完成后将刷新项目列表。",
            ),
            Text::ArchiveOpenFailedDescription => (
                "ZiFile could not open this archive. Choose another file or try again.",
                "ZiFile 无法打开此压缩文件。请选择其他文件或重试。",
            ),
            Text::EncryptedArchiveDescription => (
                "This archive may encrypt its file list. Enter the password and try again.",
                "此压缩文件可能加密了文件列表。请输入密码后重试。",
            ),
            Text::UnlockArchive => ("Unlock archive", "解锁压缩文件"),
            Text::OpenAnother => ("Open another", "打开其他文件"),
            Text::CloseArchive => ("Close archive", "关闭压缩文件"),
            Text::ArchiveClosed => ("Archive closed", "已关闭压缩文件"),
            Text::RevealInExplorer => ("Show in File Explorer", "在资源管理器中显示"),
            Text::RevealedInExplorer => ("Opened the containing folder", "已打开所在文件夹"),
            Text::RevealInExplorerFailed => ("Could not open File Explorer", "无法打开资源管理器"),
            Text::RevealOutput => ("Show output", "查看输出"),
            Text::OutputRevealed => (
                "Opened the output in File Explorer",
                "已在资源管理器中打开输出",
            ),
            Text::TestArchive => ("Test archive", "校验压缩文件"),
            Text::Selected => ("selected", "项已选择"),
            Text::PasswordEncrypted => ("Password (if encrypted)", "密码（如已加密）"),
            Text::ShowPassword => ("Show password", "显示密码"),
            Text::Reload => ("Reload", "重新加载"),
            Text::ExtractSelected => ("Extract selected", "解压所选项目"),
            Text::ExtractAll => ("Extract all", "解压全部"),
            Text::ExtractToNamedFolder => ("Extract to named folder", "解压到同名文件夹"),
            Text::Name => ("Name", "名称"),
            Text::Original => ("Original", "原始大小"),
            Text::Packed => ("Packed", "压缩大小"),
            Text::Modified => ("Modified", "修改时间"),
            Text::Flags => ("Flags", "标记"),
            Text::Checksum => ("SHA-256", "SHA-256 校验和"),
            Text::CopyChecksum => ("Copy checksum", "复制校验和"),
            Text::ChecksumCopied => ("Checksum copied", "校验和已复制"),
            Text::Locked => ("Locked", "已加密"),
            Text::Search => ("Search paths", "搜索路径"),
            Text::ClearSearch => ("Clear search", "清除搜索"),
            Text::Previous => ("Previous", "上一页"),
            Text::Next => ("Next", "下一页"),
            Text::Page => ("Page", "页码"),
            Text::CreateHeading => ("Create archive", "创建压缩文件"),
            Text::CreateHelp => (
                "Choose sources, format and compression. The archive is written through a temporary file before replacing its destination.",
                "选择来源、格式与压缩等级。ZiFile 先写入临时文件，再安全替换目标。",
            ),
            Text::NoSources => (
                "No sources yet. Add files or one or more folders.",
                "尚未添加来源。请添加文件或一个或多个文件夹。",
            ),
            Text::MissingSource => (
                "One or more sources no longer exist. Remove them or choose them again.",
                "一个或多个来源已不存在。请移除后重新选择。",
            ),
            Text::LinkSource => (
                "Symbolic-link, junction, and reparse-point sources are not archived. Choose the original file or folder.",
                "不支持将符号链接、junction 或 reparse point 作为压缩来源。请选择原始文件或文件夹。",
            ),
            Text::FilesAndFoldersSupported => (
                "This format accepts files and folders.",
                "此格式支持添加文件和文件夹。",
            ),
            Text::SingleFileRequired => (
                "This stream format requires exactly one file. Use a TAR composition for folders or multiple items.",
                "此流格式只能压缩一个文件。文件夹或多个项目请改用 TAR 组合格式。",
            ),
            Text::FormatCannotCreate => ("This format cannot be created.", "此格式不支持创建。"),
            Text::AddFiles => ("Add files", "添加文件"),
            Text::AddFolder => ("Add folder", "添加文件夹"),
            Text::Clear => ("Clear", "清空"),
            Text::Remove => ("Remove", "移除"),
            Text::Format => ("Format", "格式"),
            Text::CompressionLevel => ("Compression level", "压缩等级"),
            Text::CompressionFixed => (
                "Compression level · fixed for this format",
                "压缩等级 · 此格式使用固定设置",
            ),
            Text::PasswordOptional => ("Password · optional", "密码 · 可选"),
            Text::PasswordUnavailable => (
                "Password · unavailable for this format",
                "密码 · 此格式不支持加密",
            ),
            Text::NoEncryption => ("Leave empty for no encryption", "留空表示不加密"),
            Text::CreateAction => ("Create archive", "创建压缩文件"),
            Text::ChooseExtractionFolder => ("Choose extraction folder", "选择解压目录"),
            Text::AddFilesDialog => ("Add files to archive", "添加要压缩的文件"),
            Text::AddFolderDialog => ("Add folder to archive", "添加要压缩的文件夹"),
            Text::CreateDialog => ("Create archive", "创建压缩文件"),
            Text::OpenDialog => ("Open archive", "打开压缩文件"),
            Text::SupportedArchives => ("Supported archives", "支持的压缩文件"),
            Text::AllFiles => ("All files", "所有文件"),
            Text::ConflictOverwrite => ("Overwrite existing", "覆盖现有文件"),
            Text::ConflictSkip => ("Skip existing", "跳过现有文件"),
            Text::ConflictRename => ("Keep both (rename)", "保留两者（自动重命名）"),
            Text::ConflictError => ("Stop on conflict", "遇到冲突时停止"),
        };
        match self {
            Self::En => english,
            Self::ZhCn => chinese,
        }
    }
}

pub fn format_archive_modified(locale: Locale, value: Option<&ArchiveTimestamp>) -> String {
    let Some(value) = value else {
        return "—".to_owned();
    };
    let mut formatted = format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
        value.year, value.month, value.day, value.hour, value.minute, value.second
    );
    if value.precision == ArchiveTimestampPrecision::Subsecond && value.nanosecond != 0 {
        let fraction = format!("{:09}", value.nanosecond);
        formatted.push('.');
        formatted.push_str(fraction.trim_end_matches('0'));
    }
    match value.offset {
        ArchiveTimestampOffset::Utc => formatted.push_str(" UTC"),
        ArchiveTimestampOffset::Unspecified => formatted.push_str(if locale == Locale::ZhCn {
            " · 无时区"
        } else {
            " · no TZ"
        }),
    }
    formatted
}

/// Maps stable Worker diagnostics to user-facing localized text while keeping
/// backend-specific error details intact for all other failures.
pub fn format_worker_error(locale: Locale, error: &str) -> String {
    if error.eq_ignore_ascii_case("operation cancelled") {
        return if locale == Locale::ZhCn {
            "操作已取消".to_owned()
        } else {
            "Operation cancelled".to_owned()
        };
    }
    if error.eq_ignore_ascii_case("a password is required to open this archive") {
        return if locale == Locale::ZhCn {
            "需要密码。请输入压缩文件密码后重试。".to_owned()
        } else {
            "A password is required. Enter the archive password and try again.".to_owned()
        };
    }
    if error.eq_ignore_ascii_case("the archive format could not be identified") {
        return if locale == Locale::ZhCn {
            "无法识别压缩文件格式。".to_owned()
        } else {
            "The archive format could not be identified.".to_owned()
        };
    }
    if let Some(details) = error.strip_prefix("destination already exists: ") {
        return if locale == Locale::ZhCn {
            format!("目标已存在：{details}")
        } else {
            format!("Destination already exists: {details}")
        };
    }
    if let Some(details) =
        error.strip_prefix("the extraction destination contains a symbolic link or reparse point: ")
    {
        return if locale == Locale::ZhCn {
            format!("解压目标包含符号链接或 reparse point：{details}")
        } else {
            format!(
                "The extraction destination contains a symbolic link or reparse point: {details}"
            )
        };
    }
    if let Some(details) = error.strip_prefix("a configured safety limit was exceeded: ") {
        return if locale == Locale::ZhCn {
            format!("已超过安全限制：{details}")
        } else {
            format!("A configured safety limit was exceeded: {details}")
        };
    }
    if let Some(details) = error.strip_prefix("the requested operation is not supported for ") {
        return if locale == Locale::ZhCn {
            format!("此格式不支持所选操作：{details}")
        } else {
            format!("The requested operation is not supported for {details}")
        };
    }
    error.to_owned()
}

pub fn worker_error_may_require_password(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("password") || lower.contains("encrypted")
}

pub fn archive_no_matches(locale: Locale, filter: &str) -> String {
    match locale {
        Locale::En if filter.is_empty() => "This folder has no entries".to_owned(),
        Locale::En => format!("No archive entries match “{filter}”"),
        Locale::ZhCn if filter.is_empty() => "此文件夹没有项目".to_owned(),
        Locale::ZhCn => format!("没有压缩文件项目匹配“{filter}”"),
    }
}

pub fn archive_filter_summary(
    locale: Locale,
    filter: &str,
    matches: usize,
    total: usize,
) -> String {
    match locale {
        Locale::En if filter.is_empty() => format!(
            "Showing {matches} {} in this folder",
            if matches == 1 { "entry" } else { "entries" }
        ),
        Locale::En => format!(
            "Showing {matches} of {total} {} for “{filter}”",
            if total == 1 { "entry" } else { "entries" }
        ),
        Locale::ZhCn if filter.is_empty() => format!("此文件夹显示 {matches} 个项目"),
        Locale::ZhCn => format!("{total} 个项目中显示 {matches} 个匹配“{filter}”的结果"),
    }
}

pub fn create_sources_added_status(locale: Locale, added: usize, total: usize) -> String {
    match locale {
        Locale::En => format!(
            "Added {added} archive {}; {total} total",
            if added == 1 { "source" } else { "sources" }
        ),
        Locale::ZhCn => format!("已添加 {added} 个压缩来源；共 {total} 个"),
    }
}

pub fn create_source_removed_status(locale: Locale, path: &str, remaining: usize) -> String {
    match locale {
        Locale::En => format!("Removed archive source {path}; {remaining} remaining"),
        Locale::ZhCn => format!("已移除压缩来源 {path}；剩余 {remaining} 个"),
    }
}

pub fn create_sources_cleared_status(locale: Locale, cleared: usize) -> String {
    match locale {
        Locale::En => format!("Cleared {cleared} archive sources"),
        Locale::ZhCn => format!("已清除 {cleared} 个压缩来源"),
    }
}

pub fn create_source_summary(locale: Locale, count: usize) -> String {
    match locale {
        Locale::En => format!(
            "{count} archive {} added",
            if count == 1 { "source" } else { "sources" }
        ),
        Locale::ZhCn => format!("已添加 {count} 个压缩来源"),
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Text {
    ArchiveStudio,
    Home,
    Archive,
    Create,
    About,
    Light,
    Dark,
    PreferencesSaveFailed,
    SwitchLanguage,
    Ready,
    Cancel,
    Hero,
    HeroSub,
    OpenArchive,
    OpenDescription,
    OpenAction,
    CreateArchive,
    CreateDescription,
    StartCreating,
    Privacy,
    PrivacyDescription,
    AboutHeading,
    AboutDescription,
    Version,
    License,
    SupportedFormatFamilies,
    ProjectWebsite,
    KeyboardShortcuts,
    ShortcutOpen,
    ShortcutCreate,
    ShortcutReload,
    ShortcutSearch,
    ShortcutClose,
    ShortcutSelectAll,
    ShortcutInvertSelection,
    ShortcutAbout,
    ShortcutCancel,
    NoArchive,
    SelectAll,
    SelectNone,
    InvertSelection,
    NoArchiveDescription,
    OpeningArchiveDescription,
    BusyArchiveDescription,
    ArchiveOpenFailedDescription,
    EncryptedArchiveDescription,
    UnlockArchive,
    OpenAnother,
    CloseArchive,
    ArchiveClosed,
    RevealInExplorer,
    RevealedInExplorer,
    RevealInExplorerFailed,
    RevealOutput,
    OutputRevealed,
    TestArchive,
    Selected,
    PasswordEncrypted,
    ShowPassword,
    Reload,
    ExtractSelected,
    ExtractAll,
    ExtractToNamedFolder,
    Name,
    Original,
    Packed,
    Modified,
    Flags,
    Checksum,
    CopyChecksum,
    ChecksumCopied,
    Locked,
    Search,
    ClearSearch,
    Previous,
    Next,
    Page,
    CreateHeading,
    CreateHelp,
    NoSources,
    MissingSource,
    LinkSource,
    FilesAndFoldersSupported,
    SingleFileRequired,
    FormatCannotCreate,
    AddFiles,
    AddFolder,
    Clear,
    Remove,
    Format,
    CompressionLevel,
    CompressionFixed,
    PasswordOptional,
    PasswordUnavailable,
    NoEncryption,
    CreateAction,
    ChooseExtractionFolder,
    AddFilesDialog,
    AddFolderDialog,
    CreateDialog,
    OpenDialog,
    SupportedArchives,
    AllFiles,
    ConflictOverwrite,
    ConflictSkip,
    ConflictRename,
    ConflictError,
}

pub const fn archive_empty_state_description(
    locale: Locale,
    busy: bool,
    pending: bool,
    requires_password: bool,
) -> &'static str {
    let text = if requires_password {
        Text::EncryptedArchiveDescription
    } else if busy && pending {
        Text::OpeningArchiveDescription
    } else if pending {
        Text::ArchiveOpenFailedDescription
    } else {
        Text::NoArchiveDescription
    };
    locale.text(text)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_locales_cover_representative_navigation_and_security_text() {
        assert_eq!(Locale::En.text(Text::Home), "Home");
        assert_eq!(Locale::ZhCn.text(Text::Home), "首页");
        assert_eq!(Locale::En.text(Text::ClearSearch), "Clear search");
        assert_eq!(Locale::ZhCn.text(Text::ClearSearch), "清除搜索");
        assert_eq!(
            Locale::En.text(Text::OpeningArchiveDescription),
            "Opening this archive…"
        );
        assert_eq!(
            Locale::En.text(Text::ArchiveOpenFailedDescription),
            "ZiFile could not open this archive. Choose another file or try again."
        );
        assert!(worker_error_may_require_password("invalid password"));
        assert!(!worker_error_may_require_password(
            "archive header is corrupt"
        ));
        assert_eq!(
            archive_no_matches(Locale::En, "missing"),
            "No archive entries match “missing”"
        );
        assert_eq!(archive_no_matches(Locale::ZhCn, ""), "此文件夹没有项目");
        assert_eq!(
            archive_filter_summary(Locale::En, "missing", 0, 3),
            "Showing 0 of 3 entries for “missing”"
        );
        assert_eq!(
            archive_filter_summary(Locale::ZhCn, "", 2, 3),
            "此文件夹显示 2 个项目"
        );
        assert_eq!(
            create_sources_added_status(Locale::En, 1, 2),
            "Added 1 archive source; 2 total"
        );
        assert_eq!(
            create_source_removed_status(Locale::ZhCn, "C:\\资料\\a.txt", 1),
            "已移除压缩来源 C:\\资料\\a.txt；剩余 1 个"
        );
        assert_eq!(
            create_sources_cleared_status(Locale::En, 3),
            "Cleared 3 archive sources"
        );
        assert_eq!(
            create_source_summary(Locale::En, 1),
            "1 archive source added"
        );
        assert_eq!(Locale::En.text(Text::About), "About");
        assert_eq!(Locale::ZhCn.text(Text::AboutHeading), "关于 ZiFile");
        assert_eq!(Locale::ZhCn.text(Text::License), "许可证");
        assert_eq!(
            Locale::En.text(Text::KeyboardShortcuts),
            "Keyboard shortcuts"
        );
        assert_eq!(Locale::ZhCn.text(Text::ShortcutCancel), "取消当前操作");
        assert_eq!(Locale::En.text(Text::Checksum), "SHA-256");
        assert_eq!(Locale::ZhCn.text(Text::Checksum), "SHA-256 校验和");
        assert_eq!(Locale::En.text(Text::CopyChecksum), "Copy checksum");
        assert_eq!(Locale::ZhCn.text(Text::ChecksumCopied), "校验和已复制");
        assert_eq!(
            Locale::En.text(Text::RevealInExplorer),
            "Show in File Explorer"
        );
        assert_eq!(
            Locale::ZhCn.text(Text::RevealedInExplorer),
            "已打开所在文件夹"
        );
        assert_eq!(Locale::En.text(Text::RevealOutput), "Show output");
        assert_eq!(
            Locale::ZhCn.text(Text::OutputRevealed),
            "已在资源管理器中打开输出"
        );
        assert!(Locale::ZhCn.text(Text::PrivacyDescription).contains("本机"));
        assert_eq!(Locale::ZhCn.toggle(), Locale::En);
    }

    #[test]
    fn archive_empty_state_description_distinguishes_loading_and_failure() {
        assert_eq!(
            archive_empty_state_description(Locale::En, true, true, false),
            "Opening this archive…"
        );
        assert_eq!(
            archive_empty_state_description(Locale::ZhCn, false, true, true),
            "此压缩文件可能加密了文件列表。请输入密码后重试。"
        );
        assert_eq!(
            archive_empty_state_description(Locale::En, false, true, false),
            "ZiFile could not open this archive. Choose another file or try again."
        );
        assert_eq!(
            archive_empty_state_description(Locale::ZhCn, false, false, false),
            "打开压缩文件以查看内容并解压所需文件。"
        );
    }

    #[test]
    fn archive_modified_time_exposes_timezone_semantics() {
        let value = ArchiveTimestamp {
            year: 2023,
            month: 11,
            day: 14,
            hour: 22,
            minute: 13,
            second: 20,
            nanosecond: 0,
            offset: ArchiveTimestampOffset::Unspecified,
            precision: ArchiveTimestampPrecision::TwoSeconds,
        };
        assert_eq!(
            format_archive_modified(Locale::En, Some(&value)),
            "2023-11-14 22:13:20 · no TZ"
        );
        assert!(format_archive_modified(Locale::ZhCn, Some(&value)).ends_with("无时区"));
        assert_eq!(format_archive_modified(Locale::En, None), "—");
    }

    #[test]
    fn worker_cancellation_error_is_localized_without_rewriting_backend_errors() {
        assert_eq!(
            format_worker_error(Locale::ZhCn, "operation cancelled"),
            "操作已取消"
        );
        assert_eq!(
            format_worker_error(Locale::En, "Operation Cancelled"),
            "Operation cancelled"
        );
        assert_eq!(
            format_worker_error(Locale::ZhCn, "archive checksum mismatch"),
            "archive checksum mismatch"
        );
    }

    #[test]
    fn link_source_guidance_names_windows_link_like_sources() {
        let english = Locale::En.text(Text::LinkSource);
        let chinese = Locale::ZhCn.text(Text::LinkSource);
        assert!(english.contains("junction"));
        assert!(english.contains("reparse-point"));
        assert!(chinese.contains("junction"));
        assert!(chinese.contains("reparse point"));
    }

    #[test]
    fn stable_core_errors_are_localized_without_losing_details() {
        assert_eq!(
            format_worker_error(Locale::ZhCn, "a password is required to open this archive"),
            "需要密码。请输入压缩文件密码后重试。"
        );
        assert_eq!(
            format_worker_error(Locale::ZhCn, "destination already exists: C:\\资料\\out"),
            "目标已存在：C:\\资料\\out"
        );
        assert_eq!(
            format_worker_error(
                Locale::ZhCn,
                "the extraction destination contains a symbolic link or reparse point: C:\\资料\\out"
            ),
            "解压目标包含符号链接或 reparse point：C:\\资料\\out"
        );
        assert_eq!(
            format_worker_error(
                Locale::En,
                "a configured safety limit was exceeded: expanded data exceeds 512 bytes"
            ),
            "A configured safety limit was exceeded: expanded data exceeds 512 bytes"
        );
        assert_eq!(
            format_worker_error(Locale::ZhCn, "the archive format could not be identified"),
            "无法识别压缩文件格式。"
        );
    }
}
