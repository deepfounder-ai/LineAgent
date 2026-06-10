//! HTTP client used by the CLI to talk to a running `lineagent serve`.
//!
//! All subcommands go through [`Client::request`] so that auth header
//! injection, JSON error parsing, and exit-code mapping happen in one
//! place. The client is cheap to clone (an `Arc<reqwest::Client>`) and
//! can be created per command.
//!
//! ## Auth
//!
//! The bearer token is attached as `Authorization: Bearer lineagent_…`.
//! If [`Client::api_key`] is `None` at request time, the client refuses
//! to make the call and returns `CliError::Usage("api key required")`.
//!
//! ## Error parsing
//!
//! The server's error envelope is `{ "code": "...", "message": "..." }`.
//! Non-2xx responses are turned into `CliError::Http`.

use std::time::Duration;

use reqwest::header::AUTHORIZATION;
use reqwest::{Client as HttpClient, Method, RequestBuilder, Response, StatusCode};
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::cli::config::CliConfig;
use crate::cli::output::{CliError, CliResult};

/// CLI-side HTTP client.
#[derive(Debug, Clone)]
pub struct Client {
    base_url: String,
    api_key: Option<String>,
    http: HttpClient,
}

impl Client {
    /// Build a client from a resolved [`CliConfig`].
    pub fn new(config: &CliConfig) -> CliResult<Self> {
        let http = HttpClient::builder()
            .timeout(Duration::from_secs(30))
            .user_agent(concat!("lineagent-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| CliError::Network(e.to_string()))?;
        Ok(Self {
            base_url: config.api_url.clone(),
            api_key: config.api_key.clone(),
            http,
        })
    }

    /// Replace the API key.
    pub fn with_api_key(mut self, key: String) -> Self {
        self.api_key = Some(key);
        self
    }

    /// The base URL the client is targeting.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// The currently configured API key, if any.
    pub fn api_key(&self) -> Option<&str> {
        self.api_key.as_deref()
    }

    /// Issue a typed request.
    pub async fn request<T, B>(&self, method: Method, path: &str, body: Option<&B>) -> CliResult<T>
    where
        T: DeserializeOwned,
        B: Serialize,
    {
        let url = self.url_for(path);
        let mut req = self.http.request(method.clone(), &url);
        req = self.apply_auth(req)?;
        if let Some(b) = body {
            req = req.json(b);
        }
        let resp = req.send().await?;
        self.handle(resp).await
    }

    /// Issue a request without a request body and returning a raw
    /// `serde_json::Value`.
    pub async fn request_value(&self, method: Method, path: &str) -> CliResult<Value> {
        self.request(method, path, None::<&()>).await
    }

    /// Send a raw `reqwest::RequestBuilder`.
    pub async fn send_raw(&self, builder: RequestBuilder) -> CliResult<Response> {
        let builder = self.apply_auth(builder)?;
        let resp = builder.send().await?;
        if !resp.status().is_success() {
            return Err(self.parse_error(resp).await);
        }
        Ok(resp)
    }

    /// Convenience: `GET <path>` and return the JSON body.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> CliResult<T> {
        self.request(Method::GET, path, None::<&()>).await
    }

    /// Convenience: `POST <path>` with a JSON body.
    pub async fn post<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> CliResult<T> {
        self.request(Method::POST, path, Some(body)).await
    }

    /// `POST <path>` with a JSON body and **no** auth header.
    pub async fn post_noauth<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> CliResult<T> {
        let url = self.url_for(path);
        let resp = self.http.post(&url).json(body).send().await?;
        self.handle(resp).await
    }

    /// Convenience: `PUT <path>` with a JSON body.
    pub async fn put<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> CliResult<T> {
        self.request(Method::PUT, path, Some(body)).await
    }

    /// Convenience: `PATCH <path>` with a JSON body.
    pub async fn patch<T: DeserializeOwned, B: Serialize>(
        &self,
        path: &str,
        body: &B,
    ) -> CliResult<T> {
        self.request(Method::PATCH, path, Some(body)).await
    }

    /// Convenience: `DELETE <path>` returning the JSON body.
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> CliResult<T> {
        self.request(Method::DELETE, path, None::<&()>).await
    }

    /// `POST <path>` as `multipart/form-data`.
    pub async fn post_multipart<T: DeserializeOwned>(
        &self,
        path: &str,
        form: reqwest::multipart::Form,
    ) -> CliResult<T> {
        let url = self.url_for(path);
        let req = self.http.post(&url).multipart(form);
        let req = self.apply_auth(req)?;
        let resp = req.send().await?;
        self.handle(resp).await
    }

    /// `GET <path>` returning the raw text body.
    pub async fn get_text(&self, path: &str) -> CliResult<String> {
        let url = self.url_for(path);
        let req = self.http.get(&url);
        let req = self.apply_auth(req)?;
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(self.parse_error(resp).await);
        }
        Ok(resp.text().await?)
    }

    /// `GET <path>` returning the raw bytes.
    pub async fn get_bytes(&self, path: &str) -> CliResult<Vec<u8>> {
        let url = self.url_for(path);
        let req = self.http.get(&url);
        let req = self.apply_auth(req)?;
        let resp = req.send().await?;
        if !resp.status().is_success() {
            return Err(self.parse_error(resp).await);
        }
        Ok(resp.bytes().await?.to_vec())
    }

    fn apply_auth(&self, req: RequestBuilder) -> CliResult<RequestBuilder> {
        if let Some(key) = &self.api_key {
            Ok(req.header(AUTHORIZATION, format!("Bearer {key}")))
        } else {
            Err(CliError::Usage(
                "api key is required (set LINEAGENT_API_KEY or run `lineagent user login`)".into(),
            ))
        }
    }

    fn url_for(&self, path: &str) -> String {
        let base = self.base_url.trim_end_matches('/');
        if path.starts_with('/') {
            format!("{base}{path}")
        } else {
            format!("{base}/{path}")
        }
    }

    async fn handle<T: DeserializeOwned>(&self, resp: Response) -> CliResult<T> {
        if !resp.status().is_success() {
            return Err(self.parse_error(resp).await);
        }
        if resp.status() == StatusCode::NO_CONTENT {
            return serde_json::from_str("null").map_err(CliError::from);
        }
        let bytes = resp.bytes().await?;
        if bytes.is_empty() {
            return serde_json::from_str("null").map_err(CliError::from);
        }
        serde_json::from_slice(&bytes).map_err(CliError::from)
    }

    async fn parse_error(&self, resp: Response) -> CliError {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        if let Ok(v) = serde_json::from_str::<Value>(&body) {
            let obj = v.get("error").unwrap_or(&v);
            let code = obj
                .get("code")
                .and_then(|c| c.as_str())
                .unwrap_or("unknown")
                .to_string();
            let message = obj
                .get("message")
                .and_then(|c| c.as_str())
                .unwrap_or("(no message)")
                .to_string();
            CliError::Http {
                status,
                code,
                message,
            }
        } else if body.is_empty() {
            CliError::Http {
                status,
                code: "unknown".into(),
                message: format!("HTTP {status} with empty body"),
            }
        } else {
            CliError::Http {
                status,
                code: "unknown".into(),
                message: body.chars().take(200).collect(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg(url: &str, key: Option<&str>) -> CliConfig {
        CliConfig {
            api_url: url.to_string(),
            api_key: key.map(|s| s.to_string()),
            credentials_path: std::path::PathBuf::from("/tmp/none"),
            config_path: None,
        }
    }

    #[test]
    fn url_for_joins_correctly() {
        let c = Client::new(&cfg("http://h:8080", None)).unwrap();
        assert_eq!(c.url_for("/api/v1/auth/whoami"), "http://h:8080/api/v1/auth/whoami");
        assert_eq!(c.url_for("api/v1/auth/whoami"), "http://h:8080/api/v1/auth/whoami");
        let c2 = Client::new(&cfg("http://h:8080/", None)).unwrap();
        assert_eq!(c2.url_for("/api/v1/auth/whoami"), "http://h:8080/api/v1/auth/whoami");
    }

    #[test]
    fn auth_required_for_get() {
        let c = Client::new(&cfg("http://h:8080", None)).unwrap();
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let err = rt.block_on(async { c.get::<serde_json::Value>("/api/v1/auth/whoami").await });
        assert!(matches!(err, Err(CliError::Usage(_))));
    }
}
