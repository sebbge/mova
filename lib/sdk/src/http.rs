use crate::{Error, Result};

use reqwest::{RequestBuilder, Response, StatusCode};
use std::time::Duration;

pub fn client(timeout: Duration) -> Result<reqwest::Client> {
  let _ = rustls::crypto::ring::default_provider().install_default();
  let builder = reqwest::Client::builder().tls_backend_rustls().timeout(timeout);
  builder.build().map_err(|e| Error::Config(format!("http client: {e}")))
}

pub fn legacy_tls_client(timeout: Duration) -> Result<reqwest::Client> {
  let builder = reqwest::Client::builder().tls_backend_native().timeout(timeout);
  builder.build().map_err(|e| Error::Config(format!("http client: {e}")))
}

pub async fn send(req: RequestBuilder, tag: &str) -> Result<Response> {
  req.send().await.map_err(|e| {
    let url = e.url().map(|u| u.as_str()).unwrap_or("unknown");
    tracing::warn!(url = %url, error = %e, "{tag} request failed");
    Error::from(e)
  })
}

pub async fn ok(res: Response, tag: &str) -> Result<Response> {
  let status = res.status();
  if status.is_success() {
    return Ok(res);
  }

  let url = res.url().to_string();
  if status == StatusCode::TOO_MANY_REQUESTS {
    tracing::warn!(url = %url, "{tag} rate limited (429)");
    return Err(Error::RateLimited);
  }

  let body: String = res.text().await.unwrap_or_default().chars().take(300).collect();
  tracing::warn!(status = status.as_u16(), url = %url, body = %body, "{tag} non-2xx");
  Err(Error::External(format!("{status} at {url}")))
}

pub fn retry_after(res: &Response) -> Option<Duration> {
  let secs = res.headers().get(reqwest::header::RETRY_AFTER)?.to_str().ok()?.trim().parse().ok()?;
  Some(Duration::from_secs(secs))
}

pub async fn fetch(req: RequestBuilder, tag: &str) -> Result<Response> {
  ok(send(req, tag).await?, tag).await
}

pub async fn bytes(res: Response, tag: &str) -> Result<Vec<u8>> {
  res.bytes().await.map(|b| b.to_vec()).map_err(|e| {
    tracing::warn!(error = %e, "{tag} body read failed");
    Error::from(e)
  })
}

pub async fn json<T: serde::de::DeserializeOwned>(res: Response, tag: &str) -> Result<T> {
  let url = res.url().to_string();
  let bytes = bytes(res, tag).await?;
  serde_json::from_slice(&bytes).map_err(|e| {
    let snippet = String::from_utf8_lossy(&bytes[..bytes.len().min(300)]);
    tracing::error!(url = %url, error = %e, body = %snippet, "{tag} JSON decode failed");
    Error::External(format!("JSON decode at {url}: {e}"))
  })
}
