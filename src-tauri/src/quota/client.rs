//! HTTP client for the Antigravity language server (Connect protocol).
//!
//! Only loopback is contacted, only the three quota endpoints are called, and
//! the CSRF token is sent in the header the service expects. Responses are size
//! limited and parsed into the raw protobuf-JSON types.

use std::time::Duration;

use serde::de::DeserializeOwned;

use super::discovery::Connection;
use super::model::{ModelsResponse, QuotaSummaryResponse, UserStatusResponse};

const SERVICE_PREFIX: &str = "/exa.language_server_pb.LanguageServerService/";
const MAX_BODY_BYTES: usize = 1_048_576;

#[derive(Debug, Clone, PartialEq)]
pub enum RpcError {
    /// The service rejected the session token (expired/restarted server).
    Unauthorized,
    /// The endpoint does not exist in this Antigravity build.
    Unsupported,
    Timeout,
    Unreachable,
    Malformed,
    Http(u16),
}

pub struct QuotaClient {
    http: reqwest::Client,
}

impl QuotaClient {
    pub fn new() -> Self {
        let http = reqwest::Client::builder()
            // Loopback only: never route the local session token through a proxy.
            .no_proxy()
            .timeout(Duration::from_secs(8))
            .connect_timeout(Duration::from_millis(1500))
            .user_agent(concat!("Quoty/", env!("CARGO_PKG_VERSION")))
            .build()
            .expect("static HTTP client configuration is valid");

        Self { http }
    }

    pub async fn rpc<T: DeserializeOwned>(
        &self,
        connection: &Connection,
        method: &'static str,
    ) -> Result<T, RpcError> {
        let url = format!(
            "http://127.0.0.1:{}{SERVICE_PREFIX}{method}",
            connection.port
        );

        let mut response = self
            .http
            .post(url)
            .header("content-type", "application/json")
            .header("connect-protocol-version", "1")
            .header("x-codeium-csrf-token", connection.csrf.as_str())
            .body("{}")
            .send()
            .await
            .map_err(|error| {
                if error.is_timeout() {
                    RpcError::Timeout
                } else if error.is_connect() {
                    RpcError::Unreachable
                } else {
                    RpcError::Unreachable
                }
            })?;

        let status = response.status().as_u16();
        if !response.status().is_success() {
            return Err(match status {
                401 | 403 => RpcError::Unauthorized,
                404 | 405 | 501 => RpcError::Unsupported,
                other => RpcError::Http(other),
            });
        }

        if response
            .content_length()
            .is_some_and(|length| length > MAX_BODY_BYTES as u64)
        {
            return Err(RpcError::Malformed);
        }

        let mut body: Vec<u8> = Vec::new();
        while let Some(chunk) = response.chunk().await.map_err(|error| {
            if error.is_timeout() {
                RpcError::Timeout
            } else {
                RpcError::Unreachable
            }
        })? {
            if body.len() + chunk.len() > MAX_BODY_BYTES {
                return Err(RpcError::Malformed);
            }
            body.extend_from_slice(&chunk);
        }

        serde_json::from_slice(&body).map_err(|_| RpcError::Malformed)
    }

    /// Confirms that a candidate port/token pair really is the quota service.
    pub async fn probe(&self, connection: &Connection) -> Result<UserStatusResponse, RpcError> {
        self.rpc::<UserStatusResponse>(connection, "GetUserStatus")
            .await
    }

    pub async fn fetch(
        &self,
        connection: &Connection,
    ) -> Result<Fetched, RpcError> {
        let status = self
            .rpc::<UserStatusResponse>(connection, "GetUserStatus")
            .await?;
        let summary = self
            .rpc::<QuotaSummaryResponse>(connection, "RetrieveUserQuotaSummary")
            .await?;
        // The catalog is optional: group quotas remain useful without it.
        let models = self
            .rpc::<ModelsResponse>(connection, "GetAvailableModels")
            .await
            .ok();

        Ok(Fetched {
            status,
            summary,
            models,
        })
    }

}

pub struct Fetched {
    pub status: UserStatusResponse,
    pub summary: QuotaSummaryResponse,
    pub models: Option<ModelsResponse>,
}
