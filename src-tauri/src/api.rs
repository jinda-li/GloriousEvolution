use std::{sync::OnceLock, time::Duration};

use reqwest::{RequestBuilder, Response, StatusCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{settings::AppSettings, AppError, AppResult};

const MAX_ATTEMPTS: usize = 2;

pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .pool_idle_timeout(Duration::from_secs(90))
            .tcp_keepalive(Duration::from_secs(30))
            .user_agent(concat!("GloriousEvolution/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("failed to build HTTP client")
    })
}

pub fn authorized(builder: RequestBuilder, settings: &AppSettings) -> RequestBuilder {
    let mut builder = builder.bearer_auth(settings.api_key.trim());
    if settings.is_openrouter() {
        builder = builder
            .header("HTTP-Referer", "https://github.com/glorious-evolution")
            .header("X-Title", "GloriousEvolution");
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
        .map_err(|error| AppError::Api(format!("{stage}返回了无法解析的数据：{error}")))
}

fn network_error(error: reqwest::Error) -> AppError {
    if error.is_timeout() {
        AppError::Api("网络请求超时，请检查网络后重试。".to_string())
    } else if error.is_connect() {
        AppError::Api("无法连接到服务器，请检查网络或代理设置。".to_string())
    } else {
        AppError::Api(format!("网络请求失败：{error}"))
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
        401 => "API Key 无效或已被删除，请在设置中重新填写。".to_string(),
        402 => "OpenRouter 余额不足。语音识别要求账户余额至少 $0.50，请前往 openrouter.ai/settings/credits 充值。".to_string(),
        403 => format!("请求被拒绝（可能触发了内容审核或 Key 权限限制）：{detail}"),
        404 => format!("找不到模型或接口，请检查模型名称：{detail}"),
        408 | 504 => "上游模型响应超时，请稍后重试或换一个更快的模型。".to_string(),
        429 => "请求过于频繁或免费额度已用完，请稍后重试。".to_string(),
        _ => format!("{} {detail}", status.as_u16()),
    };
    format!("{stage}失败：{hint}")
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
        return Err(AppError::Config("请先填写 OpenRouter API Key。".to_string()));
    }

    let base_url = settings.base_url();
    let url = if settings.is_openrouter() {
        format!("{base_url}/key")
    } else {
        format!("{base_url}/models")
    };
    let response = send_with_retry(|| authorized(client().get(&url), settings)).await?;

    if !settings.is_openrouter() {
        let _: serde_json::Value = parse_json(response, "连接测试").await?;
        return Ok(KeyStatus {
            label: "自定义接口".to_string(),
            usage: 0.0,
            limit: None,
            limit_remaining: None,
            is_free_tier: false,
            credits_remaining: None,
        });
    }

    let envelope: KeyEnvelope = parse_json(response, "连接测试").await?;
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
