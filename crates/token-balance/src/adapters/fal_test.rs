use super::*;
use crate::domain::{CreditUnit, ProviderStatus, effective_available};
use serde_json::json;

#[test]
fn fal_current_balance_is_wallet_not_percent() {
    let v = json!({
        "username": "my-team",
        "credits": {"current_balance": 24.5, "currency": "USD"}
    });
    let status = map_fal_billing(&v);
    let av = effective_available(&status).expect("available");
    assert!(av.windows.is_empty());
    assert_eq!(av.plan.as_deref(), Some("my-team"));
    let extra = av.extra.as_ref().unwrap();
    assert_eq!(extra.remaining, 24.5);
    assert!(matches!(extra.unit, CreditUnit::Usd));
}

#[test]
fn fal_string_balance_and_missing_credits_error() {
    let v = json!({"credits": {"current_balance": "3.00", "currency": "USD"}});
    let extra = effective_available(&map_fal_billing(&v))
        .unwrap()
        .extra
        .clone()
        .unwrap();
    assert_eq!(extra.remaining, 3.0);
    match map_fal_billing(&json!({"username": "x"})) {
        ProviderStatus::Error { message, .. } => {
            assert!(message.contains("current_balance"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn fal_auth_prefixes_key() {
    assert_eq!(auth_value("abc"), "Key abc");
    assert_eq!(auth_value("Key already"), "Key already");
    assert_eq!(auth_value("key already"), "key already");
}

#[tokio::test]
async fn fal_recorded_fetch() {
    let v = json!({
        "username": "acct",
        "credits": {"current_balance": 1.25, "currency": "USD"}
    });
    match FalAdapter::with_recorded(v).fetch().await {
        ProviderStatus::Available { windows, extra, .. } => {
            assert!(windows.is_empty());
            assert_eq!(extra.unwrap().remaining, 1.25);
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn fal_missing_key_is_not_configured() {
    let home = std::env::temp_dir().join(format!(
        "tb-fal-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).unwrap();
    let creds = crate::credentials::Credentials::isolated(home, std::collections::BTreeMap::new());
    let ident = AccountIdentity::vendor_default("fal", "fal");
    let adapter = FalAdapter::from_account(
        &creds,
        ident,
        &crate::accounts::Pointer::Env("FAL_KEY".into()),
    );
    match adapter.fetch().await {
        ProviderStatus::NotConfigured { hint } => {
            assert!(hint.contains("FAL_KEY"), "{hint}");
        }
        other => panic!("{other:?}"),
    }
}
