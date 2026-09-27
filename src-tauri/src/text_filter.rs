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

/// Whole-transcript phrases speech models are known to hallucinate on silence.
const HALLUCINATIONS: &[&str] = &[
    "谢谢观看",
    "谢谢大家观看",
    "感谢观看",
    "请不吝点赞 订阅 转发 打赏支持明镜与点点栏目",
    "字幕由Amara.org社区提供",
    "字幕志愿者 杨茜茜",
    "Thank you for watching",
    "Thanks for watching",
    "you",
];

pub fn clean_human_speech_text(text: &str, language: &str) -> AppResult<String> {
    let trimmed = text.trim();
    if trimmed.is_empty() || is_hallucination(trimmed) {
        return Err(AppError::Audio("没有检测到有效的人声文本。".to_string()));
    }

    let expects_chinese = matches!(language, "auto" | "zh" | "");
    if expects_chinese && cyrillic_ratio(trimmed) > 0.2 {
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

fn is_hallucination(text: &str) -> bool {
    let normalized = text.trim_matches(|ch: char| ch.is_ascii_punctuation() || "，。！？…、 ".contains(ch));
    HALLUCINATIONS
        .iter()
        .any(|phrase| normalized.eq_ignore_ascii_case(phrase))
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

#[cfg(test)]
mod tests {
    use super::clean_human_speech_text;

    #[test]
    fn drops_silence_hallucinations() {
        assert!(clean_human_speech_text("谢谢观看。", "auto").is_err());
        assert!(clean_human_speech_text("Thank you for watching!", "auto").is_err());
    }

    #[test]
    fn keeps_speech_and_strips_noise_tags() {
        assert_eq!(
            clean_human_speech_text("[键盘声] 明天下午开会", "auto").unwrap(),
            "明天下午开会"
        );
    }
}
