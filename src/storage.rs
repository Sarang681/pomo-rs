//! Reading and writing the config file and the session history.
//!
//! A missing file is normal (first run) and gives defaults. A file that exists but can't be
//! read or parsed is an error, so a typo in the config never gets overwritten by a later save.

use crate::settings::Settings;
use crate::stats::History;
use color_eyre::eyre::{Context, Result};
use std::ffi::OsString;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub fn config_path() -> Option<PathBuf> {
    app_file(
        std::env::var_os("XDG_CONFIG_HOME"),
        std::env::var_os("HOME"),
        &[".config"],
        "config.toml",
    )
}

pub fn history_path() -> Option<PathBuf> {
    app_file(
        std::env::var_os("XDG_DATA_HOME"),
        std::env::var_os("HOME"),
        &[".local", "share"],
        "history.json",
    )
}

/// `$XDG_*_HOME/pomo-rs/<file>`, or `$HOME/<fallback>/pomo-rs/<file>` when the variable is unset.
fn app_file(
    xdg: Option<OsString>,
    home: Option<OsString>,
    fallback: &[&str],
    file: &str,
) -> Option<PathBuf> {
    let base = xdg
        .filter(|dir| !dir.is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            home.filter(|dir| !dir.is_empty())
                .map(|home| fallback.iter().fold(PathBuf::from(home), |p, s| p.join(s)))
        })?;
    Some(base.join("pomo-rs").join(file))
}

fn read_if_exists(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(text) => Ok(Some(text)),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error).wrap_err_with(|| format!("could not read {}", path.display())),
    }
}

/// Writes through a temporary file so a crash mid-write can't leave a half-written file.
fn write_atomic(path: &Path, text: &str) -> Result<()> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).wrap_err_with(|| format!("could not create {}", dir.display()))?;
    }
    let mut temp = path.as_os_str().to_owned();
    temp.push(".tmp");
    let temp = PathBuf::from(temp);

    fs::write(&temp, text).wrap_err_with(|| format!("could not write {}", temp.display()))?;
    fs::rename(&temp, path).wrap_err_with(|| format!("could not replace {}", path.display()))
}

pub fn load_settings(path: &Path) -> Result<Settings> {
    match read_if_exists(path)? {
        Some(text) => toml::from_str::<Settings>(&text)
            .map(Settings::sanitized)
            .wrap_err_with(|| format!("invalid config {}", path.display())),
        None => Ok(Settings::default()),
    }
}

pub fn save_settings(path: &Path, settings: &Settings) -> Result<()> {
    write_atomic(path, &toml::to_string_pretty(settings)?)
}

pub fn load_history(path: &Path) -> Result<History> {
    match read_if_exists(path)? {
        Some(text) => serde_json::from_str(&text)
            .wrap_err_with(|| format!("invalid history file {}", path.display())),
        None => Ok(History::default()),
    }
}

pub fn save_history(path: &Path, history: &History) -> Result<()> {
    write_atomic(path, &serde_json::to_string_pretty(history)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::SessionRecord;

    /// A fresh directory per test, removed on drop.
    struct TempDir(PathBuf);

    impl TempDir {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("pomo-rs-{}-{name}", std::process::id()));
            let _ = fs::remove_dir_all(&dir);
            fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        fn file(&self, name: &str) -> PathBuf {
            self.0.join(name)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn test_paths_prefer_the_xdg_variable() {
        let path = app_file(
            Some("/xdg".into()),
            Some("/home/me".into()),
            &[".config"],
            "c.toml",
        );

        assert_eq!(path, Some(PathBuf::from("/xdg/pomo-rs/c.toml")));
    }

    #[test]
    fn test_paths_fall_back_to_home() {
        let path = app_file(
            None,
            Some("/home/me".into()),
            &[".local", "share"],
            "h.json",
        );

        assert_eq!(
            path,
            Some(PathBuf::from("/home/me/.local/share/pomo-rs/h.json"))
        );
    }

    #[test]
    fn test_an_empty_xdg_variable_counts_as_unset() {
        let path = app_file(
            Some("".into()),
            Some("/home/me".into()),
            &[".config"],
            "c.toml",
        );

        assert_eq!(path, Some(PathBuf::from("/home/me/.config/pomo-rs/c.toml")));
    }

    #[test]
    fn test_no_home_means_no_path() {
        assert_eq!(app_file(None, None, &[".config"], "c.toml"), None);
    }

    #[test]
    fn test_missing_files_give_defaults() {
        let dir = TempDir::new("missing");

        assert_eq!(
            load_settings(&dir.file("c.toml")).unwrap(),
            Settings::default()
        );
        assert_eq!(
            load_history(&dir.file("h.json")).unwrap(),
            History::default()
        );
    }

    #[test]
    fn test_settings_survive_a_save_and_load_and_parent_dirs_are_created() {
        let dir = TempDir::new("settings");
        let path = dir.file("nested/deeper/c.toml");
        let settings = Settings {
            focus_minutes: 45,
            notifications: false,
            ..Settings::default()
        };

        save_settings(&path, &settings).unwrap();

        assert_eq!(load_settings(&path).unwrap(), settings);
        assert!(!dir.file("nested/deeper/c.toml.tmp").exists());
    }

    #[test]
    fn test_history_survives_a_save_and_load() {
        let dir = TempDir::new("history");
        let path = dir.file("h.json");
        let mut history = History::default();
        history.record(SessionRecord {
            day: 7,
            minutes: 25,
            task: "x".into(),
        });

        save_history(&path, &history).unwrap();

        assert_eq!(load_history(&path).unwrap(), history);
    }

    #[test]
    fn test_hand_edited_lengths_are_clamped_on_load() {
        let dir = TempDir::new("clamp");
        let path = dir.file("c.toml");
        fs::write(&path, "focus_minutes = 0\nshort_break_minutes = 1000").unwrap();

        let settings = load_settings(&path).unwrap();

        assert_eq!(settings.focus_minutes, 1);
        assert_eq!(settings.short_break_minutes, 99);
    }

    #[test]
    fn test_a_broken_config_is_an_error_naming_the_file() {
        let dir = TempDir::new("broken-config");
        let path = dir.file("c.toml");
        fs::write(&path, "focus_minutes = \"lots\"").unwrap();

        let error = load_settings(&path).unwrap_err();

        assert!(format!("{error:#}").contains("c.toml"));
    }

    #[test]
    fn test_a_broken_history_is_an_error_not_an_empty_history() {
        let dir = TempDir::new("broken-history");
        let path = dir.file("h.json");
        fs::write(&path, "{ not json").unwrap();

        assert!(load_history(&path).is_err());
    }
}
