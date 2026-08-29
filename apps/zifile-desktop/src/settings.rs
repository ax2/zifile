use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use crate::i18n::Locale;

#[derive(Debug, Clone, Copy)]
pub struct AppSettings {
    pub locale: Locale,
    pub dark: bool,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            locale: Locale::detect(),
            dark: true,
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
                _ => {}
            }
        }
    }

    pub fn save(self) -> io::Result<()> {
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

    fn save_to_path(self, path: &Path) -> io::Result<()> {
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

    fn contents(self) -> String {
        let theme = if self.dark { "dark" } else { "light" };
        format!("language={}\ntheme={theme}\n", self.locale.code())
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
        }
        .save_to_path(&path)
        .unwrap();

        assert_eq!(
            fs::read_to_string(&path).unwrap(),
            "language=zh-CN\ntheme=light\n"
        );
        assert!(!path.with_extension("conf.tmp").exists());
    }
}
