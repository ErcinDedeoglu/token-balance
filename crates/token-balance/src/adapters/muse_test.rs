use super::*;
use crate::domain::{WindowLabel, effective_available};

const EXHAUSTED: &str = r#"{"error":{"code":"rate_limit_exceeded","message":"Subscription quota exhausted. Your usage window resets at 2026-09-14T00:00:00Z.","resets_at":1789344000}}"#;

#[test]
fn muse_429_with_resets_at_is_exhausted_everyday_not_dead_token() {
    let status = map_muse_http(429, EXHAUSTED);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("Everyday"));
    assert_eq!(av.windows.len(), 1);
    assert!(matches!(av.windows[0].label, WindowLabel::FiveHour));
    assert_eq!(av.windows[0].remaining_percent, 0.0);
    assert_eq!(av.windows[0].used_percent, 100.0);
    assert!(av.windows[0].resets_at.is_some());
}

#[test]
fn muse_429_without_quota_body_stays_error() {
    match map_muse_http(429, "rate limited") {
        ProviderStatus::Error { message, .. } => {
            assert!(message.contains("429"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn recorded_sse_fetch_still_maps() {
    let sse = r#"event: response.subscription_usage
data: {"type":"response.subscription_usage","subscription_usage":{"used_percent":20,"window_duration_mins":300,"resets_at":"2026-09-15T19:32:00Z"}}
"#;
    match MuseAdapter::with_recorded(sse.into()).fetch().await {
        ProviderStatus::Available { windows, .. } => {
            assert!((windows[0].remaining_percent - 80.0).abs() < 0.01);
        }
        other => panic!("{other:?}"),
    }
}
