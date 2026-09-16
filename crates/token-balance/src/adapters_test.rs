use super::*;
use crate::credentials::Credentials;
use crate::domain::{ProviderStatus, WindowLabel, effective_available, hero_window};
use crate::providers::{Provider, RefreshPolicy, RefreshTrigger, allows_refresh};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::PathBuf;

const KIMI_JSON: &str = r#"{
  "usage": {
    "limit": "2048",
    "used": "214",
    "remaining": "1834",
    "resetTime": "2026-09-21T15:32:00Z"
  },
  "limits": [{
    "window": {"duration": 300, "timeUnit": "TIME_UNIT_MINUTE"},
    "detail": {
      "limit": "200",
      "used": "90",
      "remaining": "110",
      "resetTime": "2026-09-15T17:32:00Z"
    }
  }],
  "user": { "membership": { "level": "LEVEL_INTERMEDIATE" } }
}"#;

const ZAI_JSON: &str = r#"{
  "code": 200,
  "data": {
    "level": "PRO",
    "limits": [
      {
        "type": "TOKENS_LIMIT",
        "unit": 3,
        "number": 5,
        "percentage": 25,
        "nextResetTime": 1770648402389
      },
      {
        "type": "TOKENS_LIMIT",
        "unit": 6,
        "number": 7,
        "percentage": 40,
        "nextResetTime": 1771253202389
      },
      {
        "type": "TIME_LIMIT",
        "unit": 5,
        "number": 1,
        "percentage": 45
      }
    ]
  }
}"#;

const CODEX_JSON: &str = r#"{
  "result": {
    "rateLimits": {
      "limitId": "codex",
      "planType": "plus",
      "primary": { "usedPercent": 82, "windowDurationMins": 300, "resetsAt": 1757948640 },
      "secondary": { "usedPercent": 37, "windowDurationMins": 10080, "resetsAt": 1758301920 }
    },
    "rateLimitsByLimitId": {
      "codex": {
        "limitId": "codex",
        "planType": "plus",
        "primary": { "usedPercent": 82, "windowDurationMins": 300, "resetsAt": 1757948640 },
        "secondary": { "usedPercent": 37, "windowDurationMins": 10080, "resetsAt": 1758301920 }
      }
    }
  }
}"#;

const CODEX_WEEKLY_ONLY: &str = r#"{
  "result": {
    "rateLimits": {
      "primary": null,
      "secondary": { "usedPercent": 40, "windowDurationMins": 10080, "resetsAt": 1758301920 }
    }
  }
}"#;

const CLAUDE_JSON: &str = r#"{
  "five_hour": { "utilization": 28.0, "resets_at": "2026-09-15T16:37:00Z" },
  "seven_day": { "utilization": 59.0, "resets_at": "2026-09-21T01:32:00Z" },
  "extra_usage": {
    "is_enabled": true,
    "monthly_limit": 25.0,
    "used_credits": 12.6,
    "utilization": null
  }
}"#;

const GROK_JSON: &str = r#"{
  "config": {
    "creditUsagePercent": 73.0,
    "currentPeriod": { "type": "USAGE_PERIOD_TYPE_WEEKLY", "end": "2026-09-18T18:32:00Z" },
    "onDemandCap": { "val": 400 },
    "onDemandUsed": { "val": 0 },
    "prepaidBalance": { "val": 9999 },
    "remaining_balance": 42.0
  }
}"#;

const MUSE_SSE: &str = "event: response.subscription_usage\ndata: {\"type\":\"response.subscription_usage\",\"subscription_usage\":{\"used_percent\":20,\"window_duration_mins\":300,\"resets_at\":\"2026-09-15T19:32:00Z\"}}\n";

#[test]
fn kimi_maps_remaining_from_usages_url() {
    assert_eq!(KIMI_USAGES_URL, "https://api.kimi.com/coding/v1/usages");
    let v: Value = serde_json::from_str(KIMI_JSON).unwrap();
    let status = map_kimi_usages(&v);
    let av = effective_available(&status).expect("available");
    let session = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::FiveHour))
        .unwrap();
    assert!((session.remaining_percent - 55.0).abs() < 0.01);
    let weekly = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Weekly))
        .unwrap();
    let expect = 1834.0 / 2048.0 * 100.0;
    assert!((weekly.remaining_percent - expect as f32).abs() < 0.2);
}

#[tokio::test]
async fn kimi_recorded_fetch_uses_mapper() {
    let v: Value = serde_json::from_str(KIMI_JSON).unwrap();
    match KimiAdapter::with_recorded(v).fetch().await {
        ProviderStatus::Available { .. } => {}
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn kimi_missing_key_is_not_configured() {
    let k = KimiAdapter::from_credentials(&Credentials::empty());
    match k.fetch().await {
        ProviderStatus::NotConfigured { .. } => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn zai_credit_limit_maps_5h_and_weekly_number_1() {
    let v: Value = serde_json::from_str(
        r#"{
          "code": 200,
          "data": {
            "level": "max",
            "limits": [
              {
                "type": "CREDIT_LIMIT",
                "unit": 3,
                "number": 5,
                "percentage": 11,
                "nextResetTime": 1789545169435
              },
              {
                "type": "CREDIT_LIMIT",
                "unit": 6,
                "number": 1,
                "percentage": 27,
                "nextResetTime": 1789810016984
              }
            ]
          }
        }"#,
    )
    .unwrap();
    let status = map_zai_quota(&v);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("max"));
    let session = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::FiveHour))
        .unwrap();
    assert_eq!(session.used_percent, 11.0);
    assert_eq!(session.remaining_percent, 89.0);
    let weekly = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Weekly))
        .unwrap();
    assert_eq!(weekly.used_percent, 27.0);
    assert_eq!(weekly.remaining_percent, 73.0);
}

#[test]
fn zai_percentage_is_used_and_zhipu_ignored() {
    assert_eq!(
        ZAI_QUOTA_URL,
        "https://api.z.ai/api/monitor/usage/quota/limit"
    );
    let v: Value = serde_json::from_str(ZAI_JSON).unwrap();
    let status = map_zai_quota(&v);
    let av = effective_available(&status).unwrap();
    let session = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::FiveHour))
        .unwrap();
    assert_eq!(session.used_percent, 25.0);
    assert_eq!(session.remaining_percent, 75.0);
}

#[tokio::test]
async fn zai_recorded_fetch() {
    let v: Value = serde_json::from_str(ZAI_JSON).unwrap();
    match ZaiAdapter::with_recorded(v).fetch().await {
        ProviderStatus::Available { .. } => {}
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn zai_zhipu_key_only_is_not_configured() {
    let mut env = BTreeMap::new();
    env.insert("ZHIPUAI_API_KEY".into(), "zhipu-redacted".into());
    let c = Credentials::isolated(PathBuf::from("/nonexistent-tb-home"), env);
    let z = ZaiAdapter::from_credentials(&c);
    match z.fetch().await {
        ProviderStatus::NotConfigured { .. } => {}
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn zai_missing_pointer_hint_names_env() {
    use crate::accounts::Pointer;
    use crate::providers::AccountIdentity;
    let c = Credentials::empty();
    let z = ZaiAdapter::from_account(
        &c,
        AccountIdentity {
            id: "zai-2".into(),
            label: "glm 2".into(),
            vendor: "zai",
        },
        &Pointer::Env("ZAI_API_KEY_2".into()),
    );
    match z.fetch().await {
        ProviderStatus::NotConfigured { hint } => {
            assert!(hint.contains("ZAI_API_KEY_2"), "{hint}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn codex_used_percent_and_duration() {
    let v: Value = serde_json::from_str(CODEX_JSON).unwrap();
    let status = map_codex_rate_limits(&v);
    let av = effective_available(&status).unwrap();
    let session = av
        .windows
        .iter()
        .find(|w| w.duration_mins == Some(300))
        .unwrap();
    assert_eq!(session.remaining_percent, 18.0);
    assert!(session.is_session());
}

#[test]
fn codex_weekly_only_has_no_invented_5h() {
    let v: Value = serde_json::from_str(CODEX_WEEKLY_ONLY).unwrap();
    let status = map_codex_rate_limits(&v);
    let av = effective_available(&status).unwrap();
    assert!(av.windows.iter().all(|w| !w.is_session()));
    assert!(
        av.windows
            .iter()
            .all(|w| !matches!(w.label, WindowLabel::FiveHour))
    );
    let hero = hero_window(av.windows).unwrap();
    assert!(matches!(hero.label, WindowLabel::Weekly));
}

#[test]
fn claude_utilization_is_used_percent() {
    assert_eq!(
        CLAUDE_USAGE_URL,
        "https://api.anthropic.com/api/oauth/usage"
    );
    assert_eq!(CLAUDE_BETA, "oauth-2025-04-20");
    let v: Value = serde_json::from_str(CLAUDE_JSON).unwrap();
    let status = map_claude_usage(&v);
    let av = effective_available(&status).unwrap();
    let fh = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::FiveHour))
        .unwrap();
    assert_eq!(fh.used_percent, 28.0);
    assert_eq!(fh.remaining_percent, 72.0);
    let extra = av.extra.as_ref().unwrap();
    assert!((extra.remaining - 12.4).abs() < 0.05);
}

#[test]
fn grok_uses_credit_usage_not_prepaid() {
    assert_eq!(
        GROK_BILLING_URL,
        "https://cli-chat-proxy.grok.com/v1/billing?format=credits"
    );
    let v: Value = serde_json::from_str(GROK_JSON).unwrap();
    let status = map_grok_billing(&v);
    let av = effective_available(&status).unwrap();
    assert_eq!(av.windows.len(), 1);
    assert!(matches!(av.windows[0].label, WindowLabel::Weekly));
    assert_eq!(av.windows[0].remaining_percent, 27.0);
    let extra = av.extra.as_ref().unwrap();
    assert_eq!(extra.remaining, 400.0);
}

#[test]
fn grok_prepaid_only_is_error() {
    let v = json!({ "remaining_balance": 12.5, "prepaidBalance": { "val": 99 } });
    match map_grok_billing(&v) {
        ProviderStatus::Error { .. } => {}
        other => panic!("prepaid must not map as remaining: {other:?}"),
    }
}

#[test]
fn grok_cli_auth_json_nested_key_is_token() {
    use crate::accounts::Pointer;
    use std::sync::atomic::{AtomicU64, Ordering};
    static N: AtomicU64 = AtomicU64::new(1);
    let home = std::env::temp_dir().join(format!(
        "tb-grok-auth-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(home.join(".grok")).unwrap();
    std::fs::write(
        home.join(".grok/auth.json"),
        r#"{"https://auth.x.ai::example":{"key":"redacted-grok","refresh_token":"r"}}"#,
    )
    .unwrap();
    let c = Credentials::isolated(home, BTreeMap::new());
    let got = pointer_secret(&c, &Pointer::File(".grok/auth.json".into()), GROK_AUTH_KEYS);
    assert_eq!(got.as_deref(), Some("redacted-grok"));
}

#[test]
fn muse_keychain_blob_prefers_api_key() {
    let blob = r#"{"secret_schema_version":1,"api_key":"muse-api","access_token":"muse-access"}"#;
    assert_eq!(token_from_keychain_blob(blob).as_deref(), Some("muse-api"));
}

#[test]
fn muse_default_skips_timer_and_does_not_post() {
    assert_eq!(MUSE_RESPONSES_URL, "https://api.meta.ai/v1/responses");
    let m = MuseAdapter::unsupported();
    assert_eq!(m.refresh_policy(), RefreshPolicy::OnDemand);
    assert!(!allows_refresh(m.refresh_policy(), RefreshTrigger::Timer));
    assert!(allows_refresh(m.refresh_policy(), RefreshTrigger::Manual));
}

#[tokio::test]
async fn muse_default_fetch_is_unsupported() {
    let m = MuseAdapter::unsupported();
    match m.fetch().await {
        ProviderStatus::Unsupported { .. } => {}
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn recorded_fetch_codex_claude_grok_muse() {
    let c: Value = serde_json::from_str(CODEX_JSON).unwrap();
    match CodexAdapter::with_recorded(c).fetch().await {
        ProviderStatus::Available { .. } => {}
        other => panic!("{other:?}"),
    }
    let cl: Value = serde_json::from_str(CLAUDE_JSON).unwrap();
    match ClaudeAdapter::with_recorded(cl).fetch().await {
        ProviderStatus::Available { .. } => {}
        other => panic!("{other:?}"),
    }
    let g: Value = serde_json::from_str(GROK_JSON).unwrap();
    match GrokAdapter::with_recorded(g).fetch().await {
        ProviderStatus::Available { .. } => {}
        other => panic!("{other:?}"),
    }
    match MuseAdapter::with_recorded(MUSE_SSE.into()).fetch().await {
        ProviderStatus::Available { .. } => {}
        other => panic!("{other:?}"),
    }
}

#[test]
fn muse_opt_in_maps_sse() {
    let status = map_muse_sse(MUSE_SSE);
    let av = effective_available(&status).unwrap();
    assert_eq!(av.windows[0].remaining_percent, 80.0);
}

#[test]
fn muse_nested_subscription_maps_remaining() {
    let sse = r#"event: response.subscription_usage
data: {"type":"response.subscription_usage","subscription":{"tier":1,"weekly":{"resets_at":1789948800,"used_percent":34},"window":{"resets_at":1789571667,"used_percent":0,"window_duration_mins":300}}}
"#;
    let status = map_muse_sse(sse);
    let av = effective_available(&status).expect("available");
    let fh = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::FiveHour))
        .unwrap();
    assert_eq!(fh.remaining_percent, 100.0);
    let wk = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Weekly))
        .unwrap();
    assert_eq!(wk.used_percent, 34.0);
    assert_eq!(wk.remaining_percent, 66.0);
}

#[tokio::test]
async fn fixture_mixed_is_whole_registry() {
    use crate::domain::Clock;
    use crate::fixtures::{FixtureSet, fixture_registry, frozen_demo_clock};
    let clock: std::sync::Arc<dyn Clock> = std::sync::Arc::new(frozen_demo_clock());
    let list = fixture_registry(FixtureSet::Mixed, clock);
    assert_eq!(list.len(), 6);
    let kimi = list.iter().find(|p| p.id() == "kimi").unwrap();
    match kimi.fetch().await {
        ProviderStatus::Available { .. } => {}
        other => panic!("fixture must replace live: {other:?}"),
    }
}
