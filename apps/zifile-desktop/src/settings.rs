use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::i18n::Locale;

pub const MAX_RECENT_ARCHIVES: usize = 8;

#[derive(Debug, Clone)]
pub struct AppSettings {
    pub locale: Locale,
    pub dark: bool,
    pub recent_archives: Vec<PathBuf>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: Locale::detect(),
            dark: true,
            recent_archives: Vec::new(),
        }
    }
}

impl AppSettings {
    pub fn load() -> Self {
        let mut settings = Self::default();
        let Some(path) = settings_path() else {
            return settings;
        };
        let Ok(contents) = fs::read_to_string(path) else {
            return settings;
        };
        settings.apply_contents(&contents);
        settings
    }

    fn apply_contents(&mut self, contents: &str) {
        for line in contents.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match (key.trim(), value.trim()) {
                ("language", "zh-CN") => self.locale = Locale::ZhCn,
                ("language", "en-US") => self.locale = Locale::En,
                ("theme", "light") => self.dark = false,
                ("theme", "dark") => self.dark = true,
                ("recent_archive", encoded) if self.recent_archives.len() < MAX_RECENT_ARCHIVES => {
                    if let Some(path) = decode_path(encoded)
                        && !self
                            .recent_archives
                            .iter()
                            .any(|existing| same_path(existing, &path))
                    {
                        self.recent_archives.push(path);
                    }
                }
                _ => {}
            }
        }
    }

    pub fn record_recent_archive(&mut self, path: PathBuf) {
        self.recent_archives
            .retain(|existing| !same_path(existing, &path));
        self.recent_archives.insert(0, path);
        self.recent_archives.truncate(MAX_RECENT_ARCHIVES);
    }

    pub fn save(&self) -> io::Result<()> {
        let Some(path) = settings_path() else {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                "LOCALAPPDATA is not available",
            ));
        };
        let Some(parent) = path.parent() else {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "settings path has no parent directory",
            ));
        };
        fs::create_dir_all(parent)?;
        self.save_to_path(&path)
    }

    fn save_to_path(&self, path: &Path) -> io::Result<()> {
        let temporary = path.with_extension("conf.tmp");
        let result = (|| {
            let mut file = fs::OpenOptions::new()
                .create(true)
                .truncate(true)
                .write(true)
                .open(&temporary)?;
            file.write_all(self.contents().as_bytes())?;
            file.flush()?;
            file.sync_all()?;
            replace_file(&temporary, path)
        })();
        if result.is_err() {
            let _ = fs::remove_file(&temporary);
        }
        result
    }

    fn contents(&self) -> String {
        let theme = if self.dark { "dark" } else { "light" };
        let mut contents = format!("language={}\ntheme={theme}\n", self.locale.code());
        for path in &self.recent_archives {
            contents.push_str("recent_archive=");
            contents.push_str(&encode_path(path));
            contents.push('\n');
        }
        contents
    }
}

fn encode_path(path: &Path) -> String {
    path.to_string_lossy()
        .as_bytes()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn decode_path(encoded: &str) -> Option<PathBuf> {
    if encoded.is_empty() || encoded.len() > 32_768 || !encoded.len().is_multiple_of(2) {
        return None;
    }
    let bytes = encoded
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let pair = std::str::from_utf8(pair).ok()?;
            u8::from_str_radix(pair, 16).ok()
        })
        .collect::<Option<Vec<_>>>()?;
    let path = String::from_utf8(bytes).ok()?;
    (!path.is_empty()).then(|| PathBuf::from(path))
}

fn same_path(left: &Path, right: &Path) -> bool {
    #[cfg(windows)]
    {
        let normalized = |path: &Path| path.to_string_lossy().replace('/', "\\").to_lowercase();
        normalized(left) == normalized(right)
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn settings_path() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|root| root.join("ZiFile").join("settings.conf"))
}

#[cfg(windows)]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    use std::os::windows::ffi::OsStrExt;

    use windows_sys::Win32::Storage::FileSystem::{
        MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH, MoveFileExW,
    };

    let temporary: Vec<u16> = temporary
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    let destination: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect();
    // SAFETY: both vectors are NUL-terminated UTF-16 paths that remain alive
    // for the duration of the synchronous Win32 call.
    let moved = unsafe {
        MoveFileExW(
            temporary.as_ptr(),
            destination.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if moved == 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(temporary: &Path, destination: &Path) -> io::Result<()> {
    fs::rename(temporary, destination)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn applies_supported_settings_and_ignores_unknown_lines() {
        let mut settings = AppSettings {
            locale: Locale::En,
            dark: true,
            recent_archives: Vec::new(),
        };
        settings.apply_contents("language=zh-CN\ntheme=light\nfuture=value\n");
        assert_eq!(settings.locale, Locale::ZhCn);
        assert!(!settings.dark);
    }

    #[test]
    fn serialized_settings_have_a_strict_non_secret_schema() {
        let settings = AppSettings {
            locale: Locale::ZhCn,
            dark: false,
            recent_archives: Vec::new(),
        };
        assert_eq!(settings.contents(), "language=zh-CN\ntheme=light\n");
    }

    #[test]
    fn save_to_path_replaces_existing_settings_and_cleans_the_temporary_file() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("settings.conf");
        fs::write(&path, "language=en-US\ntheme=dark\n").unwrap();

        AppSettings {
            locale: Locale::ZhCn,
            dark: false,
            recent_archives: Vec::new(),
        }
        .save_to_path(&path)
        .unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "language=zh-CN\ntheme=light\n"
        );
        assert!(!path.with_extension("conf.tmp").exists());
    }

    #[test]
    fn recent_archives_round_trip_without_delimiter_injection() {
        let path = PathBuf::from(r"C:\资料\work=sample.zip");
        let mut settings = AppSettings {
            locale: Locale::ZhCn,
            dark: true,
            recent_archives: Vec::new(),
        };
        settings.record_recent_archive(path.clone());
        let contents = settings.contents();
        assert!(!contents.contains("资料"));

        let mut restored = AppSettings::default();
        restored.apply_contents(&contents);
        assert_eq!(restored.recent_archives, vec![path]);
    }

    #[test]
    fn recent_archives_are_most_recent_first_deduplicated_and_bounded() {
        let mut settings = AppSettings::default();
        for index in 0..=MAX_RECENT_ARCHIVES {
            settings.record_recent_archive(PathBuf::from(format!("archive-{index}.zip")));
        }
        settings.record_recent_archive(PathBuf::from("archive-4.zip"));
        assert_eq!(settings.recent_archives.len(), MAX_RECENT_ARCHIVES);
        assert_eq!(settings.recent_archives[0], PathBuf::from("archive-4.zip"));
        assert_eq!(
            settings
                .recent_archives
                .iter()
                .filter(|path| *path == &PathBuf::from("archive-4.zip"))
                .count(),
            1
        );
    }

    #[cfg(windows)]
    #[test]
    fn recent_archives_deduplicate_windows_case_and_separator_variants() {
        let mut settings = AppSettings::default();
        settings.record_recent_archive(PathBuf::from(r"C:\Data\Sample.zip"));
        settings.record_recent_archive(PathBuf::from("c:/data/sample.ZIP"));
        assert_eq!(
            settings.recent_archives,
            vec![PathBuf::from("c:/data/sample.ZIP")]
        );
    }

    #[test]
    fn malformed_recent_archive_lines_are_ignored() {
        let mut settings = AppSettings::default();
        settings.apply_contents("recent_archive=not-hex\nrecent_archive=0\nrecent_archive=\n");
        assert!(settings.recent_archives.is_empty());
    }
}
