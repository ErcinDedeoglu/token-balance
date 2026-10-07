use super::*;
use crate::domain::{ProviderStatus, WindowLabel, effective_available};
use serde_json::json;

fn devpass_lite() -> Value {
    json!({"data":{
        "label":"Dev Plan API Key",
        "usage":"0.026576975",
        "limit":null,
        "devPlan":"lite",
        "devPlanCreditsUsed":"0.026576975",
        "devPlanCreditsLimit":"87",
        "devPlanCreditsRemaining":"86.97",
        "devPlanPremiumWeeklyLimit":"10.44",
        "devPlanPremiumCreditsUsed":"0.00",
        "devPlanPremiumWeekResetsAt":null
    }})
}

#[test]
fn llmgateway_devpass_maps_monthly_and_weekly() {
    let status = map_llmgateway_key(&devpass_lite());
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("DevPass Lite"));
    assert_eq!(av.windows.len(), 2);
    let monthly = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Other(_)))
        .expect("monthly");
    assert!(
        (monthly.remaining_percent - 99.9655).abs() < 0.1,
        "{}",
        monthly.remaining_percent
    );
    assert!(monthly.resets_at.is_none());
    let weekly = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Weekly))
        .expect("weekly");
    assert_eq!(weekly.remaining_percent, 100.0);
    assert!(weekly.resets_at.is_none());
}

#[test]
fn llmgateway_weekly_reset_is_parsed() {
    let v = json!({"data":{
        "devPlan":"pro",
        "devPlanCreditsLimit":"200",
        "devPlanCreditsRemaining":"150",
        "devPlanPremiumWeeklyLimit":"40",
        "devPlanPremiumCreditsUsed":"10",
        "devPlanPremiumWeekResetsAt":"2026-08-28T12:00:00.000Z"
    }});
    let status = map_llmgateway_key(&v);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("DevPass Pro"));
    let weekly = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Weekly))
        .expect("weekly");
    assert_eq!(weekly.remaining_percent, 75.0);
    assert!(weekly.resets_at.is_some());
}

#[test]
fn llmgateway_payg_key_is_unsupported() {
    let v = json!({"data":{
        "devPlan":"none",
        "devPlanCreditsLimit":"0",
        "devPlanCreditsRemaining":"0",
        "devPlanPremiumWeeklyLimit":"0",
        "devPlanPremiumCreditsUsed":"0",
        "devPlanPremiumWeekResetsAt":null
    }});
    assert!(matches!(map_llmgateway_key(&v), ProviderStatus::Unsupported { .. }));
}

#[test]
fn llmgateway_missing_devplan_is_error() {
    assert!(matches!(
        map_llmgateway_key(&json!({"data":{}})),
        ProviderStatus::Error { .. }
    ));
}

#[tokio::test]
async fn llmgateway_recorded_fetch() {
    match LlmGatewayAdapter::with_recorded(devpass_lite()).fetch().await {
        ProviderStatus::Available { plan, windows, .. } => {
            assert_eq!(plan.as_deref(), Some("DevPass Lite"));
            assert_eq!(windows.len(), 2);
        }
        other => panic!("{other:?}"),
    }
}
