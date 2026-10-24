//! Normalized quota data model.
//!
//! The language server speaks protobuf-JSON (Connect). This module turns those
//! raw responses into a small, stable shape for the UI. It never invents values:
//! anything the service does not report stays `None`.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use time::format_description::well_known::Rfc3339;
use time::OffsetDateTime;

// ---------------------------------------------------------------------------
// Raw response shapes (protobuf-JSON, camelCase)
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QuotaSummaryResponse {
    pub response: Option<QuotaSummary>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct QuotaSummary {
    pub groups: Vec<RawGroup>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RawGroup {
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub buckets: Vec<RawBucket>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RawBucket {
    pub bucket_id: Option<String>,
    pub display_name: Option<String>,
    pub description: Option<String>,
    pub window: Option<String>,
    pub remaining_fraction: Option<f64>,
    pub reset_time: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelsResponse {
    pub response: Option<ModelCatalog>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ModelCatalog {
    pub models: BTreeMap<String, RawModel>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RawModel {
    pub display_name: Option<String>,
    pub model_provider: Option<String>,
    pub api_provider: Option<String>,
    pub quota_info: Option<RawQuotaInfo>,
    pub is_internal: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RawQuotaInfo {
    pub remaining_fraction: Option<f64>,
    pub reset_time: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UserStatusResponse {
    pub user_status: Option<RawUserStatus>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RawUserStatus {
    pub plan_status: Option<RawPlanStatus>,
    pub user_tier: Option<RawUserTier>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RawPlanStatus {
    pub plan_info: Option<RawPlanInfo>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RawPlanInfo {
    pub plan_name: Option<String>,
    pub teams_tier: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct RawUserTier {
    pub name: Option<String>,
}

// ---------------------------------------------------------------------------
// Normalized shape sent to the webview
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    /// "ok" or "unavailable".
    pub status: &'static str,
    pub fetched_at: String,
    pub fetched_at_ms: i64,
    pub account: Option<Account>,
    pub groups: Vec<Group>,
    pub models: Vec<Model>,
    pub source: Option<String>,
    pub error: Option<Problem>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub plan: Option<String>,
    pub tier: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Group {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub buckets: Vec<Bucket>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Bucket {
    pub id: String,
    pub label: String,
    pub window: Option<String>,
    pub description: Option<String>,
    /// Percent remaining, one decimal. `None` when the service omits the value.
    pub remaining_percent: Option<f64>,
    pub reset_at: Option<String>,
    pub reset_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: String,
    pub provider: String,
    /// Group whose shared quota applies to this model, when known.
    pub pool_id: Option<String>,
    pub pool_name: Option<String>,
    /// Bucket id that currently gates the model (shared) or "model" for a
    /// model-specific reported limit.
    pub limit_id: Option<String>,
    pub limit_label: Option<String>,
    pub shared: bool,
    pub remaining_percent: Option<f64>,
    pub reset_at: Option<String>,
    pub reset_at_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub code: String,
    pub title: String,
    pub detail: String,
}

impl Snapshot {
    pub fn unavailable(now: OffsetDateTime, problem: Problem) -> Self {
        Self {
            status: "unavailable",
            fetched_at: format_time(now),
            fetched_at_ms: to_millis(now),
            account: None,
            groups: Vec::new(),
            models: Vec::new(),
            source: None,
            error: Some(problem),
        }
    }
}

// ---------------------------------------------------------------------------
// Mapping
// ---------------------------------------------------------------------------

fn clamp_percent(fraction: Option<f64>) -> Option<f64> {
    let value = fraction?;
    if !value.is_finite() {
        return None;
    }
    let percent = (value.clamp(0.0, 1.0) * 1000.0).round() / 10.0;
    Some(percent)
}

fn to_millis(dt: OffsetDateTime) -> i64 {
    (dt.unix_timestamp_nanos() / 1_000_000) as i64
}

fn format_time(dt: OffsetDateTime) -> String {
    dt.format(&Rfc3339).unwrap_or_default()
}

/// Parses an RFC 3339 timestamp reported by the language server.
/// Returns the normalized string and epoch milliseconds.
fn parse_reset(value: &str) -> Option<(String, i64)> {
    let parsed = OffsetDateTime::parse(value.trim(), &Rfc3339).ok()?;
    Some((format_time(parsed), to_millis(parsed)))
}

fn group_id_for(bucket_id: &str, index: usize) -> String {
    let prefix = bucket_id
        .split('-')
        .next()
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    match prefix.as_str() {
        "" => format!("group-{index}"),
        "3p" => "third-party".to_string(),
        other => other.to_string(),
    }
}

fn non_empty(value: Option<&String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

/// Maps a raw model's provider key to a human label.
fn provider_label(raw: &RawModel) -> String {
    let key = raw.model_provider.as_deref().unwrap_or_default();
    if key.contains("ANTHROPIC") || raw.api_provider.as_deref().is_some_and(|p| p.contains("ANTHROPIC")) {
        "Anthropic".into()
    } else if key.contains("OPENAI") || raw.api_provider.as_deref().is_some_and(|p| p.contains("OPENAI")) {
        "OpenAI".into()
    } else if key.contains("GOOGLE") || raw.api_provider.as_deref().is_some_and(|p| p.contains("GOOGLE")) {
        "Google".into()
    } else {
        "Other".into()
    }
}

fn provider_rank(provider: &str) -> u8 {
    match provider {
        "Google" => 0,
        "Anthropic" => 1,
        "OpenAI" => 2,
        _ => 3,
    }
}

fn group_sort_key(id: &str) -> u8 {
    match id {
        "gemini" => 0,
        "third-party" => 1,
        _ => 2,
    }
}

/// The bucket currently gating a pool: lowest remaining percentage, with the
/// weekly window preferred on ties. Falls back to the weekly bucket when no
/// percentage was reported.
fn tightest_bucket(group: &Group) -> Option<&Bucket> {
    let mut best: Option<&Bucket> = None;
    for bucket in &group.buckets {
        let Some(percent) = bucket.remaining_percent else {
            continue;
        };
        let better = match best {
            None => true,
            Some(current) => match current.remaining_percent {
                None => true,
                Some(current_percent) => {
                    percent < current_percent
                        || (percent == current_percent
                            && bucket.window.as_deref() == Some("weekly")
                            && current.window.as_deref() != Some("weekly"))
                }
            },
        };
        if better {
            best = Some(bucket);
        }
    }

    best.or_else(|| {
        group
            .buckets
            .iter()
            .find(|b| b.window.as_deref() == Some("weekly"))
    })
    .or_else(|| group.buckets.first())
}

/// Builds the display snapshot from the three language-server responses.
///
/// `summary` is required: it is the only endpoint that reports authoritative
/// remaining quota and reset times.
pub fn build_snapshot(
    now: OffsetDateTime,
    summary: QuotaSummaryResponse,
    models: Option<ModelsResponse>,
    status: Option<UserStatusResponse>,
    source: String,
) -> Snapshot {
    let raw_groups = summary.response.map(|r| r.groups).unwrap_or_default();

    let mut groups: Vec<Group> = Vec::new();
    for (index, raw) in raw_groups.into_iter().enumerate() {
        let mut buckets: Vec<Bucket> = Vec::new();
        for raw_bucket in raw.buckets {
            let id = raw_bucket
                .bucket_id
                .clone()
                .filter(|v| !v.trim().is_empty())
                .unwrap_or_else(|| format!("bucket-{index}-{}", buckets.len()));
            let reset = raw_bucket.reset_time.as_deref().and_then(parse_reset);
            let (reset_at, reset_at_ms) = match reset {
                Some((iso, ms)) => (Some(iso), Some(ms)),
                None => (None, None),
            };
            buckets.push(Bucket {
                label: non_empty(raw_bucket.display_name.as_ref())
                    .unwrap_or_else(|| id.clone()),
                id,
                window: non_empty(raw_bucket.window.as_ref()),
                description: non_empty(raw_bucket.description.as_ref()),
                remaining_percent: clamp_percent(raw_bucket.remaining_fraction),
                reset_at,
                reset_at_ms,
            });
        }

        let group_id = buckets
            .first()
            .map(|b| group_id_for(&b.id, index))
            .unwrap_or_else(|| format!("group-{index}"));

        // Two raw groups could map to the same derived id; keep them distinct.
        let group_id = if groups.iter().any(|g| g.id == group_id) {
            format!("{group_id}-{index}")
        } else {
            group_id
        };

        groups.push(Group {
            id: group_id,
            name: non_empty(raw.display_name.as_ref()).unwrap_or_else(|| format!("Group {}", index + 1)),
            description: non_empty(raw.description.as_ref()),
            buckets,
        });
    }

    groups.sort_by_key(|g| group_sort_key(&g.id));

    let account = status.and_then(|s| s.user_status).map(|raw| Account {
        plan: raw
            .plan_status
            .and_then(|p| p.plan_info)
            .and_then(|p| non_empty(p.plan_name.as_ref()).or_else(|| non_empty(p.teams_tier.as_ref()))),
        tier: raw.user_tier.and_then(|t| non_empty(t.name.as_ref())),
    });

    let mut normalized_models: Vec<Model> = Vec::new();
    if let Some(catalog) = models.and_then(|m| m.response) {
        for (id, raw) in catalog.models {
            if raw.is_internal.unwrap_or(false) {
                continue;
            }
            if id.to_ascii_lowercase().starts_with("tab_") {
                continue;
            }
            let Some(name) = non_empty(raw.display_name.as_ref()) else {
                continue;
            };
            let provider = provider_label(&raw);

            let pool = match provider.as_str() {
                "Google" => groups.iter().find(|g| g.id == "gemini"),
                "Anthropic" | "OpenAI" => groups.iter().find(|g| g.id == "third-party"),
                _ => None,
            };

            let quota = raw.quota_info.unwrap_or_default();
            let own_reset = quota.reset_time.as_deref().and_then(parse_reset);

            // Antigravity groups models into shared quota pools: every model in
            // a pool is limited by the pool's tightest reported bucket. Only a
            // model outside every pool has a limit of its own.
            let limit = pool.and_then(tightest_bucket);
            let shared = limit.is_some();

            let (remaining_percent, reset_at, reset_at_ms, limit_id, limit_label) =
                if let Some(bucket) = limit {
                    (
                        bucket.remaining_percent,
                        bucket.reset_at.clone(),
                        bucket.reset_at_ms,
                        Some(bucket.id.clone()),
                        bucket.window.clone().or_else(|| Some(bucket.label.clone())),
                    )
                } else {
                    let reported = clamp_percent(quota.remaining_fraction);
                    let has_own_limit = reported.is_some() || own_reset.is_some();
                    (
                        reported,
                        own_reset.as_ref().map(|(iso, _)| iso.clone()),
                        own_reset.as_ref().map(|(_, ms)| *ms),
                        has_own_limit.then(|| "model".to_string()),
                        has_own_limit.then(|| "model".to_string()),
                    )
                };

            normalized_models.push(Model {
                id,
                name,
                provider,
                pool_id: pool.map(|g| g.id.clone()),
                pool_name: pool.map(|g| g.name.clone()),
                limit_id,
                limit_label,
                shared,
                remaining_percent,
                reset_at,
                reset_at_ms,
            });
        }
    }

    // Collapse duplicate display names (the catalog lists the same model under
    // several internal ids). Keep the most informative entry.
    let mut by_name: BTreeMap<String, Model> = BTreeMap::new();
    for model in normalized_models {
        let key = model.name.to_ascii_lowercase();
        match by_name.get(&key) {
            Some(existing) if existing.remaining_percent.is_some() => {}
            _ => {
                by_name.insert(key, model);
            }
        }
    }
    let mut models: Vec<Model> = by_name.into_values().collect();
    models.sort_by(|a, b| {
        provider_rank(&a.provider)
            .cmp(&provider_rank(&b.provider))
            .then_with(|| a.name.to_ascii_lowercase().cmp(&b.name.to_ascii_lowercase()))
    });

    let error = if groups.is_empty() {
        Some(Problem {
            code: "no_quota_data".into(),
            title: "No quota data reported".into(),
            detail: "Antigravity responded, but did not report any quota groups for this account. \
                     Refresh after starting a chat to let the local service populate limits."
                .into(),
        })
    } else {
        None
    };

    Snapshot {
        status: if error.is_some() { "unavailable" } else { "ok" },
        fetched_at: format_time(now),
        fetched_at_ms: to_millis(now),
        account,
        groups,
        models,
        source: Some(source),
        error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn now() -> OffsetDateTime {
        OffsetDateTime::parse("2026-10-23T19:18:00Z", &Rfc3339).unwrap()
    }

    const SUMMARY: &str = r#"{
      "response": {
        "groups": [
          {
            "displayName": "Gemini Models",
            "description": "Models within this group: Gemini Flash, Gemini Pro",
            "buckets": [
              {"bucketId":"gemini-weekly","displayName":"Weekly Limit Remaining","window":"weekly","remainingFraction":0.3281504,"resetTime":"2026-10-24T02:34:50Z"},
              {"bucketId":"gemini-5h","displayName":"Five Hour Limit Remaining","window":"5h","remainingFraction":1,"resetTime":"2026-10-24T00:16:52Z"}
            ]
          },
          {
            "displayName": "Claude and GPT models",
            "buckets": [
              {"bucketId":"3p-weekly","displayName":"Weekly Limit Remaining","window":"weekly","remainingFraction":0.4926136,"resetTime":"2026-10-29T03:35:10Z"},
              {"bucketId":"3p-5h","displayName":"Five Hour Limit Remaining","window":"5h","remainingFraction":1,"resetTime":"2026-10-24T00:16:52Z"}
            ]
          }
        ]
      }
    }"#;

    const MODELS: &str = r#"{
      "response": {
        "models": {
          "chat_20706": {"modelProvider":"MODEL_PROVIDER_GOOGLE","apiProvider":"API_PROVIDER_INTERNAL","isInternal":true,"quotaInfo":{"remainingFraction":1}},
          "tab_flash": {"modelProvider":"MODEL_PROVIDER_GOOGLE","quotaInfo":{"remainingFraction":1}},
          "claude-opus-4-6-thinking": {"displayName":"Claude Opus 4.6 (Thinking)","modelProvider":"MODEL_PROVIDER_ANTHROPIC","apiProvider":"API_PROVIDER_ANTHROPIC_VERTEX","quotaInfo":{"remainingFraction":0.4926136,"resetTime":"2026-10-29T03:35:10Z"}},
          "claude-sonnet-4-6": {"displayName":"Claude Sonnet 4.6 (Thinking)","modelProvider":"MODEL_PROVIDER_ANTHROPIC","apiProvider":"API_PROVIDER_ANTHROPIC_VERTEX","quotaInfo":{"remainingFraction":0.4926136,"resetTime":"2026-10-29T03:35:10Z"}},
          "gemini-3-flash": {"displayName":"Gemini 3 Flash","modelProvider":"MODEL_PROVIDER_GOOGLE","apiProvider":"API_PROVIDER_GOOGLE_GEMINI","quotaInfo":{"remainingFraction":1,"resetTime":"2026-10-24T00:16:52Z"}},
          "gemini-3.5-flash-lite": {"displayName":"Gemini 3.5 Flash Lite","modelProvider":"MODEL_PROVIDER_GOOGLE","apiProvider":"API_PROVIDER_GOOGLE_GEMINI","quotaInfo":{"remainingFraction":1,"resetTime":"2026-10-24T00:16:52Z"}},
          "gemini-2.5-flash": {"displayName":"Gemini 3.5 Flash Lite","modelProvider":"MODEL_PROVIDER_GOOGLE","apiProvider":"API_PROVIDER_GOOGLE_GEMINI","quotaInfo":{"remainingFraction":0.9,"resetTime":"2026-10-24T00:16:52Z"}},
          "gpt-oss-120b-medium": {"displayName":"GPT-OSS 120B (Medium)","modelProvider":"MODEL_PROVIDER_OPENAI","apiProvider":"API_PROVIDER_OPENAI_VERTEX","quotaInfo":{"remainingFraction":0.4926136,"resetTime":"2026-10-29T03:35:10Z"}},
          "experimental-model": {"displayName":"Experimental Model","modelProvider":"MODEL_PROVIDER_EXPERIMENTAL","apiProvider":"API_PROVIDER_EXPERIMENTAL","quotaInfo":{"remainingFraction":0.25,"resetTime":"2026-11-01T00:00:00Z"}},
          "no-quota": {"displayName":"Fresh Model","modelProvider":"MODEL_PROVIDER_GOOGLE","apiProvider":"API_PROVIDER_GOOGLE_GEMINI","quotaInfo":{}}
        }
      }
    }"#;

    const STATUS: &str = r#"{"userStatus":{"name":"Test","email":"test@example.com","planStatus":{"planInfo":{"planName":"Pro","teamsTier":"TEAMS_TIER_PRO"}},"userTier":{"name":"Google AI Pro"}}}"#;

    fn snapshot() -> Snapshot {
        build_snapshot(
            now(),
            serde_json::from_str(SUMMARY).unwrap(),
            Some(serde_json::from_str(MODELS).unwrap()),
            Some(serde_json::from_str(STATUS).unwrap()),
            "Antigravity IDE".to_string(),
        )
    }

    #[test]
    fn normalizes_groups_and_buckets() {
        let snap = snapshot();
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.groups.len(), 2);

        let gemini = &snap.groups[0];
        assert_eq!(gemini.id, "gemini");
        assert_eq!(gemini.buckets.len(), 2);
        assert_eq!(gemini.buckets[0].remaining_percent, Some(32.8));
        assert_eq!(gemini.buckets[0].reset_at.as_deref(), Some("2026-10-24T02:34:50Z"));
        assert_eq!(gemini.buckets[0].reset_at_ms, Some(1_792_809_290_000));
        assert_eq!(gemini.buckets[1].window.as_deref(), Some("5h"));

        assert_eq!(snap.groups[1].id, "third-party");
        assert_eq!(snap.account.as_ref().unwrap().plan.as_deref(), Some("Pro"));
        assert_eq!(snap.account.as_ref().unwrap().tier.as_deref(), Some("Google AI Pro"));
    }

    #[test]
    fn filters_internal_and_unlabeled_models() {
        let snap = snapshot();
        let ids: Vec<&str> = snap.models.iter().map(|m| m.id.as_str()).collect();
        assert!(!ids.contains(&"chat_20706"));
        assert!(!ids.contains(&"tab_flash"));
        assert!(ids.contains(&"gemini-3-flash"));
    }

    #[test]
    fn deduplicates_identical_display_names() {
        let snap = snapshot();
        let flash_lite: Vec<&Model> = snap
            .models
            .iter()
            .filter(|m| m.name == "Gemini 3.5 Flash Lite")
            .collect();
        assert_eq!(flash_lite.len(), 1);
        // Both entries draw on the same shared Gemini pool.
        assert_eq!(flash_lite[0].remaining_percent, Some(32.8));
        assert_eq!(flash_lite[0].limit_id.as_deref(), Some("gemini-weekly"));
    }

    #[test]
    fn marks_shared_pool_limits() {
        let snap = snapshot();
        let opus = snap.models.iter().find(|m| m.name.starts_with("Claude Opus")).unwrap();
        assert_eq!(opus.pool_id.as_deref(), Some("third-party"));
        assert_eq!(opus.limit_id.as_deref(), Some("3p-weekly"));
        assert!(opus.shared);
        assert_eq!(opus.remaining_percent, Some(49.3));

        // Every model in a pool is gated by the pool's tightest bucket.
        let flash = snap.models.iter().find(|m| m.name == "Gemini 3 Flash").unwrap();
        assert_eq!(flash.pool_id.as_deref(), Some("gemini"));
        assert_eq!(flash.limit_id.as_deref(), Some("gemini-weekly"));
        assert!(flash.shared);
        assert_eq!(flash.remaining_percent, Some(32.8));
    }

    #[test]
    fn distinguishes_model_specific_limits() {
        let snap = snapshot();
        let model = snap.models.iter().find(|m| m.name == "Experimental Model").unwrap();
        assert!(!model.shared);
        assert_eq!(model.limit_id.as_deref(), Some("model"));
        assert_eq!(model.remaining_percent, Some(25.0));
        assert_eq!(model.reset_at.as_deref(), Some("2026-11-01T00:00:00Z"));
    }

    #[test]
    fn pool_models_ignore_a_differing_reported_fraction() {
        // The catalog can report a fraction for a different window than the
        // one currently gating the pool; the pool value wins.
        let snap = snapshot();
        let model = snap.models.iter().find(|m| m.name == "Gemini 3 Flash").unwrap();
        assert_ne!(model.remaining_percent, Some(100.0));
        assert_eq!(model.reset_at.as_deref(), Some("2026-10-24T02:34:50Z"));
    }

    #[test]
    fn missing_fraction_stays_unknown() {
        let snap = snapshot();
        let model = snap.models.iter().find(|m| m.name == "Fresh Model").unwrap();
        // No reset time: falls back to the pool's weekly bucket value, which is
        // reported by the service rather than inferred.
        assert_eq!(model.limit_id.as_deref(), Some("gemini-weekly"));
        assert_eq!(model.remaining_percent, Some(32.8));
    }

    #[test]
    fn missing_bucket_fraction_stays_unknown() {
        let summary = r#"{"response":{"groups":[{"displayName":"Gemini Models","buckets":[{"bucketId":"gemini-weekly","displayName":"Weekly","window":"weekly","resetTime":"2026-10-24T02:34:50Z"}]}]}}"#;
        let snap = build_snapshot(
            now(),
            serde_json::from_str(summary).unwrap(),
            None,
            None,
            "test".into(),
        );
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.groups[0].buckets[0].remaining_percent, None);
        assert!(snap.groups[0].buckets[0].reset_at_ms.is_some());
    }

    #[test]
    fn malformed_reset_time_does_not_fail() {
        let summary = r#"{"response":{"groups":[{"displayName":"Gemini Models","buckets":[{"bucketId":"gemini-weekly","displayName":"Weekly","remainingFraction":0.5,"resetTime":"not-a-date"}]}]}}"#;
        let snap = build_snapshot(
            now(),
            serde_json::from_str(summary).unwrap(),
            None,
            None,
            "test".into(),
        );
        assert_eq!(snap.status, "ok");
        assert_eq!(snap.groups[0].buckets[0].reset_at, None);
        assert_eq!(snap.groups[0].buckets[0].remaining_percent, Some(50.0));
    }

    #[test]
    fn empty_groups_is_unavailable() {
        let snap = build_snapshot(
            now(),
            serde_json::from_str(r#"{"response":{"groups":[]}}"#).unwrap(),
            None,
            None,
            "test".into(),
        );
        assert_eq!(snap.status, "unavailable");
        assert_eq!(snap.error.unwrap().code, "no_quota_data");
    }

    #[test]
    fn clamps_out_of_range_fractions() {
        assert_eq!(clamp_percent(Some(1.4)), Some(100.0));
        assert_eq!(clamp_percent(Some(-0.3)), Some(0.0));
        assert_eq!(clamp_percent(Some(f64::NAN)), None);
        assert_eq!(clamp_percent(None), None);
    }

    #[test]
    fn ignores_session_identity_fields() {
        let snap = snapshot();
        let json = serde_json::to_string(&snap).unwrap();
        assert!(!json.contains("test@example.com"));
        assert!(!json.contains("Test\""));
    }
}
