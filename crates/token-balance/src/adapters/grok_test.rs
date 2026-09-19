use super::*;
use crate::adapters::pointer_secret;
use crate::domain::{ProviderStatus, WindowLabel, effective_available};
use crate::providers::AccountIdentity;
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
