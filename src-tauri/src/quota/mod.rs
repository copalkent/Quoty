//! Quota adapter.
//!
//! The UI never talks to Antigravity directly. Everything goes through this
//! module, which can be replaced if the data source changes.

pub mod client;
pub mod discovery;
pub mod model;

use std::sync::Mutex;

use time::OffsetDateTime;

use client::RpcError;
use model::{Problem, Snapshot};

pub struct QuotaService {
    client: client::QuotaClient,
    connection: Mutex<Option<discovery::Connection>>,
}

impl Default for QuotaService {
    fn default() -> Self {
        Self::new()
    }
}

impl QuotaService {
    pub fn new() -> Self {
        Self {
            client: client::QuotaClient::new(),
            connection: Mutex::new(None),
        }
    }

    /// Returns either a live snapshot or a snapshot describing exactly why the
    /// data is unavailable. Never returns invented numbers.
    pub async fn snapshot(&self) -> Snapshot {
        match self.fetch().await {
            Ok(snapshot) => snapshot,
            Err(problem) => Snapshot::unavailable(OffsetDateTime::now_utc(), problem),
        }
    }

    async fn fetch(&self) -> Result<Snapshot, Problem> {
        let mut last_error: Option<RpcError> = None;

        for attempt in 0..2 {
            let connection = self.connect(attempt > 0).await?;

            match self.client.fetch(&connection).await {
                Ok(fetched) => {
                    return Ok(model::build_snapshot(
                        OffsetDateTime::now_utc(),
                        fetched.summary,
                        fetched.models,
                        Some(fetched.status),
                        connection.source_label(),
                    ));
                }
                Err(error) => {
                    self.invalidate();
                    let retryable = matches!(
                        error,
                        RpcError::Unauthorized | RpcError::Unreachable | RpcError::Timeout
                    );
                    if !retryable || attempt == 1 {
                        return Err(problem_for(error));
                    }
                    last_error = Some(error);
                }
            }
        }

        Err(problem_for(last_error.unwrap_or(RpcError::Unreachable)))
    }

    async fn connect(&self, rediscover: bool) -> Result<discovery::Connection, Problem> {
        if !rediscover {
            if let Some(cached) = self.connection.lock().ok().and_then(|guard| guard.clone()) {
                return Ok(cached);
            }
        }

        let candidates = tauri::async_runtime::spawn_blocking(discovery::candidates)
            .await
            .unwrap_or_default();

        if candidates.is_empty() {
            return Err(not_running());
        }

        for candidate in candidates {
            let connection = discovery::Connection {
                port: 0,
                csrf: candidate.csrf,
                product: candidate.product,
            };

            for port in candidate.ports {
                let probe = discovery::Connection {
                    port,
                    ..connection.clone()
                };
                if self.client.probe(&probe).await.is_ok() {
                    if let Ok(mut guard) = self.connection.lock() {
                        *guard = Some(probe.clone());
                    }
                    return Ok(probe);
                }
            }
        }

        Err(unreachable())
    }

    fn invalidate(&self) {
        if let Ok(mut guard) = self.connection.lock() {
            *guard = None;
        }
    }
}

fn not_running() -> Problem {
    Problem {
        code: "antigravity_not_running".into(),
        title: "Antigravity isn't running".into(),
        detail: "Open Antigravity IDE and sign in. Quoty reads quotas from the local language \
                 server Antigravity starts; it never connects to the internet on its own."
            .into(),
    }
}

fn unreachable() -> Problem {
    Problem {
        code: "service_unreachable".into(),
        title: "Language server not responding".into(),
        detail: "Antigravity is open, but its local quota service didn't answer. It may still be \
                 starting — refresh in a moment, or restart Antigravity."
            .into(),
    }
}

fn problem_for(error: RpcError) -> Problem {
    match error {
        RpcError::Unauthorized => Problem {
            code: "session_rejected".into(),
            title: "Session not accepted".into(),
            detail: "Antigravity's local service rejected the widget's session token. Restart \
                     Antigravity so it issues a new one. Quoty keeps the token in memory only."
                .into(),
        },
        RpcError::Unsupported => Problem {
            code: "unsupported_version".into(),
            title: "Quota API unavailable".into(),
            detail: "This Antigravity build does not expose the quota endpoints Quoty reads. \
                     Update Antigravity and try again."
                .into(),
        },
        RpcError::Timeout => Problem {
            code: "timeout".into(),
            title: "Local service timed out".into(),
            detail: "Antigravity's quota service took too long to respond. Try refreshing."
                .into(),
        },
        RpcError::Malformed => Problem {
            code: "malformed_response".into(),
            title: "Unexpected response".into(),
            detail: "The local quota service returned data Quoty couldn't read. The Antigravity \
                     version may have changed its response format."
                .into(),
        },
        RpcError::Unreachable => unreachable(),
        RpcError::Http(status) => Problem {
            code: "http_error".into(),
            title: "Local service error".into(),
            detail: format!("The quota service responded with HTTP {status}. Try refreshing."),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unavailable_snapshot_has_no_numbers() {
        let snapshot = Snapshot::unavailable(OffsetDateTime::now_utc(), not_running());
        assert_eq!(snapshot.status, "unavailable");
        assert!(snapshot.groups.is_empty());
        assert!(snapshot.models.is_empty());
        assert_eq!(snapshot.error.unwrap().code, "antigravity_not_running");
    }

    #[test]
    fn maps_errors_to_readable_problems() {
        assert_eq!(problem_for(RpcError::Unauthorized).code, "session_rejected");
        assert_eq!(problem_for(RpcError::Unsupported).code, "unsupported_version");
        assert_eq!(problem_for(RpcError::Malformed).code, "malformed_response");
        assert_eq!(problem_for(RpcError::Http(502)).code, "http_error");
        assert!(!problem_for(RpcError::Http(502)).detail.contains("token"));
    }

    /// Manual verification against a live install:
    /// `cargo test --lib -- --ignored --nocapture live_snapshot`
    #[test]
    #[ignore = "requires a running Antigravity instance"]
    fn live_snapshot() {
        let service = QuotaService::new();
        let snapshot = tauri::async_runtime::block_on(service.snapshot());

        println!("status: {}", snapshot.status);
        println!("source: {:?}", snapshot.source);
        println!("plan: {:?}", snapshot.account.as_ref().and_then(|a| a.plan.clone()));
        println!("groups: {}", snapshot.groups.len());
        for group in &snapshot.groups {
            println!("  {} ({})", group.name, group.id);
            for bucket in &group.buckets {
                println!(
                    "    {:?} window={:?} remaining={:?} reset={:?}",
                    bucket.label, bucket.window, bucket.remaining_percent, bucket.reset_at
                );
            }
        }
        println!("models: {}", snapshot.models.len());
        for model in snapshot.models.iter().take(6) {
            println!(
                "  {} [{}] pool={:?} limit={:?} shared={} remaining={:?}",
                model.name, model.provider, model.pool_id, model.limit_id, model.shared,
                model.remaining_percent
            );
        }
        if let Some(error) = &snapshot.error {
            println!("error: {} — {}", error.code, error.detail);
        }

        assert_eq!(snapshot.status, "ok", "live quota read failed");
        assert!(!snapshot.groups.is_empty());
    }
}
