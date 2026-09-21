// https://api.montevideo.gub.uy/apidocs/publictransport
use chrono::{DateTime, Utc};
use serde::Deserialize;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Deserialize)]
struct TokenRes {
  access_token: String,
  expires_in: i64, // always 300
}

struct SavedToken {
  access_token: String,
  expires_at: DateTime<Utc>,
}

//# GET <MVD>/auth/realms/pci/protocol/openid-connect/token
#[derive(Clone)]
pub struct ImAuthClient {
  auth_url: String,
  client_id: String,
  client_secret: String,
  saved: Arc<RwLock<Option<SavedToken>>>,
}

impl ImAuthClient {
  pub fn new(auth_url: String, client_id: String, client_secret: String) -> Self {
    Self { auth_url, client_id, client_secret, saved: Arc::new(RwLock::new(None)) }
  }

  // auth
  pub(super) async fn token(&self, http: &reqwest::Client) -> crate::core::Result<String> {
    {
      let guard = self.saved.read().await;
      if let Some(cached) = guard.as_ref()
        && Utc::now() < cached.expires_at
      {
        return Ok(cached.access_token.clone());
      }
    }

    let mut guard = self.saved.write().await;

    if let Some(cached) = guard.as_ref()
      && Utc::now() < cached.expires_at
    {
      return Ok(cached.access_token.clone());
    }

    let form = [
      ("grant_type", "client_credentials"),
      ("client_id", self.client_id.as_str()),
      ("client_secret", self.client_secret.as_str()),
    ];

    let res = http
      .post(&self.auth_url)
      .form(&form)
      .send()
      .await
      .map_err(|e| crate::core::Error::External(format!("auth request failed: {e}")))?;

    if !res.status().is_success() {
      let status = res.status();
      let body = res.text().await.unwrap_or_default();
      return Err(crate::core::Error::External(format!("auth returned {status}: {body}")));
    }

    let token_res: TokenRes =
      res.json().await.map_err(|e| crate::core::Error::External(format!("failed to parse auth response: {e}")))?;

    let expires_at = Utc::now() + chrono::Duration::seconds(token_res.expires_in - 30);

    let access_token = token_res.access_token.clone();
    *guard = Some(SavedToken { access_token: token_res.access_token, expires_at });

    Ok(access_token)
  }
}
