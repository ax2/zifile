use std::ffi::OsString;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StartupRequest {
    Home,
    OpenArchive(PathBuf),
    ExtractHere(PathBuf),
    CreateFrom(Vec<PathBuf>),
}

pub fn parse<I>(arguments: I) -> StartupRequest
where
    I: IntoIterator<Item = OsString>,
{
    let values = arguments.into_iter().collect::<Vec<_>>();
    if values.first().is_some_and(|value| value == "--create") {
        let sources = values.into_iter().skip(1).map(PathBuf::from).collect();
        return StartupRequest::CreateFrom(sources);
    }
    if values
        .first()
        .is_some_and(|value| value == "--extract-here")
    {
        return values
            .into_iter()
            .nth(1)
            .map(PathBuf::from)
            .map_or(StartupRequest::Home, StartupRequest::ExtractHere);
    }
    values
        .into_iter()
        .next()
        .map(PathBuf::from)
        .map_or(StartupRequest::Home, StartupRequest::OpenArchive)
}

pub fn extraction_destination(archive: &Path) -> PathBuf {
    archive
        .parent()
        .unwrap_or_else(|| Path::new("."))
        .join(extraction_folder_name(archive))
}

fn extraction_folder_name(archive: &Path) -> OsString {
    const TAR_SUFFIXES: &[&str] = &[
        ".tar.gz",
        ".tar.zst",
        ".tar.xz",
        ".tar.lzma",
        ".tar.bz",
        ".tar.bz2",
        ".tar.lz4",
        ".tar.br",
        ".tgz",
        ".tzst",
        ".txz",
        ".tbz",
        ".tbz2",
    ];

    if let Some(file_name) = archive.file_name().and_then(|value| value.to_str()) {
        let lowercase = file_name.to_ascii_lowercase();
        if let Some(suffix) = TAR_SUFFIXES
            .iter()
            .find(|suffix| lowercase.ends_with(**suffix))
        {
            let stem_length = file_name.len() - suffix.len();
            if stem_length > 0 {
                return OsString::from(&file_name[..stem_length]);
            }
        }
    }

    archive.file_stem().unwrap_or_default().to_os_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_arguments_opens_home() {
        assert_eq!(parse(Vec::<OsString>::new()), StartupRequest::Home);
    }

    #[test]
    fn positional_path_preserves_file_association_behavior() {
        assert_eq!(
            parse([OsString::from(r"C:\work\sample.zip")]),
            StartupRequest::OpenArchive(PathBuf::from(r"C:\work\sample.zip"))
        );
    }

    #[test]
    fn create_mode_preserves_multiple_unicode_sources() {
        assert_eq!(
            parse([
                OsString::from("--create"),
                OsString::from(r"C:\资料\甲.txt"),
                OsString::from(r"C:\资料\乙 folder"),
            ]),
            StartupRequest::CreateFrom(vec![
                PathBuf::from(r"C:\资料\甲.txt"),
                PathBuf::from(r"C:\资料\乙 folder"),
            ])
        );
    }

    #[test]
    fn extract_here_requires_and_preserves_one_archive_path() {
        assert_eq!(
            parse([
                OsString::from("--extract-here"),
                OsString::from(r"C:\资料\示例.zip"),
            ]),
            StartupRequest::ExtractHere(PathBuf::from(r"C:\资料\示例.zip"))
        );
        assert_eq!(
            parse([OsString::from("--extract-here")]),
            StartupRequest::Home
        );
    }

    #[test]
    fn extract_here_destination_is_a_matching_sibling_folder() {
        assert_eq!(
            extraction_destination(Path::new(r"C:\资料\示例.zip")),
            PathBuf::from(r"C:\资料\示例")
        );
        assert_eq!(
            extraction_destination(Path::new(r"C:\资料\backup.tar.gz")),
            PathBuf::from(r"C:\资料\backup")
        );
    }

    #[test]
    fn extract_here_destination_collapses_tar_stream_aliases() {
        for archive_name in [
            "backup.tar.gz",
            "backup.TAR.ZST",
            "backup.tar.xz",
            "backup.tar.lzma",
            "backup.tar.bz",
            "backup.tar.bz2",
            "backup.tar.lz4",
            "backup.tar.br",
            "backup.tgz",
            "backup.tzst",
            "backup.txz",
            "backup.tbz",
            "backup.tbz2",
        ] {
            assert_eq!(
                extraction_destination(Path::new(archive_name)),
                PathBuf::from("backup"),
                "unexpected destination for {archive_name}"
            );
        }
        assert_eq!(
            extraction_destination(Path::new("backup.tar.zip")),
            PathBuf::from("backup.tar")
        );
    }
}
