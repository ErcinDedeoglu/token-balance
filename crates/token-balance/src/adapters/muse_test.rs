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

const OPENAI_RATE_LIMIT: &str = r#"{"error":{"message":"Rate limit reached for muse-spark-1.3 on requests. Please try again later.","type":"requests","param":null,"code":"rate_limit_exceeded"}}"#;

const LIVE_STREAM: &str = r#"event: response.created
data: {"type":"response.created","sequence_number":0,"response":{"id":"resp_x","status":"in_progress"}}

event: response.output_text.delta
data: {"type":"response.output_text.delta","sequence_number":11,"delta":"hi"}

event: response.subscription_usage
data: {"subscription":{"tier":"27681631238169137","weekly":{"resets_at":1791158400,"used_percent":36},"window":{"resets_at":1791115751,"used_percent":0,"window_duration_mins":300}},"type":"response.subscription_usage"}

data: [DONE]
"#;

#[test]
fn muse_429_openai_style_body_is_exhausted_everyday() {
    let status = map_muse_http(429, OPENAI_RATE_LIMIT);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("Everyday"));
    assert_eq!(av.windows.len(), 1);
    assert!(matches!(av.windows[0].label, WindowLabel::FiveHour));
    assert_eq!(av.windows[0].remaining_percent, 0.0);
    assert!(av.windows[0].resets_at.is_none());
}

#[test]
fn muse_429_finds_nested_reset() {
    let body = r#"{"error":{"code":"rate_limit_exceeded","message":"quota exhausted"},"meta":{"resets_at":1789344000}}"#;
    let status = map_muse_http(429, body);
    let av = effective_available(&status).expect("available");
    assert!(av.windows[0].resets_at.is_some());
}

#[test]
fn muse_live_stream_maps_window_and_weekly() {
    let status = map_muse_sse(LIVE_STREAM);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("Everyday"));
    assert_eq!(av.windows.len(), 2);
    let five = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::FiveHour))
        .expect("5h");
    assert_eq!(five.remaining_percent, 100.0);
    let weekly = av
        .windows
        .iter()
        .find(|w| matches!(w.label, WindowLabel::Weekly))
        .expect("weekly");
    assert!((weekly.remaining_percent - 64.0).abs() < 0.01);
}
