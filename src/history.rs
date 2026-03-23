use crate::bench::{TestKind, TestResult};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const MAX_ENTRIES: usize = 50;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HistoryEntry {
    pub label: String,
    pub drive: String,
    pub results: Vec<(TestKind, TestResult)>,
}

pub fn load() -> Vec<HistoryEntry> {
    read_json(&history_path()).unwrap_or_default()
}

pub fn save(drive: &str, results: &[(TestKind, TestResult)]) -> Vec<HistoryEntry> {
    let mut history = load();

    history.insert(
        0,
        HistoryEntry {
            label: now_label(),
            drive: drive.to_string(),
            results: results.to_vec(),
        },
    );
    history.truncate(MAX_ENTRIES);
    write_json(&history_path(), &history);
    history
}

fn data_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home)
        .join(".local")
        .join("share")
        .join("diskrush")
}

fn history_path() -> PathBuf {
    data_dir().join("history.json")
}

fn config_path() -> PathBuf {
    data_dir().join("config.json")
}

fn read_json<T: serde::de::DeserializeOwned>(path: &std::path::Path) -> Option<T> {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|d| serde_json::from_str(&d).ok())
}

fn write_json<T: serde::Serialize>(path: &std::path::Path, val: &T) {
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(data) = serde_json::to_string(val) {
        let _ = std::fs::write(path, data);
    }
}

#[derive(Default, Serialize, Deserialize)]
struct Config {
    #[serde(skip_serializing_if = "Option::is_none")]
    test_size_mb: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    fullscreen: Option<bool>,
}

pub fn load_settings() -> (Option<u64>, Option<bool>) {
    let config: Config = read_json(&config_path()).unwrap_or_default();
    (config.test_size_mb, config.fullscreen)
}

fn save_setting(f: impl FnOnce(&mut Config)) {
    let mut config: Config = read_json(&config_path()).unwrap_or_default();
    f(&mut config);
    write_json(&config_path(), &config);
}

pub fn save_test_size(size_mb: u64) {
    save_setting(|c| c.test_size_mb = Some(size_mb));
}

pub fn save_fullscreen(fullscreen: bool) {
    save_setting(|c| c.fullscreen = Some(fullscreen));
}

#[cfg(unix)]
fn now_label() -> String {
    unsafe {
        let t = libc::time(std::ptr::null_mut());
        let tm = libc::localtime(&t);
        if tm.is_null() {
            return "Unknown".into();
        }
        let tm = *tm;
        format!(
            "{:02}/{:02} {:02}:{:02}",
            tm.tm_mon + 1,
            tm.tm_mday,
            tm.tm_hour,
            tm.tm_min
        )
    }
}

#[cfg(not(unix))]
fn now_label() -> String {
    "Run".into()
}
