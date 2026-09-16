#[path = "kiro_session.rs"]
mod kiro_session;

use crate::accounts::Pointer;
use crate::adapters::json_f64;
use crate::credentials::Credentials;
use crate::domain::{ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use chrono::{DateTime, TimeZone, Utc};
use kiro_session::{KiroSession, get_usage, load_session, refresh_access};
use serde_json::Value;

pub struct KiroAdapter {
    ident: AccountIdentity,
    creds: Option<KiroSession>,
    unofficial_url: Option<String>,
    recorded: Option<Value>,
}

fn unofficial_url(pointer: &Pointer) -> Option<String> {
    match pointer {
        Pointer::File(p) if p.starts_with("http://") || p.starts_with("https://") => Some(p.clone()),
        _ => None,
    }
}

impl KiroAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        let unofficial_url = unofficial_url(pointer);
        Self {
            ident,
            creds: if unofficial_url.is_none() {
                load_session(c, pointer)
            } else {
                None
            },
            unofficial_url,
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("kiro", "Kiro"),
            creds: Some(KiroSession {
                access_token: "redacted".into(),
                refresh_token: None,
                client_id: None,
                client_secret: None,
                oidc_region: "eu-west-1".into(),
                profile_arn: "arn:aws:codewhisperer:us-east-1:1:profile/x".into(),
                api_region: "us-east-1".into(),
            }),
            unofficial_url: None,
            recorded: Some(json),
        }
    }
}

fn json_get<'a>(v: &'a Value, names: &[&str]) -> Option<&'a Value> {
    let obj = v.as_object()?;
    for n in names {
        if let Some(x) = obj.get(*n) {
            return Some(x);
        }
        for (k, val) in obj {
            if k.eq_ignore_ascii_case(n) {
                return Some(val);
            }
        }
    }
    None
}

fn json_num(v: &Value, names: &[&str]) -> Option<f64> {
    json_f64(json_get(v, names)?)
}

fn json_str<'a>(v: &'a Value, names: &[&str]) -> Option<&'a str> {
    json_get(v, names).and_then(|x| x.as_str())
}

fn bucket_kind(b: &Value) -> Option<&str> {
    json_str(b, &["resourceType", "type"])
}

fn is_credit(kind: &str) -> bool {
    kind.eq_ignore_ascii_case("CREDIT") || kind.eq_ignore_ascii_case("CREDITS")
}

fn bucket_used_limit(b: &Value) -> Option<(f64, f64)> {
    let used = json_num(
        b,
        &[
            "currentUsageWithPrecision",
            "currentUsage",
            "current_usage_with_precision",
        ],
    )?;
    let limit = json_num(
        b,
        &[
            "usageLimitWithPrecision",
            "usageLimit",
            "usage_limit_with_precision",
        ],
    )?;
    if limit <= 0.0 {
        None
    } else {
        Some((used, limit))
    }
}

fn pick_bucket(list: &[Value]) -> Option<&Value> {
    list.iter()
        .find(|b| bucket_kind(b).is_some_and(is_credit))
        .or_else(|| list.iter().find(|b| bucket_used_limit(b).is_some()))
}

fn unwrap_output(v: &Value) -> &Value {
    json_get(v, &["Output", "output", "Result", "result"]).unwrap_or(v)
}

pub fn map_kiro_usage(v: &Value) -> ProviderStatus {
    let v = unwrap_output(v);
    if let Some(msg) = json_str(v, &["message"]).filter(|s| !s.is_empty()) {
        if json_get(v, &["usageBreakdownList", "usageBreakdowns"]).is_none() {
            return ProviderStatus::Error {
                message: format!("kiro: {msg}"),
                stale: None,
            };
        }
    }
    let list = json_get(v, &["usageBreakdownList", "usageBreakdowns"])
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let Some(bucket) = pick_bucket(&list) else {
        let kinds: Vec<&str> = list.iter().filter_map(|b| bucket_kind(b)).collect();
        let keys: Vec<&str> = v
            .as_object()
            .map(|o| o.keys().map(String::as_str).collect())
            .unwrap_or_default();
        let detail = if !kinds.is_empty() {
            format!("got {}", kinds.join(", "))
        } else if !keys.is_empty() {
            format!("keys {}", keys.join(", "))
        } else {
            "empty body".into()
        };
        return ProviderStatus::Error {
            message: format!("kiro: no CREDIT usage bucket ({detail})"),
            stale: None,
        };
    };
    let Some((used, limit)) = bucket_used_limit(bucket) else {
        return ProviderStatus::Error {
            message: "kiro: missing usage/limit".into(),
            stale: None,
        };
    };
    let pct = ((used / limit) * 100.0) as f32;
    let reset_v = json_get(v, &["nextDateReset"])
        .or_else(|| json_get(bucket, &["nextDateReset", "resetDate"]));
    let resets = reset_v.and_then(|x| {
        if let Some(s) = x.as_str() {
            return DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| d.with_timezone(&Utc));
        }
        let secs = json_f64(x)?;
        let secs = if secs > 1.0e12 { secs / 1000.0 } else { secs };
        Utc.timestamp_opt(secs as i64, 0).single()
    });
    let title = json_get(v, &["subscriptionInfo"])
        .and_then(|s| json_str(s, &["subscriptionTitle"]))
        .map(|s| s.trim_start_matches("KIRO ").trim().to_string())
        .filter(|s| !s.is_empty());
    ProviderStatus::Available {
        plan: title,
        windows: vec![QuotaWindow::from_used_percent(
            WindowLabel::Other("mo".into()),
            pct,
            resets,
            None,
        )],
        extra: None,
    }
}

async fn fetch_unofficial(url: &str) -> ProviderStatus {
    let resp = match reqwest::Client::new().get(url).send().await {
        Ok(r) => r,
        Err(e) => {
            return ProviderStatus::Error {
                message: e.to_string(),
                stale: None,
            };
        }
    };
    let status = resp.status();
    let v: Value = match resp.json().await {
        Ok(v) => v,
        Err(e) => {
            return ProviderStatus::Error {
                message: e.to_string(),
                stale: None,
            };
        }
    };
    if !status.is_success() {
        let msg = json_str(&v, &["message"]).unwrap_or(status.as_str());
        return ProviderStatus::Error {
            message: format!("kiro: {msg}"),
            stale: None,
        };
    }
    map_kiro_usage(&v)
}

impl Provider for KiroAdapter {
    fn id(&self) -> &str {
        &self.ident.id
    }
    fn display_name(&self) -> &str {
        &self.ident.label
    }
    fn vendor(&self) -> &str {
        self.ident.vendor
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some("https://kiro.dev/docs/cli/terminal-ui/")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_kiro_usage(&v) });
        }
        if let Some(url) = self.unofficial_url.clone() {
            return Box::pin(async move { fetch_unofficial(&url).await });
        }
        let Some(session) = self.creds.clone() else {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "kiro-cli login".into(),
                }
            });
        };
        Box::pin(async move {
            match get_usage(&session.access_token, &session).await {
                Ok(v) => map_kiro_usage(&v),
                Err(e) if e.contains("401") || e.contains("403") => {
                    match refresh_access(&session).await {
                        Ok(token) => match get_usage(&token, &session).await {
                            Ok(v) => map_kiro_usage(&v),
                            Err(e2) => ProviderStatus::Error {
                                message: e2,
                                stale: None,
                            },
                        },
                        Err(_) => ProviderStatus::Error {
                            message: "kiro-cli login".into(),
                            stale: None,
                        },
                    }
                }
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}

#[cfg(test)]
#[path = "kiro_test.rs"]
mod tests;
