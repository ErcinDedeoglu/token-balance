use super::*;
use crate::domain::{CreditUnit, ProviderStatus, effective_available};
use serde_json::json;

#[test]
fn deepseek_usd_total_is_wallet_not_percent() {
    let v = json!({
        "is_available": true,
        "balance_infos": [{
            "currency": "USD",
            "total_balance": "12.50",
            "granted_balance": "2.50",
            "topped_up_balance": "10.00"
        }]
    });
    let status = map_deepseek_balance(&v);
    let av = effective_available(&status).expect("available");
    assert!(av.windows.is_empty());
    assert!(av.plan.is_none());
    let extra = av.extra.as_ref().unwrap();
    assert_eq!(extra.remaining, 12.5);
    assert!(matches!(extra.unit, CreditUnit::Usd));
}

#[test]
fn deepseek_prefers_usd_over_cny() {
    let v = json!({
        "is_available": true,
        "balance_infos": [
            {"currency": "CNY", "total_balance": "99.00"},
            {"currency": "USD", "total_balance": "1.25"}
        ]
    });
    let status = map_deepseek_balance(&v);
    let extra = effective_available(&status).unwrap().extra.clone().unwrap();
    assert_eq!(extra.remaining, 1.25);
}

#[tokio::test]
async fn deepseek_recorded_fetch() {
    let v = json!({
        "is_available": true,
        "balance_infos": [{"currency": "USD", "total_balance": "3"}]
    });
    match DeepseekAdapter::with_recorded(v).fetch().await {
        ProviderStatus::Available { windows, extra, .. } => {
            assert!(windows.is_empty());
            assert_eq!(extra.unwrap().remaining, 3.0);
        }
        other => panic!("{other:?}"),
    }
}
