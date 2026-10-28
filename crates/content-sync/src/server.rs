//! The server as the sync sees it: three calls. `HttpServer` talks to the real one; the tests use a
//! fake. Everything is raw JSON: hashes are taken over what the server sent, not over typed copies.

use std::collections::BTreeMap;
use std::future::Future;
use std::time::Duration;

use serde::Deserialize;
use serde_json::Value;

use crate::error::SyncError;

/// Where the app registers and syncs from.
pub const DEFAULT_SERVER: &str = "https://red-companion.ru";

/// What the app tells the server about itself when it registers (`POST /installations`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallInfo {
    pub app_version: String,
    /// `windows`, `macos` or `android`.
    pub os: String,
    /// Interface language, `ru` or `en`.
    pub lang: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registration {
    pub installation_id: String,
    pub token: String,
}

/// The futures are `Send` so a Tauri command can await a run.
pub trait ContentServer {
    /// Registers this installation and returns its token.
    fn register(&self, info: &InstallInfo) -> impl Future<Output = Result<Registration, SyncError>> + Send;
    /// The signed manifest, as sent.
    fn manifest(&self, token: &str, app_version: &str) -> impl Future<Output = Result<Value, SyncError>> + Send;
    /// One collection of a pack in one language: `{ name, version, entries }`.
    fn collection(&self, token: &str, pack: &str, collection: &str, lang: &str) -> impl Future<Output = Result<Value, SyncError>> + Send;
}

pub struct HttpServer {
    base: String,
    client: reqwest::Client,
}

impl HttpServer {
    pub fn new(base: &str) -> Result<Self, SyncError> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(60))
            .user_agent(concat!("RedCompanion/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| SyncError::Unreachable(e.to_string()))?;
        Ok(Self { base: base.trim_end_matches('/').to_string(), client })
    }

    fn url(&self, path: &str) -> String {
        format!("{}/api/v1/{path}", self.base)
    }

    async fn read(response: reqwest::Response) -> Result<Value, SyncError> {
        let status = response.status();
        let body = response.text().await.map_err(|e| SyncError::Unreachable(e.to_string()))?;
        if status.is_success() {
            return serde_json::from_str(&body).map_err(|e| SyncError::BadResponse(e.to_string()));
        }
        Err(server_error(status.as_u16(), &body))
    }
}

/// Turns an error response into `SyncError::Server` when its body is the server's error shape, else
/// into `BadResponse` (a proxy page, an empty body).
pub(crate) fn server_error(status: u16, body: &str) -> SyncError {
    #[derive(Deserialize)]
    struct ErrorBody {
        code: String,
        #[serde(default)]
        params: BTreeMap<String, Value>,
    }
    match serde_json::from_str::<ErrorBody>(body) {
        Ok(e) => SyncError::Server {
            code: e.code,
            status,
            params: e.params.into_iter().map(|(k, v)| (k, v.as_str().map_or_else(|| v.to_string(), str::to_string))).collect(),
        },
        Err(_) => SyncError::BadResponse(format!("HTTP {status}")),
    }
}

impl ContentServer for HttpServer {
    async fn register(&self, info: &InstallInfo) -> Result<Registration, SyncError> {
        let body = serde_json::json!({ "app_version": info.app_version, "os": info.os, "lang": info.lang });
        let response = self.client.post(self.url("installations")).json(&body).send().await.map_err(|e| SyncError::Unreachable(e.to_string()))?;
        let value = Self::read(response).await?;
        let text = |key: &str| value.get(key).and_then(Value::as_str).map(str::to_string).ok_or_else(|| SyncError::BadResponse(format!("no {key}")));
        Ok(Registration { installation_id: text("installation_id")?, token: text("token")? })
    }

    async fn manifest(&self, token: &str, app_version: &str) -> Result<Value, SyncError> {
        let response = self
            .client
            .get(self.url("manifest"))
            .bearer_auth(token)
            .header("X-App-Version", app_version)
            .send()
            .await
            .map_err(|e| SyncError::Unreachable(e.to_string()))?;
        Self::read(response).await
    }

    async fn collection(&self, token: &str, pack: &str, collection: &str, lang: &str) -> Result<Value, SyncError> {
        let response = self
            .client
            .get(self.url(&format!("packs/{pack}/{collection}")))
            .query(&[("lang", lang)])
            .bearer_auth(token)
            .send()
            .await
            .map_err(|e| SyncError::Unreachable(e.to_string()))?;
        Self::read(response).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_servers_error_body_becomes_a_coded_error() {
        let e = server_error(404, r#"{"code":"server.unknown_pack","params":{"pack":"dlc9"},"message":"Unknown pack"}"#);
        match e {
            SyncError::Server { code, status, params } => {
                assert_eq!((code.as_str(), status), ("server.unknown_pack", 404));
                assert_eq!(params["pack"], "dlc9");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn a_body_that_is_not_the_servers_shape_is_a_bad_response() {
        assert!(matches!(server_error(502, "<html>Bad gateway</html>"), SyncError::BadResponse(_)));
    }
}
