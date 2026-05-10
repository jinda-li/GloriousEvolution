use crate::{AppError, AppResult};

const NOISE_PHRASES: &[&str] = &[
    "啪塔声",
    "啪嗒声",
    "咔哒声",
    "咔嗒声",
    "敲击声",
    "点击声",
    "键盘声",
    "鼠标声",
    "背景音",
    "背景音乐",
    "音乐声",
    "掌声",
    "笑声",
    "噪音",
    "杂音",
    "沉默",
    "无语音",
];

pub fn clean_human_speech_text(text: &str) -> AppResult<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AppError::Audio("没有检测到有效的人声文本。".to_string()));
    }

    if cyrillic_ratio(trimmed) > 0.2 {
        return Err(AppError::Audio(
            "识别结果像俄文误识别，已忽略这段录音。".to_string(),
        ));
    }

    let cleaned = trimmed
        .lines()
        .map(remove_bracketed_noise)
        .map(|line| remove_noise_phrases(&line))
        .map(|line| line.trim().to_string())
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let cleaned = cleaned.trim().to_string();
    if cleaned.is_empty() || is_noise_only(trimmed) {
        return Err(AppError::Audio("没有检测到有效的人声文本。".to_string()));
    }

    Ok(cleaned)
}

fn cyrillic_ratio(text: &str) -> f32 {
    let letters = text.chars().filter(|ch| ch.is_alphabetic()).count();
    if letters == 0 {
        return 0.0;
    }

    let cyrillic = text
        .chars()
        .filter(|ch| ('\u{0400}'..='\u{04FF}').contains(ch))
        .count();

    cyrillic as f32 / letters as f32
}

fn remove_noise_phrases(text: &str) -> String {
    NOISE_PHRASES
        .iter()
        .fold(text.to_string(), |current, phrase| current.replace(phrase, ""))
}

fn remove_bracketed_noise(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut depth = 0u8;

    for ch in text.chars() {
        match ch {
            '[' | '【' | '(' | '（' => depth = depth.saturating_add(1),
            ']' | '】' | ')' | '）' => depth = depth.saturating_sub(1),
            _ if depth == 0 => output.push(ch),
            _ => {}
        }
    }

    output
}

fn is_noise_only(text: &str) -> bool {
    let without_noise = remove_noise_phrases(&remove_bracketed_noise(text));
    without_noise
        .chars()
        .all(|ch| ch.is_whitespace() || ch.is_ascii_punctuation() || "，。！？、；：…".contains(ch))
}
