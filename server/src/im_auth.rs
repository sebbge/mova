use crate::error::{AppError, AppResult};

use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    expires_in: i64,
}

struct CachedToken {
    access_token: String,
    expires_at: DateTime<Utc>,
}

impl CachedToken {
    fn fresh(cached: &Option<Self>) -> Option<String> {
        cached
            .as_ref()
            .filter(|c| Utc::now() < c.expires_at)
            .map(|c| c.access_token.clone())
    }
}

#[derive(Clone)]
pub struct ImAuthClient {
    http: reqwest::Client,
    auth_url: String,
    client_id: String,
    client_secret: String,
    cached: Arc<RwLock<Option<CachedToken>>>,
}

impl ImAuthClient {
    pub fn new(auth_url: String, client_id: String, client_secret: String) -> Self {
        Self {
            http: reqwest::Client::new(),
            auth_url,
            client_id,
            client_secret,
            cached: Arc::new(RwLock::new(None)),
        }
    }

    pub async fn token(&self) -> AppResult<String> {
        if let Some(token) = CachedToken::fresh(&*self.cached.read().await) {
            return Ok(token);
        }

        let mut guard = self.cached.write().await;
        if let Some(token) = CachedToken::fresh(&guard) {
            return Ok(token);
        }

        let form_body = format!(
            "grant_type=client_credentials&client_id={}&client_secret={}",
            self.client_id, self.client_secret
        );

        let res: reqwest::Response = self
            .http
            .post(&self.auth_url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(form_body)
            .send()
            .await
            .map_err(|e| AppError::ExternalApi(format!("auth request failed: {e}")))?;

        if !res.status().is_success() {
            let status = res.status();
            let body: String = res.text().await.unwrap_or_default();
            return Err(AppError::ExternalApi(format!(
                "auth returned {status}: {body}"
            )));
        }

        let token_res: TokenResponse = res
            .json::<TokenResponse>()
            .await
            .map_err(|e| AppError::ExternalApi(format!("failed to parse auth response: {e}")))?;

        let expires_at = Utc::now() + chrono::Duration::seconds(token_res.expires_in - 30);

        let access_token = token_res.access_token.clone();
        *guard = Some(CachedToken {
            access_token: token_res.access_token,
            expires_at,
        });

        Ok(access_token)
    }
}
