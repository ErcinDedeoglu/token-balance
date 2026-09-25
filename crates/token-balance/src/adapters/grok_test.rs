use super::oidc::{
    access_expired, map_oidc_refresh, oidc_token_url, parse_grok_auth, persist_refreshed,
};
use super::*;
use crate::adapters::pointer_secret;
use crate::domain::{ProviderStatus, WindowLabel, effective_available};
use crate::providers::{AccountIdentity, Provider};
use serde_json::{Value, json};
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, Ordering};

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

fn scratch_home() -> std::path::PathBuf {
    static N: AtomicU64 = AtomicU64::new(1);
    std::env::temp_dir().join(format!(
        "tb-grok-{}-{}",
        std::process::id(),
        N.fetch_add(1, Ordering::Relaxed)
    ))
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
fn grok_fresh_weekly_period_without_percent_is_full() {
    let v = json!({
        "config": {
            "currentPeriod": {
                "type": "USAGE_PERIOD_TYPE_WEEKLY",
                "start": "2026-09-25T19:00:05Z",
                "end": "2026-10-02T19:00:05Z"
            },
            "onDemandCap": { "val": 0 },
            "onDemandUsed": { "val": 0 },
            "prepaidBalance": { "val": 0 },
            "billingPeriodEnd": "2026-10-02T19:00:05Z"
        }
    });
    let status = map_grok_billing(&v);
    let av = effective_available(&status).expect("weekly window");
    assert_eq!(av.windows[0].remaining_percent, 100.0);
    assert!(av.extra.is_none());
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
    let home = scratch_home();
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
fn grok_rereads_auth_json_after_login() {
    let home = scratch_home();
    std::fs::create_dir_all(home.join(".grok")).unwrap();
    std::fs::write(
        home.join(".grok/auth.json"),
        r#"{"https://auth.x.ai::old":{"key":"stale-jwt","refresh_token":"r"}}"#,
    )
    .unwrap();
    let c = Credentials::isolated(home.clone(), BTreeMap::new());
    let adapter = GrokAdapter::from_account(
        &c,
        AccountIdentity::vendor_default("grok", "Grok"),
        &Pointer::File(".grok/auth.json".into()),
    );
    assert_eq!(adapter.live_token().as_deref(), Some("stale-jwt"));
    std::fs::write(
        home.join(".grok/auth.json"),
        r#"{"https://auth.x.ai::new":{"key":"fresh-jwt","refresh_token":"r2"}}"#,
    )
    .unwrap();
    assert_eq!(adapter.live_token().as_deref(), Some("fresh-jwt"));
}

#[test]
fn grok_401_points_at_login() {
    let m = billing_http_error(401);
    assert!(m.contains("401"), "{m}");
    assert!(m.contains("grok login"), "{m}");
    assert_eq!(billing_http_error(503), "HTTP 503");
}

#[test]
fn grok_parses_nested_oidc_and_expired_access() {
    let text = r#"{"https://auth.x.ai::x":{"key":"jwt","refresh_token":"rt","oidc_client_id":"cid","oidc_issuer":"https://auth.x.ai","expires_at":"2020-01-01T00:00:00Z"}}"#;
    let a = parse_grok_auth(text).unwrap();
    assert_eq!(a.access, "jwt");
    assert_eq!(a.refresh.as_deref(), Some("rt"));
    assert_eq!(a.client_id.as_deref(), Some("cid"));
    assert_eq!(a.issuer.as_deref(), Some("https://auth.x.ai"));
    assert!(access_expired(&a));
    assert_eq!(
        oidc_token_url("https://auth.x.ai"),
        "https://auth.x.ai/oauth2/token"
    );
}

#[test]
fn grok_maps_oidc_refresh_json() {
    let v = json!({
        "access_token": "new",
        "refresh_token": "rt2",
        "expires_in": 21600,
        "token_type": "Bearer"
    });
    let (a, r, exp) = map_oidc_refresh(&v).unwrap();
    assert_eq!(a, "new");
    assert_eq!(r.as_deref(), Some("rt2"));
    assert_eq!(exp, 21600);
}

#[test]
fn grok_persists_rotated_refresh_into_auth_json() {
    let home = scratch_home();
    let path = home.join(".grok/auth.json");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        r#"{"https://auth.x.ai::x":{"key":"old","refresh_token":"rt1","oidc_client_id":"cid","oidc_issuer":"https://auth.x.ai","expires_at":"2020-01-01T00:00:00Z"}}"#,
    )
    .unwrap();
    let c = Credentials::isolated(home, BTreeMap::new());
    persist_refreshed(
        &c,
        &Pointer::File(".grok/auth.json".into()),
        "new-jwt",
        Some("rt2"),
        21600,
    );
    let text = std::fs::read_to_string(path).unwrap();
    assert!(text.contains("new-jwt"), "{text}");
    assert!(text.contains("rt2"), "{text}");
    assert!(!text.contains("rt1"), "{text}");
}

#[tokio::test]
#[ignore]
async fn grok_live_oidc_refresh_fetches_billing() {
    let c = Credentials::from_process();
    let adapter = GrokAdapter::from_credentials(&c);
    match adapter.fetch().await {
        ProviderStatus::Available { windows, .. } => {
            assert!(!windows.is_empty(), "live grok weekly window");
        }
        other => panic!("live grok fetch failed: {other:?}"),
    }
}
