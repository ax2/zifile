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
            Text::NoArchive => ("No archive open", "尚未打开压缩文件"),
            Text::NoArchiveDescription => (
                "Open an archive to inspect its contents and extract files.",
                "打开压缩文件以查看内容并解压所需文件。",
            ),
            Text::EncryptedArchiveDescription => (
                "This archive may encrypt its file list. Enter the password and try again.",
                "此压缩文件可能加密了文件列表。请输入密码后重试。",
            ),
            Text::UnlockArchive => ("Unlock archive", "解锁压缩文件"),
            Text::OpenAnother => ("Open another", "打开其他文件"),
            Text::TestArchive => ("Test archive", "校验压缩文件"),
            Text::Selected => ("selected", "项已选择"),
            Text::PasswordEncrypted => ("Password (if encrypted)", "密码（如已加密）"),
            Text::Reload => ("Reload", "重新加载"),
            Text::ExtractSelected => ("Extract selected", "解压所选项目"),
            Text::Name => ("Name", "名称"),
            Text::Original => ("Original", "原始大小"),
            Text::Packed => ("Packed", "压缩大小"),
            Text::Modified => ("Modified", "修改时间"),
            Text::Flags => ("Flags", "标记"),
            Text::Locked => ("Locked", "已加密"),
            Text::Search => ("Search paths", "搜索路径"),
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
    error.to_owned()
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
    NoArchive,
    NoArchiveDescription,
    EncryptedArchiveDescription,
    UnlockArchive,
    OpenAnother,
    TestArchive,
    Selected,
    PasswordEncrypted,
    Reload,
    ExtractSelected,
    Name,
    Original,
    Packed,
    Modified,
    Flags,
    Locked,
    Search,
    Previous,
    Next,
    Page,
    CreateHeading,
    CreateHelp,
    NoSources,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn both_locales_cover_representative_navigation_and_security_text() {
        assert_eq!(Locale::En.text(Text::Home), "Home");
        assert_eq!(Locale::ZhCn.text(Text::Home), "首页");
        assert_eq!(Locale::En.text(Text::About), "About");
        assert_eq!(Locale::ZhCn.text(Text::AboutHeading), "关于 ZiFile");
        assert_eq!(Locale::ZhCn.text(Text::License), "许可证");
        assert!(Locale::ZhCn.text(Text::PrivacyDescription).contains("本机"));
        assert_eq!(Locale::ZhCn.toggle(), Locale::En);
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
}
