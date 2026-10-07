use super::*;
use crate::domain::{ProviderStatus, WindowLabel, effective_available};
use serde_json::json;

fn usage_fixture() -> Value {
    json!({"ResponseMetadata":{"Action":"GetCodingPlanUsage"},"Result":{
        "Status":"Running","UpdateTimestamp":1791357800,
        "QuotaUsage":[
            {"Level":"session","Percent":100.0,"ResetTimestamp":1791362112,"Cap":100,"RewardTotalPercent":0},
            {"Level":"weekly","Percent":33.6126036,"ResetTimestamp":1791734400,"Cap":100,"RewardTotalPercent":0},
            {"Level":"monthly","Percent":28.76202585,"ResetTimestamp":1793721599,"Cap":100,"RewardTotalPercent":0}
        ],"HasReward":false}})
}

fn subscribe_fixture(tier: &str) -> Value {
    json!({"ResponseMetadata":{"Action":"ListSubscribeTrade"},"Result":{"InfoList":[{
        "ResourceType":"CodingPlan","BizInfo":tier,"Status":"Running",
        "StartTime":"2026-10-03T03:30:08Z","EndTime":"2026-11-03T15:59:59Z","Period":"monthly"}]}})
}

#[test]
fn byteplus_maps_session_weekly_monthly() {
    let status = map_byteplus(&usage_fixture(), Some(&subscribe_fixture("pro")));
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("Coding Plan Pro"));
    assert_eq!(av.windows.len(), 3);
    let session = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::FiveHour))
        .expect("session");
    assert_eq!(session.remaining_percent, 100.0);
    assert!(session.resets_at.is_some());
    let weekly = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Weekly))
        .expect("weekly");
    assert!((weekly.remaining_percent - 33.6126036).abs() < 0.01);
    let monthly = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Other(_)))
        .expect("monthly");
    assert!((monthly.remaining_percent - 28.76202585).abs() < 0.01);
}

#[test]
fn byteplus_missing_quota_is_error() {
    match map_byteplus(
        &json!({"ResponseMetadata":{"Error":{"Code":"InvalidCSRFToken"}}}),
        None,
    ) {
        ProviderStatus::Error { message, .. } => assert!(message.contains("QuotaUsage"), "{message}"),
        other => panic!("{other:?}"),
    }
}

#[test]
fn byteplus_csrf_from_cookie() {
    assert_eq!(
        csrf_from_cookie("a=1; csrfToken=deadbeef; b=2").as_deref(),
        Some("deadbeef")
    );
    assert_eq!(csrf_from_cookie("a=1; b=2"), None);
}

#[tokio::test]
async fn byteplus_recorded_fetch() {
    let a = ByteplusAdapter::with_recorded(usage_fixture(), subscribe_fixture("lite"));
    match a.fetch().await {
        ProviderStatus::Available { plan, windows, .. } => {
            assert_eq!(plan.as_deref(), Some("Coding Plan Lite"));
            assert_eq!(windows.len(), 3);
        }
        other => panic!("{other:?}"),
    }
}
