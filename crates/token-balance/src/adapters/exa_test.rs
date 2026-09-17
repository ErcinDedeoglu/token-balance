use super::*;
use crate::domain::{CreditUnit, ProviderStatus, WindowLabel, effective_available};
use serde_json::json;

#[test]
fn dashboard_429_is_rate_limit_not_cookie() {
    let m = dashboard_http_error(429);
    assert!(m.contains("429"), "{m}");
    assert!(!m.to_ascii_lowercase().contains("cookie"), "{m}");
    let expired = dashboard_http_error(401);
    assert!(expired.contains("exa.json"), "{expired}");
}

fn assert_no_plan_windows(status: &ProviderStatus) {
    if let Some(av) = effective_available(status) {
        for w in av.windows {
            assert!(
                !matches!(w.label, WindowLabel::FiveHour | WindowLabel::Weekly),
                "exa must not paint plan windows: {:?}",
                w.label
            );
        }
    }
}

#[test]
fn orb_credits_in_cents_is_wallet_usd_not_percent() {
    let v = json!({
        "orbCreditsInCents": 1641.481,
        "orbInvoiceDebt": 0,
        "expiringCredits": [{"balanceCents": 776, "expiresAt": "2026-10-01T07:00:00+00:00"}]
    });
    let status = map_exa_json(&v);
    let av = effective_available(&status).expect("available");
    assert!(av.windows.is_empty());
    assert!(!av.windows.iter().any(|w| matches!(
        w.label,
        WindowLabel::FiveHour | WindowLabel::Weekly
    )));
    let extra = av.extra.as_ref().unwrap();
    assert!((extra.remaining - 16.41481).abs() < 1e-6);
    assert!(matches!(extra.unit, CreditUnit::Usd));
    assert_eq!(av.plan.as_deref(), Some("Pay as you go"));
}

#[tokio::test]
async fn recorded_get_credits_fetch_is_wallet() {
    let v = json!({"orbCreditsInCents": 1641.481, "orbInvoiceDebt": 0});
    match ExaAdapter::with_recorded(v).fetch().await {
        ProviderStatus::Available { windows, extra, .. } => {
            assert!(windows.is_empty());
            assert!((extra.unwrap().remaining - 16.41481).abs() < 1e-6);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn spend_only_total_cost_usd_is_error_not_remaining_percent() {
    let v = json!({
        "id": "key_abc123def456",
        "api_key_id": "550e8400-e29b-41d4-a716-446655440000",
        "api_key_name": "Production API Key",
        "team_id": "660e8400-e29b-41d4-a716-446655440000",
        "period": {"start": "2025-01-01T00:00:00Z", "end": "2025-01-31T23:59:59Z"},
        "total_cost_usd": 45.67,
        "cost_breakdown": [
            {"price_id": "price_neural_search", "price_name": "Neural Search", "quantity": 1000, "amount_usd": 30.0}
        ]
    });
    let status = map_exa_json(&v);
    match &status {
        ProviderStatus::Error { message, .. } => {
            assert!(message.contains("spend"), "{message}");
            assert!(message.contains("not remaining"), "{message}");
        }
        other => panic!("expected Error, got {other:?}"),
    }
    assert!(
        effective_available(&status).is_none(),
        "spend must not become remaining percent"
    );
}

#[test]
fn no_more_credits_has_no_fivehour_or_weekly_window() {
    let v = json!({
        "requestId": "req_1",
        "error": "You have exceeded your credits limit. Please top up to keep using Exa at dashboard.exa.ai",
        "tag": "NO_MORE_CREDITS"
    });
    let status = map_exa_json(&v);
    match &status {
        ProviderStatus::Error { message, .. } => {
            assert!(message.contains("NO_MORE_CREDITS"), "{message}");
        }
        other => panic!("{other:?}"),
    }
    assert_no_plan_windows(&status);
    assert!(effective_available(&status).is_none());
}

#[tokio::test]
async fn recorded_spend_fetch_is_error() {
    let v = json!({"total_cost_usd": 1.0});
    match ExaAdapter::with_recorded(v).fetch().await {
        ProviderStatus::Error { message, .. } => {
            assert!(message.contains("spend"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn recorded_no_more_credits_fetch_has_no_plan_windows() {
    let v = json!({"tag": "NO_MORE_CREDITS", "error": "exhausted"});
    let status = ExaAdapter::with_recorded(v).fetch().await;
    assert_no_plan_windows(&status);
    match status {
        ProviderStatus::Error { .. } => {}
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn missing_key_is_not_configured() {
    let home = std::env::temp_dir().join(format!(
        "tb-exa-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).unwrap();
    let creds = crate::credentials::Credentials::isolated(home, std::collections::BTreeMap::new());
    let ident = AccountIdentity::vendor_default("exa", "exa");
    let adapter = ExaAdapter::from_account(
        &creds,
        ident,
        &crate::accounts::Pointer::Env("EXA_API_KEY".into()),
    );
    match adapter.fetch().await {
        ProviderStatus::NotConfigured { hint } => {
            assert!(hint.contains("exa.json"), "{hint}");
            assert!(!hint.contains("0%"), "{hint}");
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn env_api_key_is_not_remaining_source() {
    let home = std::env::temp_dir().join(format!(
        "tb-exa-key-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).unwrap();
    let mut env = std::collections::BTreeMap::new();
    env.insert("EXA_API_KEY".into(), "redacted-not-a-live-call".into());
    let creds = crate::credentials::Credentials::isolated(home, env);
    let ident = AccountIdentity::vendor_default("exa", "exa");
    let adapter = ExaAdapter::from_account(
        &creds,
        ident,
        &crate::accounts::Pointer::Env("EXA_API_KEY".into()),
    );
    match adapter.fetch().await {
        ProviderStatus::NotConfigured { hint } => {
            assert!(hint.contains("cookie") || hint.contains("exa.json"), "{hint}");
        }
        other => panic!("{other:?}"),
    }
}
