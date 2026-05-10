use serde::{Deserialize, Serialize};

use crate::{settings::AppSettings, AppResult};

#[derive(Debug, Serialize)]
struct ChatRequest<'a> {
    model: &'a str,
    messages: Vec<ChatMessage<'a>>,
    temperature: f32,
}

#[derive(Debug, Serialize)]
struct ChatMessage<'a> {
    role: &'a str,
    content: &'a str,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<ChatChoice>,
}

#[derive(Debug, Deserialize)]
struct ChatChoice {
    message: ChatChoiceMessage,
}

#[derive(Debug, Deserialize)]
struct ChatChoiceMessage {
    content: String,
}

pub async fn optimize(raw_text: &str, settings: &AppSettings) -> AppResult<String> {
    if raw_text.trim().is_empty() {
        return Ok(String::new());
    }

    if settings.optimizer_api_key.trim().is_empty() {
        return Err(crate::AppError::Config(
            "请先在设置中填写文本模型 API Key。".to_string(),
        ));
    }

    if settings.optimizer_base_url.trim().is_empty() {
        return Err(crate::AppError::Config(
            "请先在设置中填写文本模型 API Base URL。".to_string(),
        ));
    }

    if settings.optimizer_model.trim().is_empty() {
        return Err(crate::AppError::Config(
            "请先在设置中填写文本模型名称。".to_string(),
        ));
    }

    let base_url = settings.optimizer_base_url.trim().trim_end_matches('/');
    let request = ChatRequest {
        model: settings.optimizer_model.trim(),
        temperature: 0.2,
        messages: vec![
            ChatMessage {
                role: "system",
                content: &settings.system_prompt,
            },
            ChatMessage {
                role: "user",
                content: raw_text,
            },
        ],
    };

    let mut builder = reqwest::Client::new()
        .post(format!("{base_url}/chat/completions"))
        .bearer_auth(settings.optimizer_api_key.trim());

    if base_url.contains("openrouter.ai") {
        builder = builder
            .header("HTTP-Referer", "https://github.com/glorious-evolution")
            .header("X-Title", "GloriousEvolution");
    }

    let response = builder.json(&request).send().await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(crate::AppError::Api(format!(
            "文本优化模型请求失败：{status}；Base URL: {base_url}；Model: {}；{body}",
            settings.optimizer_model.trim()
        )));
    }

    let payload: ChatResponse = response.json().await?;
    let text = payload
        .choices
        .first()
        .map(|choice| choice.message.content.trim().to_string())
        .unwrap_or_else(|| raw_text.to_string());

    Ok(text)
}
