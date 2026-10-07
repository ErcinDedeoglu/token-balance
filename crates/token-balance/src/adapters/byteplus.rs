use crate::accounts::Pointer;
use crate::adapters::{http_post_json, json_f64};
use crate::credentials::Credentials;
use crate::domain::{LedgerKind, ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};

/// BytePlus ModelArk console "Coding Plan" usage. Unofficial: it is the JSON the
/// console page calls with the browser session, not a public API. The data-plane
/// API key (`/api/v3`) has no remaining endpoint, so only the console cookie works.
pub const BYTEPLUS_USAGE_URL: &str =
    "https://console.byteplus.com/api/top/ark/ap-southeast-1/2024-01-01/GetCodingPlanUsage";
pub const BYTEPLUS_SUBSCRIBE_URL: &str =
    "https://console.byteplus.com/api/top/ark/ap-southeast-1/2024-01-01/ListSubscribeTrade";

pub struct ByteplusAdapter {
    ident: AccountIdentity,
    cookie: Option<String>,
    recorded: Option<(Value, Value)>,
}

impl ByteplusAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        Self {
            ident,
            cookie: load_cookie(c, pointer),
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn with_recorded(usage: Value, subscribe: Value) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("byteplus", "BytePlus Coding Plan"),
            cookie: Some("redacted".into()),
            recorded: Some((usage, subscribe)),
        }
    }
}

fn load_cookie(c: &Credentials, pointer: &Pointer) -> Option<String> {
    let Pointer::File(p) = pointer else {
        return None;
    };
    let text = c.read_to_string(p)?;
    if let Ok(v) = serde_json::from_str::<Value>(&text) {
        if let Some(s) = v.get("cookie").and_then(|x| x.as_str()) {
            let t = s.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
        return None;
    }
    let t = text.trim();
    if t.is_empty() { None } else { Some(t.to_string()) }
}

/// The console uses double-submit CSRF: the `csrfToken` cookie must be echoed in
/// the `X-Csrf-Token` header. Without it the API returns `InvalidCSRFToken`.
fn csrf_from_cookie(cookie: &str) -> Option<String> {
    cookie.split(';').map(str::trim).find_map(|part| {
        part.strip_prefix("csrfToken=")
            .map(|v| v.trim().to_string())
            .filter(|s| !s.is_empty())
    })
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

fn reset_at(q: &Value) -> Option<DateTime<Utc>> {
    let secs = q.get("ResetTimestamp").and_then(Value::as_i64)?;
    Utc.timestamp_opt(secs, 0).single()
}

/// `Percent` is remaining (0-100) per window: session (sliding 5h), weekly
/// (Monday 00:00), monthly (subscription month). `ResetTimestamp` is epoch secs.
pub fn map_byteplus(usage: &Value, subscribe: Option<&Value>) -> ProviderStatus {
    let result = usage.get("Result").unwrap_or(usage);
    let Some(list) = result.get("QuotaUsage").and_then(Value::as_array) else {
        return ProviderStatus::Error {
            message: "byteplus: no QuotaUsage (console cookie may be stale)".into(),
            stale: None,
        };
    };
    let mut windows = Vec::new();
    for q in list {
        let level = q.get("Level").and_then(Value::as_str).unwrap_or("");
        let Some(remaining) = json_f64(&q["Percent"]) else {
            continue;
        };
        let (label, duration) = match level {
            "session" => (WindowLabel::FiveHour, Some(300)),
            "weekly" => (WindowLabel::Weekly, Some(10080)),
            "monthly" => (WindowLabel::Other("mo".into()), None),
            _ => continue,
        };
        windows.push(QuotaWindow::from_remaining_percent(
            label,
            remaining as f32,
            reset_at(q),
            duration,
        ));
    }
    if windows.is_empty() {
        return ProviderStatus::Error {
            message: "byteplus: no session/weekly/monthly windows".into(),
            stale: None,
        };
    }
    let plan = subscribe
        .and_then(plan_from_subscribe)
        .or_else(|| Some("Coding Plan".into()));
    ProviderStatus::Available {
        plan,
        windows,
        extra: None,
    }
}

fn plan_from_subscribe(v: &Value) -> Option<String> {
    let info = v.pointer("/Result/InfoList/0")?;
    if info.get("ResourceType").and_then(Value::as_str) != Some("CodingPlan") {
        return None;
    }
    let tier = info.get("BizInfo").and_then(Value::as_str).unwrap_or("");
    (!tier.is_empty()).then(|| format!("Coding Plan {}", title_case(tier)))
}

fn http_hint(e: String) -> String {
    if e.contains("401") || e.contains("403") {
        "byteplus: console session expired — recopy the Cookie header into byteplus.json".into()
    } else {
        e
    }
}

async fn fetch_live(cookie: &str) -> Result<(Value, Value), String> {
    let csrf = csrf_from_cookie(cookie).ok_or_else(|| {
        "byteplus: cookie has no csrfToken — copy the full Cookie request header".to_string()
    })?;
    let headers = [
        ("Cookie", cookie.to_string()),
        ("X-Csrf-Token", csrf),
        ("Accept", "application/json, text/plain, */*".to_string()),
        ("Origin", "https://console.byteplus.com".to_string()),
        (
            "Referer",
            "https://console.byteplus.com/ark/region:ap-southeast-1/subscription/coding-plan"
                .to_string(),
        ),
    ];
    let usage = http_post_json(BYTEPLUS_USAGE_URL, &headers, json!({}))
        .await
        .map_err(http_hint)?;
    if let Some(code) = usage
        .pointer("/ResponseMetadata/Error/Code")
        .and_then(Value::as_str)
    {
        return Err(match code {
            "InvalidCSRFToken" => "byteplus: invalid CSRF token — recopy the Cookie header".into(),
            other => format!("byteplus: {other}"),
        });
    }
    let subscribe = http_post_json(BYTEPLUS_SUBSCRIBE_URL, &headers, json!({}))
        .await
        .unwrap_or(Value::Null);
    Ok((usage, subscribe))
}

impl Provider for ByteplusAdapter {
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
        Some("https://console.byteplus.com/ark/region:ap-southeast-1/subscription/coding-plan")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if self.cookie.is_none() && self.recorded.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "point credentials at ~/.config/token-balance/byteplus.json with the console cookie".into(),
                }
            });
        }
        if let Some((usage, subscribe)) = self.recorded.clone() {
            return Box::pin(async move { map_byteplus(&usage, Some(&subscribe)) });
        }
        let cookie = self.cookie.clone().unwrap();
        Box::pin(async move {
            match fetch_live(&cookie).await {
                Ok((usage, subscribe)) => map_byteplus(&usage, Some(&subscribe)),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}

#[cfg(test)]
#[path = "byteplus_test.rs"]
mod tests;
