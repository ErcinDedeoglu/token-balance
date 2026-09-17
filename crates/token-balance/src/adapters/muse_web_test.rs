use super::*;
use crate::domain::{WindowLabel, effective_available};

#[test]
fn muse_web_maps_dashboard_quota() {
    let v: Value = serde_json::from_str(
        r#"{
          "data": {
            "team": {
              "subscription_quota_usage": {
                "tier": "Muse Code Power Usage",
                "as_of": 1789581602,
                "window_weighted_used": "0",
                "window_weighted_limit": "400000000000",
                "window_resets_at": 1789590893,
                "weekly_weighted_used": "427337973580",
                "weekly_weighted_limit": "1200000000000",
                "weekly_resets_at": 1789948800
              }
            }
          }
        }"#,
    )
    .unwrap();
    let status = map_muse_web_quota(&v);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("Power Usage"));
    let fh = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::FiveHour))
        .unwrap();
    assert_eq!(fh.used_percent, 0.0);
    assert_eq!(fh.remaining_percent, 100.0);
    let wk = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Weekly))
        .unwrap();
    assert!((wk.used_percent - 35.611).abs() < 0.02, "{}", wk.used_percent);
    assert!((wk.remaining_percent - 64.389).abs() < 0.02);
}

#[tokio::test]
async fn muse_web_recorded_fetch() {
    let v = json!({
        "subscription_quota_usage": {
            "tier": "Muse Code Everyday Usage",
            "window_weighted_used": "10",
            "window_weighted_limit": "100",
            "weekly_weighted_used": "20",
            "weekly_weighted_limit": "100"
        }
    });
    match MuseWebAdapter::with_recorded(v).fetch().await {
        crate::domain::ProviderStatus::Available { plan, windows, .. } => {
            assert_eq!(plan.as_deref(), Some("Everyday Usage"));
            assert_eq!(windows.len(), 2);
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn muse_web_missing_session_is_not_configured() {
    let home = std::env::temp_dir().join(format!(
        "tb-muse-web-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).unwrap();
    let creds = crate::credentials::Credentials::isolated(home, std::collections::BTreeMap::new());
    let ident = AccountIdentity::vendor_default("muse-web", "muse web");
    let adapter = MuseWebAdapter::from_account(
        &creds,
        ident,
        &crate::accounts::Pointer::File(".config/token-balance/muse-web.json".into()),
    );
    match adapter.fetch().await {
        crate::domain::ProviderStatus::NotConfigured { hint } => {
            assert!(hint.contains("muse-web.json"), "{hint}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn muse_web_missing_quota_is_error() {
    let v = json!({"errors": [{"message": "login required"}]});
    match map_muse_web_quota(&v) {
        crate::domain::ProviderStatus::Error { message, .. } => {
            assert!(message.contains("subscription_quota_usage"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn muse_web_interval_is_three_minutes() {
    let home = std::env::temp_dir().join(format!(
        "tb-muse-web-pol-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).unwrap();
    let creds = crate::credentials::Credentials::isolated(home, std::collections::BTreeMap::new());
    let ident = AccountIdentity::vendor_default("muse-web", "muse web");
    let adapter = MuseWebAdapter::from_account(
        &creds,
        ident,
        &crate::accounts::Pointer::File(".config/token-balance/muse-web.json".into()),
    );
    match adapter.refresh_policy() {
        crate::providers::RefreshPolicy::Interval(d) => assert_eq!(d.as_secs(), 180),
        other => panic!("{other:?}"),
    }
}
