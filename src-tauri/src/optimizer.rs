use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::{
    api,
    i18n::{self, tr, Locale},
    settings::{self, AppSettings},
    AppResult,
};

#[derive(Debug, Serialize)]
struct ChatMessage {
    role: &'static str,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    #[serde(default)]
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    #[serde(default)]
    content: Option<String>,
}

pub async fn optimize(raw_text: &str, settings: &AppSettings) -> AppResult<String> {
    if raw_text.trim().is_empty() {
        return Ok(String::new());
    }

    let base_url = settings.base_url();
    let mut system_prompt = settings.system_prompt.trim().to_string();
    if i18n::current() == Locale::ZhTw && system_prompt == settings::default_system_prompt().trim() {
        // The built-in prompt asks for Simplified Chinese; Traditional users want the opposite.
        system_prompt = system_prompt.replace(settings::SIMPLIFIED_RULE, settings::TRADITIONAL_RULE);
    }
    let terms = settings.dictionary_terms();
    if !terms.is_empty() {
        system_prompt.push_str(
            "\n\n用户词典（识别结果里发音相近的词应替换为以下正确写法）：\n",
        );
        system_prompt.push_str(&terms.join("、"));
    }

    let messages = vec![
        ChatMessage {
            role: "system",
            content: system_prompt,
        },
        ChatMessage {
            role: "user",
            content: format!("<transcript>\n{}\n</transcript>", raw_text.trim()),
        },
    ];

    let mut body = json!({
        "model": settings.llm_model.trim(),
        "messages": messages,
        "temperature": 0.2,
        "max_tokens": (raw_text.chars().count() * 4).clamp(1024, 8192),
    });
    if settings.is_openrouter() {
        // "none" (not "minimal"): minimal switches thinking ON for hybrid models
        // like Gemini Flash Lite, adding latency and eating the output budget.
        body["reasoning"] = json!({ "effort": "none", "exclude": true });
        body["provider"] = json!({ "sort": "latency" });
    }

    let response = api::send_with_retry(|| {
        api::authorized(
            api::client().post(format!("{base_url}/chat/completions")),
            settings,
        )
        .json(&body)
    })
    .await?;

    let payload: ChatResponse = api::parse_json(response, tr(&i18n::STAGE_POLISH)).await?;
    let text = payload
        .choices
        .into_iter()
        .next()
        .and_then(|choice| choice.message.content)
        .map(|content| strip_wrapping(&content))
        .unwrap_or_default();

    Ok(text)
}

/// Models occasionally echo the transcript tags or wrap output in quotes.
fn strip_wrapping(text: &str) -> String {
    let mut text = text.trim();
    for tag in ["<transcript>", "</transcript>"] {
        text = text.trim_start_matches(tag).trim_end_matches(tag).trim();
    }
    if text.starts_with("```") && text.ends_with("```") && text.len() > 6 {
        text = text[3..text.len() - 3].trim();
    }
    text.to_string()
}

#[cfg(test)]
mod tests {
    use super::strip_wrapping;

    #[test]
    fn strips_echoed_tags() {
        assert_eq!(strip_wrapping("<transcript>\n你好。\n</transcript>"), "你好。");
        assert_eq!(strip_wrapping("  plain  "), "plain");
    }
}
