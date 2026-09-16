use super::*;
use crate::domain::{ProviderStatus, WindowLabel, effective_available};
use serde_json::json;

fn edu_payload() -> Value {
    json!({
        "copilot_plan": "individual",
        "access_type_sku": "free_educational_quota",
        "quota_reset_date": "2026-10-01",
        "quota_snapshots": {
            "chat": {
                "entitlement": 0,
                "remaining": 0,
                "percent_remaining": 100.0,
                "unlimited": true
            },
            "completions": {
                "entitlement": 0,
                "remaining": 0,
                "percent_remaining": 100.0,
                "unlimited": true
            },
            "premium_interactions": {
                "entitlement": 200,
                "remaining": 185,
                "percent_remaining": 92.9,
                "unlimited": false,
                "overage_count": 0
            }
        }
    })
}

#[test]
fn copilot_premium_remaining_skips_unlimited() {
    let status = map_copilot_user(&edu_payload());
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("edu"));
    assert_eq!(av.windows.len(), 1);
    assert!(matches!(&av.windows[0].label, WindowLabel::Other(s) if s == "mo"));
    assert!((av.windows[0].remaining_percent - 92.9).abs() < 0.02);
    assert!((av.windows[0].used_percent - 7.1).abs() < 0.02);
}

#[test]
fn copilot_unlimited_only_is_error() {
    let v = json!({
        "copilot_plan": "individual",
        "quota_snapshots": {
            "chat": {"unlimited": true, "percent_remaining": 100.0},
            "completions": {"unlimited": true, "percent_remaining": 100.0}
        }
    });
    match map_copilot_user(&v) {
        ProviderStatus::Error { message, .. } => {
            assert!(message.contains("unlimited"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn copilot_recorded_fetch() {
    match CopilotAdapter::with_recorded(edu_payload()).fetch().await {
        ProviderStatus::Available { windows, plan, .. } => {
            assert_eq!(plan.as_deref(), Some("edu"));
            assert_eq!(windows.len(), 1);
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn copilot_missing_token_is_not_configured() {
    let home = std::env::temp_dir().join(format!(
        "tb-copilot-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&home).unwrap();
    let creds = crate::credentials::Credentials::isolated(home, std::collections::BTreeMap::new());
    let ident = AccountIdentity::vendor_default("copilot", "copilot");
    let adapter = CopilotAdapter::from_account(
        &creds,
        ident,
        &crate::accounts::Pointer::Env("GITHUB_TOKEN".into()),
    );
    match adapter.fetch().await {
        ProviderStatus::NotConfigured { hint } => {
            assert!(hint.contains("gh auth"), "{hint}");
        }
        other => panic!("{other:?}"),
    }
}
