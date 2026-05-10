use reqwest::multipart;
use serde::Deserialize;
use std::path::Path;

use crate::AppResult;

#[derive(Debug, Deserialize)]
struct ElevenLabsSttResponse {
    text: String,
}

pub async fn transcribe(audio_path: &Path, api_key: &str) -> AppResult<String> {
    if api_key.trim().is_empty() {
        return Err(crate::AppError::Config(
            "请先在设置中填写 ElevenLabs API Key。".to_string(),
        ));
    }

    let audio = tokio::fs::read(audio_path).await?;
    let file = multipart::Part::bytes(audio).file_name("recording.wav");
    let form = multipart::Form::new()
        .part("file", file)
        .text("model_id", "scribe_v1");

    let response = reqwest::Client::new()
        .post("https://api.elevenlabs.io/v1/speech-to-text")
        .header("xi-api-key", api_key)
        .multipart(form)
        .send()
        .await?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        return Err(crate::AppError::Api(format!(
            "ElevenLabs STT 请求失败：{status} {body}"
        )));
    }

    let payload: ElevenLabsSttResponse = response.json().await?;
    Ok(payload.text)
}
