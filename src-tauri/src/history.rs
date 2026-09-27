use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

use crate::AppResult;

const MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
    pub id: String,
    pub created_at: u64,
    pub raw_text: String,
    pub text: String,
    pub duration_seconds: f64,
    pub polished: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UsageStats {
    pub total_seconds: f64,
    pub total_words: u64,
    pub total_sessions: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoryStore {
    pub stats: UsageStats,
    pub entries: Vec<HistoryEntry>,
}

pub fn load(app: &AppHandle) -> AppResult<HistoryStore> {
    let path = history_path(app)?;
    if !path.exists() {
        return Ok(HistoryStore::default());
    }
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

fn save(app: &AppHandle, store: &HistoryStore) -> AppResult<()> {
    let path = history_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_string(store)?)?;
    fs::rename(tmp, path)?;
    Ok(())
}

pub fn record(
    app: &AppHandle,
    keep_entry: bool,
    raw_text: &str,
    text: &str,
    duration_seconds: f64,
    polished: bool,
) -> AppResult<HistoryStore> {
    let mut store = load(app)?;
    store.stats.total_seconds += duration_seconds;
    store.stats.total_words += count_words(text) as u64;
    store.stats.total_sessions += 1;

    if keep_entry {
        store.entries.insert(
            0,
            HistoryEntry {
                id: uuid::Uuid::new_v4().to_string(),
                created_at: now_millis(),
                raw_text: raw_text.to_string(),
                text: text.to_string(),
                duration_seconds,
                polished,
            },
        );
        store.entries.truncate(MAX_ENTRIES);
    }
    save(app, &store)?;
    Ok(store)
}

pub fn delete(app: &AppHandle, id: &str) -> AppResult<HistoryStore> {
    let mut store = load(app)?;
    store.entries.retain(|entry| entry.id != id);
    save(app, &store)?;
    Ok(store)
}

pub fn clear(app: &AppHandle) -> AppResult<HistoryStore> {
    let mut store = load(app)?;
    store.entries.clear();
    save(app, &store)?;
    Ok(store)
}

/// CJK characters count individually; Latin text counts by word.
pub fn count_words(text: &str) -> usize {
    let mut count = 0;
    let mut in_word = false;
    for ch in text.chars() {
        if is_cjk(ch) {
            count += 1;
            in_word = false;
        } else if ch.is_alphanumeric() || ch == '\'' || ch == '-' {
            if !in_word {
                count += 1;
                in_word = true;
            }
        } else {
            in_word = false;
        }
    }
    count
}

fn is_cjk(ch: char) -> bool {
    matches!(ch as u32,
        0x3040..=0x30FF | 0x3400..=0x4DBF | 0x4E00..=0x9FFF | 0xAC00..=0xD7AF | 0xF900..=0xFAFF)
}

fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

fn history_path(app: &AppHandle) -> AppResult<PathBuf> {
    Ok(app.path().app_data_dir()?.join("history.json"))
}

#[cfg(test)]
mod tests {
    use super::count_words;

    #[test]
    fn counts_mixed_text() {
        assert_eq!(count_words("你好 world, it's fine"), 5);
        assert_eq!(count_words(""), 0);
    }
}
