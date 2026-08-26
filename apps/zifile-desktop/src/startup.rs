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
        .join(archive.file_stem().unwrap_or_default())
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
            PathBuf::from(r"C:\资料\backup.tar")
        );
    }
}
