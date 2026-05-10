use serde::{Deserialize, Serialize};
use std::{fs, path::PathBuf};
use tauri::{AppHandle, Manager};

use crate::AppResult;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    pub elevenlabs_api_key: String,
    pub optimizer_api_key: String,
    pub optimizer_base_url: String,
    pub optimizer_model: String,
    pub shortcut: String,
    pub auto_paste: bool,
    pub system_prompt: String,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            elevenlabs_api_key: String::new(),
            optimizer_api_key: String::new(),
            optimizer_base_url: "https://api.openai.com/v1".to_string(),
            optimizer_model: "gpt-4.1-mini".to_string(),
            shortcut: "Alt+Q".to_string(),
            auto_paste: true,
            system_prompt: default_system_prompt(),
        }
    }
}

pub fn default_system_prompt() -> String {
    "你是一个语音输入文本优化助手。只处理人的语音内容：请删除背景音、拟声词、音效描述、音乐/掌声/噪声说明和明显误识别的非目标语言内容。请在保留原意的前提下，修正语病、标点和口语化停顿，优化措辞和结构，让文本自然、清晰、适合直接发送。不要添加原文没有的事实，不要解释你的修改，只输出优化后的文本。如果没有有效的人声文本，输出空字符串。".to_string()
}

pub fn load(app: &AppHandle) -> AppResult<AppSettings> {
    let path = settings_path(app)?;
    if !path.exists() {
        return Ok(AppSettings::default());
    }

    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text).unwrap_or_default())
}

pub fn save(app: &AppHandle, settings: &AppSettings) -> AppResult<()> {
    let path = settings_path(app)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }

    let text = serde_json::to_string_pretty(settings)?;
    fs::write(path, text)?;
    Ok(())
}

fn settings_path(app: &AppHandle) -> AppResult<PathBuf> {
    let dir = app.path().app_config_dir()?;
    Ok(dir.join("settings.json"))
}
