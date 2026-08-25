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
            Text::NoArchive => ("No archive open", "尚未打开压缩文件"),
            Text::NoArchiveDescription => (
                "Open an archive to inspect its contents and extract files.",
                "打开压缩文件以查看内容并解压所需文件。",
            ),
            Text::OpenAnother => ("Open another", "打开其他文件"),
            Text::TestArchive => ("Test archive", "校验压缩文件"),
            Text::Selected => ("selected", "项已选择"),
            Text::PasswordEncrypted => ("Password (if encrypted)", "密码（如已加密）"),
            Text::Reload => ("Reload", "重新加载"),
            Text::ExtractSelected => ("Extract selected", "解压所选项目"),
            Text::Name => ("Name", "名称"),
            Text::Original => ("Original", "原始大小"),
            Text::Packed => ("Packed", "压缩大小"),
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

#[derive(Debug, Clone, Copy)]
pub enum Text {
    ArchiveStudio,
    Home,
    Archive,
    Create,
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
    NoArchive,
    NoArchiveDescription,
    OpenAnother,
    TestArchive,
    Selected,
    PasswordEncrypted,
    Reload,
    ExtractSelected,
    Name,
    Original,
    Packed,
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
        assert!(Locale::ZhCn.text(Text::PrivacyDescription).contains("本机"));
        assert_eq!(Locale::ZhCn.toggle(), Locale::En);
    }
}
