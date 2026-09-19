use crate::accounts::Pointer;
use crate::adapters::{json_f64, pointer_secret};
use crate::credentials::{Credentials, json_field};
use crate::domain::{ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::{Value, json};

pub const MUSE_RESPONSES_URL: &str = "https://api.meta.ai/v1/responses";
pub const MUSE_KEYCHAIN_SERVICE: &str = "ai.meta.dev.credentials";
pub const MUSE_KEYCHAIN_ACCOUNT: &str = "meta";
const MUSE_TOKEN_KEYS: &[&str] = &["api_key", "access_token", "token"];

fn muse_token(c: &Credentials, pointer: &Pointer) -> Option<String> {
    if let Some(t) = pointer_secret(c, pointer, MUSE_TOKEN_KEYS) {
        return Some(t);
    }
    let Pointer::File(p) = pointer else {
        return None;
    };
    let text = c.read_to_string(p)?;
    let v: Value = serde_json::from_str(&text).ok()?;
    if v.pointer("/providers/meta/storage").and_then(|x| x.as_str()) != Some("keychain") {
        return None;
    }
    macos_keychain_blob(MUSE_KEYCHAIN_SERVICE, MUSE_KEYCHAIN_ACCOUNT)
        .and_then(|blob| token_from_keychain_blob(&blob))
}

pub fn token_from_keychain_blob(blob: &str) -> Option<String> {
    json_field(blob, MUSE_TOKEN_KEYS).or_else(|| {
        let t = blob.trim();
        (!t.is_empty() && !t.starts_with('{')).then(|| t.to_string())
    })
}

fn macos_keychain_blob(service: &str, account: &str) -> Option<String> {
    #[cfg(target_os = "macos")]
    {
        let out = std::process::Command::new("security")
            .args(["find-generic-password", "-s", service, "-a", account, "-w"])
            .output()
            .ok()?;
        if !out.status.success() {
            return None;
        }
        String::from_utf8(out.stdout).ok()
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (service, account);
        None
    }
}

pub struct MuseAdapter {
    ident: AccountIdentity,
    on_demand: bool,
    token: Option<String>,
    payg_only: bool,
    recorded: Option<String>,
}

impl MuseAdapter {
    pub fn from_account(
        c: &Credentials,
        ident: AccountIdentity,
        pointer: &Pointer,
        on_demand: bool,
    ) -> Self {
        Self { ident, on_demand, token: muse_token(c, pointer), payg_only: false, recorded: None }
    }

    #[cfg(test)]
    pub fn with_recorded(sse: String) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("muse", "Muse"),
            on_demand: true,
            token: Some("redacted".into()),
            payg_only: false,
            recorded: Some(sse),
        }
    }

    #[cfg(test)]
    pub fn unsupported() -> Self {
        Self {
            ident: AccountIdentity::vendor_default("muse", "Muse"),
            on_demand: false,
            token: None,
            payg_only: false,
            recorded: None,
        }
    }
}

pub fn map_muse_sse(text: &str) -> ProviderStatus {
    let mut payload: Option<Value> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("data:") {
            if let Ok(v) = serde_json::from_str::<Value>(rest.trim()) {
                let ty = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
                if ty == "response.subscription_usage" || v.get("subscription_usage").is_some() {
                    payload = Some(v);
                }
            }
        }
    }
    let Some(v) = payload else {
        return ProviderStatus::Error {
            message: "muse: no subscription_usage".into(),
            stale: None,
        };
    };
    let root = v
        .get("subscription_usage")
        .or_else(|| v.get("subscription"))
        .unwrap_or(&v);
    if root.get("weekly").is_some() || root.get("window").is_some() {
        let mut windows = Vec::new();
        if let Some(w) = muse_limit_window(root.get("window"), None) {
            windows.push(w);
        }
        if let Some(w) = muse_limit_window(root.get("weekly"), Some(10080)) {
            windows.push(w);
        }
        if windows.is_empty() {
            return ProviderStatus::Error {
                message: "muse: missing percent".into(),
                stale: None,
            };
        }
        return ProviderStatus::Available {
            plan: Some("Everyday".into()),
            windows,
            extra: None,
        };
    }
    let used = json_f64(&root["used_percent"])
        .or_else(|| json_f64(&root["usage_percent"]))
        .or_else(|| json_f64(&root["remaining_percent"]).map(|r| 100.0 - r));
    let Some(used) = used else {
        return ProviderStatus::Error {
            message: "muse: missing percent".into(),
            stale: None,
        };
    };
    let mins = json_f64(&root["window_duration_mins"]).unwrap_or(300.0) as u32;
    let resets = muse_resets(&root["resets_at"]);
    let label = if mins <= 360 {
        WindowLabel::FiveHour
    } else {
        WindowLabel::Weekly
    };
    ProviderStatus::Available {
        plan: Some("Everyday".into()),
        windows: vec![QuotaWindow::from_used_percent(
            label,
            used as f32,
            resets,
            Some(mins),
        )],
        extra: None,
    }
}

fn muse_limit_window(node: Option<&Value>, default_mins: Option<u32>) -> Option<QuotaWindow> {
    let w = node?;
    let used = json_f64(&w["used_percent"])?;
    let mins = json_f64(&w["window_duration_mins"])
        .map(|n| n as u32)
        .or(default_mins)
        .unwrap_or(300);
    let label = if mins <= 360 {
        WindowLabel::FiveHour
    } else {
        WindowLabel::Weekly
    };
    Some(QuotaWindow::from_used_percent(
        label,
        used as f32,
        muse_resets(&w["resets_at"]),
        Some(mins),
    ))
}

fn muse_resets(v: &Value) -> Option<DateTime<Utc>> {
    if let Some(s) = v.as_str() {
        return DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|d| d.with_timezone(&Utc));
    }
    let n = json_f64(v)?;
    let secs = if n > 1.0e12 { n / 1000.0 } else { n };
    Utc.timestamp_opt(secs as i64, 0).single()
}

pub fn map_muse_http(status: u16, text: &str) -> ProviderStatus {
    if status == 429 {
        return map_muse_exhausted(text).unwrap_or(ProviderStatus::Error {
            message: "HTTP 429 Too Many Requests".into(),
            stale: None,
        });
    }
    if !(200..300).contains(&status) {
        return ProviderStatus::Error {
            message: format!("HTTP {status}"),
            stale: None,
        };
    }
    map_muse_sse(text)
}

fn map_muse_exhausted(text: &str) -> Option<ProviderStatus> {
    let v: Value = serde_json::from_str(text.trim()).ok()?;
    let err = v.get("error").unwrap_or(&v);
    let resets = muse_resets(&err["resets_at"]).or_else(|| muse_resets(&v["resets_at"]));
    let code = err.get("code").and_then(|x| x.as_str()).unwrap_or("");
    let msg = err.get("message").and_then(|x| x.as_str()).unwrap_or("");
    let low = msg.to_ascii_lowercase();
    let exhausted = resets.is_some()
        || (code == "rate_limit_exceeded" && low.contains("quota"))
        || low.contains("quota exhausted");
    exhausted.then(|| ProviderStatus::Available {
        plan: Some("Everyday".into()),
        windows: vec![QuotaWindow::from_used_percent(
            WindowLabel::FiveHour,
            100.0,
            resets,
            Some(300),
        )],
        extra: None,
    })
}

async fn post_responses(token: &str) -> Result<(u16, String), String> {
    let resp = reqwest::Client::new()
        .post(MUSE_RESPONSES_URL)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .header("Accept", "text/event-stream")
        .json(&json!({
            "model": "muse-spark-1.3",
            "input": ".",
            "stream": true,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    Ok((status, text))
}

impl Provider for MuseAdapter {
    fn id(&self) -> &str { &self.ident.id }
    fn display_name(&self) -> &str { &self.ident.label }
    fn vendor(&self) -> &str { self.ident.vendor }
    fn docs_url(&self) -> Option<&'static str> {
        Some("https://ai.developer.meta.com/docs/muse-code/subscriptions/")
    }
    fn refresh_policy(&self) -> RefreshPolicy { RefreshPolicy::OnDemand }
    fn fetch(&self) -> FetchFuture {
        if !self.on_demand {
            return Box::pin(async {
                ProviderStatus::Unsupported {
                    reason: "no remaining API; polling burns Everyday requests".into(),
                }
            });
        }
        if self.payg_only {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "META_API_KEY is PAYG, not subscription remaining".into(),
                }
            });
        }
        if let Some(sse) = self.recorded.clone() {
            return Box::pin(async move { map_muse_sse(&sse) });
        }
        let Some(token) = self.token.clone() else {
            return Box::pin(async {
                ProviderStatus::NotConfigured { hint: "muse login".into() }
            });
        };
        Box::pin(async move {
            match post_responses(&token).await {
                Ok((status, text)) => map_muse_http(status, &text),
                Err(e) => ProviderStatus::Error { message: e, stale: None },
            }
        })
    }
}

#[cfg(test)]
#[path = "muse_test.rs"]
mod tests;
