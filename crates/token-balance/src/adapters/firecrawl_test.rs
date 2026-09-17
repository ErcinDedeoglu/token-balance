use super::*;
use crate::domain::{CreditUnit, ProviderStatus, WindowLabel, effective_available};
use serde_json::json;

#[test]
fn remaining_credits_is_wallet_not_percent() {
    let v = json!({
        "success": true,
        "data": {
            "remainingCredits": 1000,
            "planCredits": 5000,
            "billingPeriodStart": "2025-01-01T00:00:00Z",
            "billingPeriodEnd": "2025-01-31T23:59:59Z"
        }
    });
    let status = map_firecrawl_credits(&v);
    let av = effective_available(&status).expect("available");
    assert!(av.windows.is_empty());
    assert!(!av.windows.iter().any(|w| matches!(
        w.label,
        WindowLabel::FiveHour | WindowLabel::Weekly
    )));
    let extra = av.extra.as_ref().unwrap();
    assert_eq!(extra.remaining, 1000.0);
    assert_eq!(extra.limit, Some(5000.0));
    assert!(matches!(extra.unit, CreditUnit::Credits));
}

#[test]
fn v1_snake_case_remaining_credits() {
    let v = json!({"success": true, "data": {"remaining_credits": 42}});
    let extra = effective_available(&map_firecrawl_credits(&v))
        .unwrap()
        .extra
        .clone()
        .unwrap();
    assert_eq!(extra.remaining, 42.0);
    assert!(extra.limit.is_none());
}

#[test]
fn success_false_and_missing_remaining_are_error() {
    match map_firecrawl_credits(&json!({"success": false, "error": "Could not find credit usage information"}))
    {
        ProviderStatus::Error { message, .. } => {
            assert!(message.contains("Could not find"), "{message}");
        }
        other => panic!("{other:?}"),
    }
    match map_firecrawl_credits(&json!({"success": true, "data": {}})) {
        ProviderStatus::Error { message, .. } => {
            assert!(message.contains("remainingCredits"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn historical_spend_is_not_remaining() {
    let v = json!({
        "success": true,
        "periods": [{"totalCredits": 1000, "startDate": "2025-01-01T00:00:00Z"}]
    });
    match map_firecrawl_credits(&v) {
        ProviderStatus::Error { message, .. } => {
            assert!(message.contains("remainingCredits"), "{message}");
        }
        other => panic!("historical spend must not become remaining: {other:?}"),
    }
}

#[tokio::test]
async fn recorded_fetch() {
    let v = json!({"success": true, "data": {"remainingCredits": 7, "planCredits": 1000}});
    match FirecrawlAdapter::with_recorded(v).fetch().await {
        ProviderStatus::Available { windows, extra, .. } => {
            assert!(windows.is_empty());
            let extra = extra.unwrap();
            assert_eq!(extra.remaining, 7.0);
            assert_eq!(extra.limit, Some(1000.0));
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn missing_key_is_not_configured() {
    let home = std::env::temp_dir().join(format!(
        "tb-fc-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).unwrap();
    let creds = crate::credentials::Credentials::isolated(home, std::collections::BTreeMap::new());
    let ident = AccountIdentity::vendor_default("firecrawl", "firecrawl");
    let adapter = FirecrawlAdapter::from_account(
        &creds,
        ident,
        &crate::accounts::Pointer::Env("FIRECRAWL_API_KEY".into()),
    );
    match adapter.fetch().await {
        ProviderStatus::NotConfigured { hint } => {
            assert!(hint.contains("FIRECRAWL_API_KEY"), "{hint}");
            assert!(!hint.contains("0%"), "{hint}");
        }
        other => panic!("{other:?}"),
    }
}
