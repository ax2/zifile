use std::fs;
use std::path::PathBuf;

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

    pub fn save(self) {
        let Some(path) = settings_path() else { return };
        let Some(parent) = path.parent() else { return };
        if fs::create_dir_all(parent).is_err() {
            return;
        }
        let _ = fs::write(path, self.contents());
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
}
