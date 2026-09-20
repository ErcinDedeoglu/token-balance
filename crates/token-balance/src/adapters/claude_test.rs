use super::*;
use crate::domain::{ProviderStatus, WindowLabel, effective_available};
use chrono::{Datelike, TimeZone, Utc};
use serde_json::json;

#[test]
fn claude_cc_proxy_extra_only_is_monthly_not_5h() {
    let v = json!({
        "status": "ok",
        "usage": {
            "extra_usage": {
                "is_enabled": true,
                "monthly_limit": 150000.0,
                "used_credits": 96366.0
            },
            "five_hour": null,
            "seven_day": null,
            "seven_day_opus": null,
            "seven_day_sonnet": null
        }
    });
    let status = map_claude_usage(&v);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.windows.len(), 1);
    assert!(matches!(&av.windows[0].label, WindowLabel::Other(s) if s == "mo"));
    let extra = av.extra.as_ref().expect("credits");
    assert_eq!(extra.remaining, 53634.0);
    assert_eq!(extra.limit, Some(150000.0));
    assert!(matches!(extra.unit, crate::domain::CreditUnit::Usd));
    let used = 96366.0 / 150000.0 * 100.0;
    assert!((av.windows[0].used_percent as f64 - used).abs() < 0.05);
    assert!(av
        .windows
        .iter()
        .all(|w| !matches!(w.label, WindowLabel::FiveHour)));
    assert!(av.windows[0].resets_at.is_some(), "enterprise monthly reset");
}

#[test]
fn claude_cc_extra_usage_resets_at_rfc3339() {
    let v = json!({
        "usage": {
            "extra_usage": {
                "is_enabled": true,
                "monthly_limit": 2000.0,
                "used_credits": 1796.21,
                "resets_at": "2026-10-01T00:00:00Z"
            },
            "five_hour": null,
            "seven_day": null
        }
    });
    let status = map_claude_usage(&v);
    let av = effective_available(&status).expect("available");
    let at = av.windows[0].resets_at.expect("resets_at");
    assert_eq!(at, Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap());
}

#[test]
fn claude_cc_extra_usage_without_resets_at_uses_next_month() {
    let now = Utc.with_ymd_and_hms(2026, 9, 20, 9, 55, 0).unwrap();
    let v = json!({
        "usage": {
            "extra_usage": {
                "is_enabled": true,
                "monthly_limit": 2000.0,
                "used_credits": 1796.21
            },
            "five_hour": null,
            "seven_day": null
        }
    });
    let status = map_claude_usage_at(&v, now);
    let av = effective_available(&status).expect("available");
    let at = av.windows[0].resets_at.expect("fallback");
    assert_eq!(at.day(), 1);
    assert_eq!(at, Utc.with_ymd_and_hms(2026, 10, 1, 0, 0, 0).unwrap());
}

#[tokio::test]
async fn claude_cc_recorded_fetch() {
    let v = json!({
        "usage": {
            "extra_usage": {
                "is_enabled": true,
                "monthly_limit": 100.0,
                "used_credits": 25.0
            },
            "five_hour": null,
            "seven_day": null
        }
    });
    match ClaudeAdapter::with_recorded(v).fetch().await {
        ProviderStatus::Available { windows, extra, .. } => {
            assert_eq!(windows.len(), 1);
            assert_eq!(windows[0].used_percent, 25.0);
            let extra = extra.expect("credits");
            assert_eq!(extra.remaining, 75.0);
            assert_eq!(extra.limit, Some(100.0));
        }
        other => panic!("{other:?}"),
    }
}
