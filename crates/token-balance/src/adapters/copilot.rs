use crate::accounts::Pointer;
use crate::adapters::{http_get_json, json_f64, pointer_secret};
use crate::credentials::Credentials;
use crate::domain::{ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use chrono::{NaiveDate, TimeZone, Utc};
use serde_json::Value;

pub const COPILOT_USER_URL: &str = "https://api.github.com/copilot_internal/user";
const KEY_FIELDS: &[&str] = &["token", "oauth_token", "github_token", "access_token"];

pub struct CopilotAdapter {
    ident: AccountIdentity,
    token: Option<String>,
    recorded: Option<Value>,
}

impl CopilotAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        Self {
            ident,
            token: copilot_token(c, pointer),
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("copilot", "copilot"),
            token: Some("redacted".into()),
            recorded: Some(json),
        }
    }
}

fn copilot_token(c: &Credentials, pointer: &Pointer) -> Option<String> {
    if let Pointer::File(p) = pointer {
        if p == "gh" || p == ".config/gh" {
            return gh_auth_token();
        }
    }
    pointer_secret(c, pointer, KEY_FIELDS)
}

fn gh_auth_token() -> Option<String> {
    let out = std::process::Command::new("gh")
        .args(["auth", "token"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let t = String::from_utf8(out.stdout).ok()?.trim().to_string();
    if t.is_empty() { None } else { Some(t) }
}

pub fn map_copilot_user(v: &Value) -> ProviderStatus {
    let snaps = v.get("quota_snapshots").and_then(|x| x.as_object());
    let Some(snaps) = snaps else {
        return ProviderStatus::Error {
            message: "copilot: no quota_snapshots".into(),
            stale: None,
        };
    };
    let reset = parse_reset(
        v.get("quota_reset_date")
            .or_else(|| v.get("quota_reset_date_utc"))
            .unwrap_or(&Value::Null),
    );
    let mut windows = Vec::new();
    if let Some(w) = map_limited(snaps.get("premium_interactions"), "mo", reset) {
        windows.push(w);
    }
    if windows.is_empty() {
        if let Some(w) = map_limited(snaps.get("chat"), "chat", reset) {
            windows.push(w);
        }
        if let Some(w) = map_limited(snaps.get("completions"), "cmpl", reset) {
            windows.push(w);
        }
    }
    if windows.is_empty() {
        return ProviderStatus::Error {
            message: "copilot: no remaining quota (chat/completions unlimited)".into(),
            stale: None,
        };
    }
    ProviderStatus::Available {
        plan: plan_text(v),
        windows,
        extra: None,
    }
}

fn map_limited(
    snap: Option<&Value>,
    label: &str,
    reset: Option<chrono::DateTime<Utc>>,
) -> Option<QuotaWindow> {
    let snap = snap?;
    if snap.get("unlimited").and_then(|x| x.as_bool()) == Some(true) {
        return None;
    }
    let remaining_pct = json_f64(&snap["percent_remaining"]).map(|p| p as f32).or_else(|| {
        let ent = json_f64(&snap["entitlement"])?;
        if ent <= 0.0 {
            return None;
        }
        let rem = json_f64(&snap["remaining"]).or_else(|| json_f64(&snap["quota_remaining"]))?;
        Some(((rem / ent) * 100.0) as f32)
    })?;
    Some(QuotaWindow::from_remaining_percent(
        WindowLabel::Other(label.into()),
        remaining_pct,
        reset,
        None,
    ))
}

fn plan_text(v: &Value) -> Option<String> {
    let sku = v
        .get("access_type_sku")
        .and_then(|x| x.as_str())
        .unwrap_or("");
    if sku.contains("educational") {
        return Some("edu".into());
    }
    v.get("copilot_plan")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

fn parse_reset(v: &Value) -> Option<chrono::DateTime<Utc>> {
    let s = v.as_str()?.trim();
    if s.len() >= 10 {
        if let Ok(d) = NaiveDate::parse_from_str(&s[..10], "%Y-%m-%d") {
            let n = d.and_hms_opt(0, 0, 0)?;
            return Some(Utc.from_utc_datetime(&n));
        }
    }
    chrono::DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

impl Provider for CopilotAdapter {
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
        Some("https://github.com/settings/copilot")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if self.token.is_none() && self.recorded.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "gh auth login or point credentials at a GitHub token".into(),
                }
            });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_copilot_user(&v) });
        }
        let token = self.token.clone().unwrap();
        Box::pin(async move {
            match http_get_json(
                COPILOT_USER_URL,
                &[
                    ("Authorization", format!("Bearer {token}")),
                    ("Accept", "application/json".into()),
                    ("User-Agent", "token-balance".into()),
                    ("X-GitHub-Api-Version", "2025-05-01".into()),
                ],
            )
            .await
            {
                Ok(v) => map_copilot_user(&v),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}

#[cfg(test)]
#[path = "copilot_test.rs"]
mod tests;
