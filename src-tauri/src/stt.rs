use reqwest::multipart;
use serde::Deserialize;

use crate::{
    api,
    i18n::{self, tr},
    settings::AppSettings,
    AppResult,
};

#[derive(Debug, Deserialize)]
struct TranscriptionResponse {
    #[serde(default)]
    text: String,
}

pub async fn transcribe(wav: Vec<u8>, settings: &AppSettings) -> AppResult<String> {
    let base_url = settings.base_url();
    let model = settings.stt_model.trim().to_string();
    let language = settings.language.trim().to_string();

    let response = api::send_with_retry(|| {
        let mut form = multipart::Form::new()
            .part(
                "file",
                multipart::Part::bytes(wav.clone())
                    .file_name("recording.wav")
                    .mime_str("audio/wav")
                    .expect("static mime type is valid"),
            )
            .text("model", model.clone())
            .text("temperature", "0");
        if !language.is_empty() && language != "auto" {
            form = form.text("language", language.clone());
        }

        api::authorized(
            api::client().post(format!("{base_url}/audio/transcriptions")),
            settings,
        )
        .multipart(form)
    })
    .await?;

    let payload: TranscriptionResponse = api::parse_json(response, tr(&i18n::STAGE_STT)).await?;
    Ok(payload.text.trim().to_string())
}
