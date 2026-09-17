use super::*;
use crate::domain::{ProviderStatus, WindowLabel, effective_available};
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
