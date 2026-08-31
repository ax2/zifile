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
use std::process::Command;

use zifile_core::{ArchiveFormat, ArchiveInfo, detect_format, detect_format_from_path};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OfficialLink {
    Project,
    DocumentationZh,
    DocumentationEn,
    PrivacyZh,
    PrivacyEn,
}

impl OfficialLink {
    pub const fn url(self) -> &'static str {
        match self {
            Self::Project => "https://github.com/ax2/zifile",
            Self::DocumentationZh => "https://ax2.github.io/zifile/",
            Self::DocumentationEn => "https://ax2.github.io/zifile/en/",
            Self::PrivacyZh => "https://ax2.github.io/zifile/product/privacy/",
            Self::PrivacyEn => "https://ax2.github.io/zifile/en/product/privacy/",
        }
    }
}

pub mod create_validation;
pub mod entry_view;
pub mod operation_queue;
pub mod startup;

/// Returns whether a dropped path should enter the archive browser.
///
/// Content signatures are authoritative whenever available. The extension is
/// only a compatibility fallback for formats without a universal signature or
/// for a file that cannot be probed during the drop event; the Worker performs
/// the definitive list operation afterwards. This performs filesystem IO and
/// must be called from a background task, not a UI event handler.
pub fn is_openable_archive_path(path: &Path) -> bool {
    if !path.is_file() {
        return false;
    }
    detect_format(path)
        .map(|format| format.capabilities().list)
        .unwrap_or_else(|_| {
            detect_format_from_path(path).is_some_and(|format| format.capabilities().list)
        })
}

/// Appends sources while treating equivalent Windows spellings as one path.
///
/// File pickers, Explorer and drag-and-drop can use different casing or slash
/// separators for the same filesystem path. Keeping this identity rule in the
/// shared desktop crate prevents duplicate archive roots and name collisions.
pub fn append_unique_paths(destination: &mut Vec<PathBuf>, paths: Vec<PathBuf>) -> usize {
    let before = destination.len();
    for path in paths {
        if !destination
            .iter()
            .any(|existing| paths_have_same_identity(existing, &path))
        {
            destination.push(path);
        }
    }
    destination.len() - before
}

/// Inverts selection across every regular file in an archive.
///
/// Directories remain navigation nodes rather than extraction selections. The
/// operation is linear in archive size and reuses the existing selection set,
/// matching the established all-files selection scope in both desktop UIs.
pub fn invert_archive_file_selection(
    archive: &ArchiveInfo,
    selected: &mut HashSet<PathBuf>,
) -> usize {
    for entry in archive.entries.iter().filter(|entry| !entry.is_directory) {
        if !selected.remove(&entry.path) {
            selected.insert(entry.path.clone());
        }
    }
    selected.len()
}

/// Adds the selected format's canonical extension when a save-dialog path has
/// no usable extension. An explicit user-entered extension is preserved.
pub fn ensure_archive_extension(path: PathBuf, format: ArchiveFormat) -> PathBuf {
    let needs_extension = path
        .extension()
        .is_none_or(|extension| extension.is_empty());
    if !needs_extension {
        return path;
    }
    let mut path = path;
    path.set_extension(format.canonical_extension());
    path
}

/// Requires both create-form password fields to match exactly.
///
/// Two empty values deliberately match and mean that encryption is disabled.
/// Keeping this rule shared prevents the default and accessible UIs from
/// diverging at the point where an unrecoverable archive password is chosen.
pub fn create_passwords_match(password: &str, confirmation: &str) -> bool {
    password == confirmation
}

/// Opens one of ZiFile's compile-time official destinations in the default browser.
///
/// Accepting an enum rather than an arbitrary URL keeps UI events from becoming a
/// general-purpose protocol launcher.
pub fn open_official_link(link: OfficialLink) -> std::io::Result<()> {
    let url = link.url();
    #[cfg(windows)]
    {
        use windows::Win32::UI::Shell::ShellExecuteW;
        use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        use windows::core::PCWSTR;

        let encoded = url
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect::<Vec<_>>();
        // SAFETY: every pointer references a live NUL-terminated UTF-16 buffer or is null;
        // ShellExecuteW does not retain these pointers after returning.
        let result = unsafe {
            ShellExecuteW(
                None,
                PCWSTR::null(),
                PCWSTR(encoded.as_ptr()),
                PCWSTR::null(),
                PCWSTR::null(),
                SW_SHOWNORMAL,
            )
        };
        if result.0 as isize <= 32 {
            return Err(std::io::Error::other(
                "Windows could not open the official link",
            ));
        }
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg(url).spawn().map(|_| ())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open").arg(url).spawn().map(|_| ())
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = url;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "no default browser launcher is available",
        ))
    }
}

/// Opens the containing folder and selects an archive when the host supports
/// a native file manager. The Windows implementation is the shipping path;
/// the other branches keep the desktop crate buildable for development.
pub fn reveal_in_file_manager(path: &Path) -> std::io::Result<()> {
    if !path.is_file() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "the archive is no longer a regular file",
        ));
    }
    #[cfg(windows)]
    {
        let argument = explorer_selection_argument(path);
        Command::new("explorer.exe")
            .arg(argument)
            .spawn()
            .map(|_| ())
    }
    #[cfg(target_os = "macos")]
    {
        Command::new("open").arg("-R").arg(path).spawn().map(|_| ())
    }
    #[cfg(all(unix, not(target_os = "macos")))]
    {
        Command::new("xdg-open")
            .arg(path.parent().unwrap_or_else(|| Path::new(".")))
            .spawn()
            .map(|_| ())
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "no native file manager is available",
        ))
    }
}

#[cfg(windows)]
fn explorer_selection_argument(path: &Path) -> String {
    format!("/select,\"{}\"", path.display())
}

fn paths_have_same_identity(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        let normalize = |path: &Path| {
            let mut value = path.to_string_lossy().replace('/', "\\").to_lowercase();
            while value.len() > 3 && value.ends_with('\\') {
                value.pop();
            }
            value
        };
        normalize(left) == normalize(right)
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dropped_archive_detection_prefers_signatures_over_extensions() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let renamed = temporary.path().join("archive.bin");
        std::fs::write(&renamed, b"PK\x03\x04minimal").expect("renamed archive");
        assert!(is_openable_archive_path(&renamed));

        let plain = temporary.path().join("notes.txt");
        std::fs::write(&plain, b"plain text").expect("plain file");
        assert!(!is_openable_archive_path(&plain));
    }

    #[test]
    fn archive_creation_requires_an_exact_password_confirmation() {
        assert!(create_passwords_match("", ""));
        assert!(create_passwords_match("correct horse", "correct horse"));
        assert!(!create_passwords_match("correct horse", "correct Horse"));
        assert!(!create_passwords_match("secret", ""));
        assert!(!create_passwords_match("", "secret"));
    }

    #[test]
    fn official_links_are_fixed_https_destinations() {
        let links = [
            OfficialLink::Project,
            OfficialLink::DocumentationZh,
            OfficialLink::DocumentationEn,
            OfficialLink::PrivacyZh,
            OfficialLink::PrivacyEn,
        ];
        for link in links {
            let url = link.url();
            assert!(url.starts_with("https://"));
            assert!(
                url.starts_with("https://github.com/ax2/zifile")
                    || url.starts_with("https://ax2.github.io/zifile/")
            );
            assert!(!url.contains(['\r', '\n', '"']));
        }
    }

    #[test]
    fn dropped_archive_detection_keeps_known_extension_fallback() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let mislabeled = temporary.path().join("archive.zip");
        std::fs::write(&mislabeled, b"not a complete archive").expect("archive placeholder");
        assert!(is_openable_archive_path(&mislabeled));
    }

    #[test]
    fn dropped_archive_detection_rejects_archive_named_directories() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let directory = temporary.path().join("folder.zip");
        std::fs::create_dir(&directory).expect("archive-named directory");
        assert!(!is_openable_archive_path(&directory));
    }

    #[test]
    fn reveal_rejects_a_missing_file_before_launching_a_file_manager() {
        let temporary = tempfile::tempdir().expect("temporary directory");
        let missing = temporary.path().join("gone.zip");
        let error = reveal_in_file_manager(&missing).expect_err("missing archive");
        assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
    }

    #[cfg(windows)]
    #[test]
    fn explorer_selection_argument_preserves_the_full_path() {
        let path = Path::new(r"C:\资料\示例 archive.zip");
        assert_eq!(
            explorer_selection_argument(path),
            r#"/select,"C:\资料\示例 archive.zip""#
        );
    }

    #[test]
    fn source_append_deduplicates_exact_paths() {
        let mut sources = vec![PathBuf::from("data/alpha.txt")];
        assert_eq!(
            append_unique_paths(
                &mut sources,
                vec![
                    PathBuf::from("data/alpha.txt"),
                    PathBuf::from("data/beta.txt")
                ]
            ),
            1
        );
        assert_eq!(sources.len(), 2);
    }

    #[test]
    fn archive_file_selection_inverts_files_and_ignores_directories() {
        use zifile_core::ArchiveEntryInfo;

        let archive = ArchiveInfo {
            path: PathBuf::from("sample.zip"),
            format: ArchiveFormat::Zip,
            entries: vec![
                ArchiveEntryInfo {
                    path: PathBuf::from("folder"),
                    size: 0,
                    compressed_size: 0,
                    is_directory: true,
                    encrypted: false,
                    checksum: None,
                    modified: None,
                },
                ArchiveEntryInfo {
                    path: PathBuf::from("folder/one.txt"),
                    size: 1,
                    compressed_size: 1,
                    is_directory: false,
                    encrypted: false,
                    checksum: None,
                    modified: None,
                },
                ArchiveEntryInfo {
                    path: PathBuf::from("two.txt"),
                    size: 1,
                    compressed_size: 1,
                    is_directory: false,
                    encrypted: false,
                    checksum: None,
                    modified: None,
                },
            ],
            total_size: 0,
            compressed_size: 0,
        };
        let mut selected = HashSet::from([PathBuf::from("folder/one.txt")]);
        assert_eq!(invert_archive_file_selection(&archive, &mut selected), 1);
        assert_eq!(selected, HashSet::from([PathBuf::from("two.txt")]));
    }

    #[test]
    fn save_paths_receive_only_the_missing_canonical_extension() {
        assert_eq!(
            ensure_archive_extension(PathBuf::from("backup"), ArchiveFormat::Zip),
            PathBuf::from("backup.zip")
        );
        assert_eq!(
            ensure_archive_extension(PathBuf::from("backup"), ArchiveFormat::TarGzip),
            PathBuf::from("backup.tar.gz")
        );
        assert_eq!(
            ensure_archive_extension(PathBuf::from("backup.custom"), ArchiveFormat::Zip),
            PathBuf::from("backup.custom")
        );
    }

    #[cfg(windows)]
    #[test]
    fn source_append_deduplicates_windows_case_and_slash_variants() {
        let mut sources = vec![PathBuf::from(r"C:\Data\Alpha.txt")];
        assert_eq!(
            append_unique_paths(
                &mut sources,
                vec![
                    PathBuf::from("c:/data/alpha.txt"),
                    PathBuf::from(r"C:\Data\Beta.txt")
                ]
            ),
            1
        );
        assert_eq!(sources.len(), 2);
    }

    #[cfg(windows)]
    #[test]
    fn source_append_deduplicates_non_ascii_case_variants() {
        let mut sources = vec![PathBuf::from(r"C:\资料\Ä.txt")];
        assert_eq!(
            append_unique_paths(&mut sources, vec![PathBuf::from(r"c:/资料/ä.txt")]),
            0
        );
        assert_eq!(sources.len(), 1);
    }
}
