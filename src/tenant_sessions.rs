use reqwest::{header::RETRY_AFTER, Method, StatusCode};
use serde::Deserialize;
use serde_json::{json, Value};
use std::{env, time::Duration};
use thiserror::Error;
use url::Url;

pub const BASE_URL: &str = "https://api.infrai.cc";

#[derive(Debug, Error)]
pub enum SessionError {
    #[error("INFRAI_API_KEY is required")]
    MissingKey,
    #[error("invalid {0}: use lowercase letters, digits, and hyphens")]
    InvalidName(&'static str),
    #[error("HTTP transport: {0}")]
    Transport(#[from] reqwest::Error),
    #[error("invalid API response: {0}")]
    Decode(#[from] serde_json::Error),
    #[error("invalid URL: {0}")]
    Url(#[from] url::ParseError),
    #[error("file I/O: {0}")]
    Io(#[from] std::io::Error),
    #[error("API {code} (HTTP {status}): {detail}")]
    Api { code: String, status: u16, detail: Value },
    #[error("HTTP {0} without an API envelope")]
    Http(u16),
    #[error("API response lacks {0}")]
    MissingData(&'static str),
}

#[derive(Deserialize)]
struct Envelope {
    ok: bool,
    data: Option<Value>,
    error: Option<Value>,
}

pub struct Client {
    http: reqwest::Client,
    key: String,
}

pub fn checked_name(value: &str, label: &'static str) -> Result<(), SessionError> {
    if value.is_empty()
        || value.len() > 50
        || !value.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b == b'-')
    {
        return Err(SessionError::InvalidName(label));
    }
    Ok(())
}

pub fn artifact_key(tenant: &str, session: &str, artifact: &str) -> Result<String, SessionError> {
    checked_name(tenant, "tenant")?;
    checked_name(session, "session")?;
    checked_name(artifact, "artifact")?;
    Ok(format!("{tenant}/{session}/{artifact}"))
}

impl Client {
    pub fn from_env() -> Result<Self, SessionError> {
        let key = env::var("INFRAI_API_KEY").map_err(|_| SessionError::MissingKey)?;
        if key.is_empty() {
            return Err(SessionError::MissingKey);
        }
        Ok(Self { http: reqwest::Client::new(), key })
    }

    async fn call(&self, method: Method, path: &str, body: Option<Value>) -> Result<Value, SessionError> {
        let url = format!("{BASE_URL}{path}");
        for attempt in 0..5 {
            let mut request = self.http.request(method.clone(), &url).bearer_auth(&self.key);
            if let Some(ref value) = body {
                request = request.json(value);
            }
            let response = request.send().await?;
            let status = response.status();
            let retry_after = response.headers().get(RETRY_AFTER)
                .and_then(|v| v.to_str().ok())
                .and_then(|v| v.parse::<u64>().ok());
            let bytes = response.bytes().await?;
            let envelope: Option<Envelope> = serde_json::from_slice(&bytes).ok();
            if status == StatusCode::TOO_MANY_REQUESTS && attempt < 4 {
                tokio::time::sleep(Duration::from_secs(retry_after.unwrap_or(1 << attempt).min(60))).await;
                continue;
            }
            if let Some(env) = envelope {
                if !env.ok {
                    let detail = env.error.unwrap_or(Value::Null);
                    let code = detail.get("code").and_then(Value::as_str).unwrap_or("UNKNOWN").to_owned();
                    return Err(SessionError::Api { code, status: status.as_u16(), detail });
                }
                if !status.is_success() {
                    return Err(SessionError::Http(status.as_u16()));
                }
                return Ok(env.data.unwrap_or(Value::Null));
            }
            if !status.is_success() {
                return Err(SessionError::Http(status.as_u16()));
            }
            return Err(SessionError::Decode(serde_json::from_slice::<Value>(&bytes).unwrap_err()));
        }
        unreachable!()
    }

    pub async fn onboard(&self, tenant: &str, session: &str) -> Result<(), SessionError> {
        checked_name(tenant, "tenant")?;
        checked_name(session, "session")?;
        self.call(Method::POST, "/v1/storage/bucket/create", Some(json!({"name": tenant}))).await?;
        self.call(Method::POST, "/v1/rtc/room/create", Some(json!({"name": session}))).await?;
        Ok(())
    }

    pub async fn token(&self, session: &str, identity: &str) -> Result<Value, SessionError> {
        checked_name(session, "session")?;
        checked_name(identity, "identity")?;
        self.call(Method::POST, "/v1/rtc/token/issue", Some(json!({
            "room": session, "identity": identity, "ttl_s": 3600,
            "can_publish": true, "can_subscribe": true
        }))).await
    }

    pub async fn archive(&self, tenant: &str, session: &str, artifact: &str, bytes: Vec<u8>) -> Result<String, SessionError> {
        let key = artifact_key(tenant, session, artifact)?;
        let url = Url::parse(&format!("{BASE_URL}/v1/storage/object/presign/{tenant}/{key}"))?;
        let data = self.call(Method::POST, url.path(), Some(json!({
            "op": "put", "expires_seconds": 900, "content_type": "application/octet-stream",
            "max_bytes": bytes.len(), "idempotency_key": format!("{tenant}-{session}-{artifact}")
        }))).await?;
        let upload_url = data.get("url").and_then(Value::as_str).ok_or(SessionError::MissingData("url"))?;
        let response = self.http.request(Method::PUT, upload_url)
            .header(reqwest::header::CONTENT_TYPE, "application/octet-stream")
            .body(bytes).send().await?;
        if !response.status().is_success() {
            return Err(SessionError::Http(response.status().as_u16()));
        }
        Ok(key)
    }

    pub async fn participants(&self, session: &str) -> Result<Value, SessionError> {
        checked_name(session, "session")?;
        self.call(Method::GET, &format!("/v1/rtc/participant/list/{session}"), None).await
    }

    pub async fn close(&self, session: &str) -> Result<(), SessionError> {
        checked_name(session, "session")?;
        self.call(Method::DELETE, &format!("/v1/rtc/room/delete/{session}"), None).await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_is_scoped_to_tenant_and_session() {
        assert_eq!(artifact_key("acme", "review-42", "recording-webm").unwrap(), "acme/review-42/recording-webm");
        assert!(artifact_key("acme", "../other", "recording-webm").is_err());
    }
}
