use std::path::{Component, Path, PathBuf};

use crate::{ZiFileError, ZiFileResult};

/// Converts an archive member name into a safe relative filesystem path.
///
/// The policy is intentionally stricter than the host filesystem: it rejects
/// traversal, absolute paths, Windows device names, alternate data streams,
/// trailing dots/spaces and paths deeper than the configured limit.
pub fn safe_relative_path(name: &str, max_depth: u16) -> ZiFileResult<PathBuf> {
    if name.is_empty() || name.contains('\0') {
        return Err(ZiFileError::UnsafePath(name.to_owned()));
    }

    let normalized = name.replace('\\', "/");
    if normalized.starts_with('/') || normalized.starts_with("//") {
        return Err(ZiFileError::UnsafePath(name.to_owned()));
    }

    let candidate = Path::new(&normalized);
    let mut safe = PathBuf::new();
    let mut depth = 0_u16;

    for component in candidate.components() {
        match component {
            Component::Normal(value) => {
                let value = value.to_string_lossy();
                validate_component(&value, name)?;
                depth = depth
                    .checked_add(1)
                    .ok_or_else(|| ZiFileError::UnsafePath(name.to_owned()))?;
                if depth > max_depth {
                    return Err(ZiFileError::LimitExceeded(format!(
                        "path depth exceeds {max_depth}: {name}"
                    )));
                }
                safe.push(value.as_ref());
            }
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => {
                return Err(ZiFileError::UnsafePath(name.to_owned()));
            }
        }
    }

    if safe.as_os_str().is_empty() {
        return Err(ZiFileError::UnsafePath(name.to_owned()));
    }
    Ok(safe)
}

fn validate_component(component: &str, original: &str) -> ZiFileResult<()> {
    if component.is_empty()
        || component.ends_with(['.', ' '])
        || component.contains(':')
        || component.chars().any(|character| character < '\u{20}')
    {
        return Err(ZiFileError::UnsafePath(original.to_owned()));
    }

    let stem = component
        .split('.')
        .next()
        .unwrap_or(component)
        .to_ascii_uppercase();
    let reserved = matches!(stem.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || stem.strip_prefix("COM").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        })
        || stem.strip_prefix("LPT").is_some_and(|suffix| {
            matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
        });

    if reserved {
        return Err(ZiFileError::UnsafePath(original.to_owned()));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepts_portable_relative_paths() {
        assert_eq!(
            safe_relative_path("folder\\子目录/file.txt", 8).unwrap(),
            PathBuf::from("folder/子目录/file.txt")
        );
    }

    #[test]
    fn rejects_traversal_and_absolute_paths() {
        for path in ["../escape", "folder/../../escape", "/root", "C:/Windows"] {
            assert!(safe_relative_path(path, 8).is_err(), "accepted {path}");
        }
    }

    #[test]
    fn rejects_windows_ambiguous_names() {
        for path in ["CON", "aux.txt", "LPT9.log", "file:stream", "name. "] {
            assert!(safe_relative_path(path, 8).is_err(), "accepted {path}");
        }
    }

    #[test]
    fn enforces_depth_limit() {
        assert!(safe_relative_path("a/b/c", 2).is_err());
    }
}
