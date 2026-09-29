use std::{sync::OnceLock, time::Duration};

use reqwest::{RequestBuilder, Response, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    i18n::{self, tr, trf},
    settings::AppSettings,
    AppError, AppResult,
};

const MAX_ATTEMPTS: usize = 2;

pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(30))
            .user_agent(concat!("Sayso/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client")
    })
}

pub fn authorized(builder: RequestBuilder, settings: &AppSettings) -> RequestBuilder {
    let mut builder = builder.bearer_auth(settings.api_key.trim());
    if settings.is_openrouter() {
        builder = builder
            .header("HTTP-Referer", "https://github.com/jinda-li/Sayso")
            .header("X-Title", "Sayso");
    }
    builder
}

/// Sends a request, retrying once on connection errors, 429 and 5xx responses.
pub async fn send_with_retry<F>(build: F) -> AppResult<Response>
where
    F: Fn() -> RequestBuilder,
{
    let mut attempt = 0;
    loop {
        attempt += 1;
        match build().send().await {
            Ok(response) => {
                let status = response.status();
                let retryable =
                    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error();
                if retryable && attempt < MAX_ATTEMPTS {
                    tokio::time::sleep(Duration::from_millis(600)).await;
                    continue;
                }
                return Ok(response);
            }
            Err(error) if attempt < MAX_ATTEMPTS && (error.is_connect() || error.is_timeout()) => {
                tokio::time::sleep(Duration::from_millis(600)).await;
            }
            Err(error) => return Err(network_error(error)),
        }
    }
}

pub async fn parse_json<T: DeserializeOwned>(response: Response, stage: &str) -> AppResult<T> {
    let status = response.status();
    if !status.is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(AppError::Api(describe_failure(stage, status, &body)));
    }
    response
        .json::<T>()
        .await
        .map_err(|error| AppError::Api(trf(&i18n::PARSE_FAILED, &[&stage, &error])))
}

fn network_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        AppError::Api(tr(&i18n::NET_TIMEOUT).to_string())
    } else if error.is_connect() {
        AppError::Api(tr(&i18n::NET_CONNECT).to_string())
    } else {
        AppError::Api(trf(&i18n::NET_FAILED, &[&error]))
    }
}

#[derive(Deserialize)]
struct ErrorEnvelope {
    error: Option<ErrorBody>,
}

#[derive(Deserialize)]
struct ErrorBody {
    message: Option<String>,
}

fn describe_failure(stage: &str, status: StatusCode, body: &str) -> String {
    let detail = serde_json::from_str::<ErrorEnvelope>(body)
        .ok()
        .and_then(|envelope| envelope.error)
        .and_then(|error| error.message)
        .unwrap_or_else(|| body.chars().take(200).collect());

    let hint = match status.as_u16() {
        401 => tr(&i18n::HTTP_401).to_string(),
        402 => tr(&i18n::HTTP_402).to_string(),
        403 => trf(&i18n::HTTP_403, &[&detail]),
        404 => trf(&i18n::HTTP_404, &[&detail]),
        408 | 504 => tr(&i18n::HTTP_TIMEOUT).to_string(),
        429 => tr(&i18n::HTTP_429).to_string(),
        _ => format!("{} {detail}", status.as_u16()),
    };
    trf(&i18n::STAGE_FAILED, &[&stage, &hint])
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyStatus {
    pub label: String,
    pub usage: f64,
    pub limit: Option<f64>,
    pub limit_remaining: Option<f64>,
    pub is_free_tier: bool,
    pub credits_remaining: Option<f64>,
}

#[derive(Deserialize)]
struct KeyEnvelope {
    data: KeyData,
}

#[derive(Deserialize)]
struct KeyData {
    #[serde(default)]
    label: String,
    #[serde(default)]
    usage: f64,
    limit: Option<f64>,
    limit_remaining: Option<f64>,
    #[serde(default)]
    is_free_tier: bool,
}

pub async fn check_key(settings: &AppSettings) -> AppResult<KeyStatus> {
    if settings.api_key.trim().is_empty() {
        return Err(AppError::Config(tr(&i18n::NEED_KEY).to_string()));
    }

    let base_url = settings.base_url();
    let url = if settings.is_openrouter() {
        format!("{base_url}/key")
    } else {
        format!("{base_url}/models")
    };
    let response = send_with_retry(|| authorized(client().get(&url), settings)).await?;

    if !settings.is_openrouter() {
        let _: serde_json::Value = parse_json(response, tr(&i18n::STAGE_TEST)).await?;
        return Ok(KeyStatus {
            label: tr(&i18n::CUSTOM_ENDPOINT).to_string(),
            usage: 0.0,
            limit: None,
            limit_remaining: None,
            is_free_tier: false,
            credits_remaining: None,
        });
    }

    let envelope: KeyEnvelope = parse_json(response, tr(&i18n::STAGE_TEST)).await?;
    let credits_remaining = fetch_credits(settings, &base_url).await;
    Ok(KeyStatus {
        label: envelope.data.label,
        usage: envelope.data.usage,
        limit: envelope.data.limit,
        limit_remaining: envelope.data.limit_remaining,
        is_free_tier: envelope.data.is_free_tier,
        credits_remaining,
    })
}

#[derive(Deserialize)]
struct CreditsEnvelope {
    data: CreditsData,
}

#[derive(Deserialize)]
struct CreditsData {
    total_credits: f64,
    total_usage: f64,
}

/// Account balance; best effort because some key types cannot read it.
async fn fetch_credits(settings: &AppSettings, base_url: &str) -> Option<f64> {
    let response = authorized(client().get(format!("{base_url}/credits")), settings)
        .send()
        .await
        .ok()?;
    if !response.status().is_success() {
        return None;
    }
    let envelope: CreditsEnvelope = response.json().await.ok()?;
    Some(envelope.data.total_credits - envelope.data.total_usage)
}
