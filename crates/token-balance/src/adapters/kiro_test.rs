use super::*;
use crate::accounts::Pointer;
use crate::credentials::Credentials;
use crate::domain::{ProviderStatus, WindowLabel, effective_available};
use serde_json::Value;
use std::collections::BTreeMap;
use std::path::PathBuf;

#[test]
fn kiro_credit_bucket_is_used_percent() {
    let v: Value = serde_json::from_str(
        r#"{
          "nextDateReset": 1785542400.0,
          "subscriptionInfo": { "subscriptionTitle": "KIRO POWER" },
          "usageBreakdownList": [{
            "resourceType": "CREDIT",
            "displayName": "Credit",
            "currentUsageWithPrecision": 2500.0,
            "usageLimitWithPrecision": 10000.0
          }]
        }"#,
    )
    .unwrap();
    let status = map_kiro_usage(&v);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("POWER"));
    assert!(matches!(&av.windows[0].label, WindowLabel::Other(s) if s == "mo"));
    assert_eq!(av.windows[0].used_percent, 25.0);
    assert_eq!(av.windows[0].remaining_percent, 75.0);
}

#[test]
fn kiro_agentic_request_bucket_maps() {
    let v: Value = serde_json::from_str(
        r#"{
          "nextDateReset": 1785542400.0,
          "subscriptionInfo": { "subscriptionTitle": "KIRO PRO" },
          "usageBreakdownList": [{
            "resourceType": "AGENTIC_REQUEST",
            "currentUsageWithPrecision": 12.0,
            "usageLimitWithPrecision": 50.0
          }]
        }"#,
    )
    .unwrap();
    let status = map_kiro_usage(&v);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("PRO"));
    assert_eq!(av.windows[0].used_percent, 24.0);
    assert_eq!(av.windows[0].remaining_percent, 76.0);
}

#[test]
fn kiro_pascal_case_and_short_fields() {
    let v: Value = serde_json::from_str(
        r#"{
          "NextDateReset": 1785542400.0,
          "SubscriptionInfo": { "SubscriptionTitle": "KIRO POWER" },
          "UsageBreakdownList": [{
            "Type": "CREDIT",
            "CurrentUsage": 10.0,
            "UsageLimit": 50.0
          }]
        }"#,
    )
    .unwrap();
    let status = map_kiro_usage(&v);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("POWER"));
    assert_eq!(av.windows[0].remaining_percent, 80.0);
}

#[test]
fn kiro_cc_proxy_usage_shape() {
    let v: Value = serde_json::from_str(
        r#"{
          "subscriptionInfo": {"subscriptionTitle": "KIRO POWER", "type": "Q_DEVELOPER_STANDALONE_POWER"},
          "overageConfiguration": {"overageStatus": "DISABLED"},
          "usageBreakdownList": [{
            "currentUsage": 0.0,
            "currentUsageWithPrecision": 0.02,
            "usageLimit": 10000.0,
            "usageLimitWithPrecision": 10000.0,
            "displayNamePlural": "Credits",
            "nextDateReset": 1790812800.0
          }]
        }"#,
    )
    .unwrap();
    let status = map_kiro_usage(&v);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("POWER"));
    assert!(av.windows[0].used_percent < 1.0);
    assert!(av.windows[0].remaining_percent > 99.0);
}

#[test]
fn kiro_aws_output_version_envelope() {
    let v: Value = serde_json::from_str(
        r#"{
          "Output": {
            "nextDateReset": 1785542400.0,
            "subscriptionInfo": { "subscriptionTitle": "KIRO PRO" },
            "usageBreakdownList": [{
              "resourceType": "CREDIT",
              "currentUsageWithPrecision": 20.0,
              "usageLimitWithPrecision": 100.0
            }]
          },
          "Version": "1.0"
        }"#,
    )
    .unwrap();
    let status = map_kiro_usage(&v);
    let av = effective_available(&status).expect("available");
    assert_eq!(av.plan.as_deref(), Some("PRO"));
    assert_eq!(av.windows[0].used_percent, 20.0);
    assert_eq!(av.windows[0].remaining_percent, 80.0);
}

#[test]
fn kiro_error_envelope_is_not_credit_miss() {
    let v: Value = serde_json::from_str(r#"{"message":"The bearer token included in the request is invalid.","reason":"UNAUTHENTICATED"}"#).unwrap();
    match map_kiro_usage(&v) {
        ProviderStatus::Error { message, .. } => {
            assert!(message.contains("bearer token"), "{message}");
            assert!(!message.contains("CREDIT"), "{message}");
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn kiro_unofficial_localhost_cc_proxy() {
    let ident = AccountIdentity::vendor_default("kiro", "Kiro");
    let a = KiroAdapter::from_account(
        &Credentials::isolated(PathBuf::from("/nonexistent-tb-kiro"), BTreeMap::new()),
        ident,
        &Pointer::File("http://localhost:9090/v1/usage/kiro".into()),
    );
    match a.fetch().await {
        ProviderStatus::Available { plan, windows, .. } => {
            assert_eq!(plan.as_deref(), Some("POWER"));
            assert!(windows[0].remaining_percent > 99.0, "{windows:?}");
        }
        ProviderStatus::Error { message, .. }
            if message.contains("Connection refused") || message.contains("error sending request") =>
        {
            // cc-proxy is a personal sidecar; CI has no :9090.
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn kiro_recorded_fetch() {
    let v: Value = serde_json::from_str(
        r#"{"subscriptionInfo":{"subscriptionTitle":"KIRO PRO"},"usageBreakdownList":[{"resourceType":"CREDIT","currentUsageWithPrecision":10.0,"usageLimitWithPrecision":1000.0}]}"#,
    )
    .unwrap();
    match KiroAdapter::with_recorded(v).fetch().await {
        ProviderStatus::Available { .. } => {}
        other => panic!("{other:?}"),
    }
}
