use crate::accounts::Pointer;
use crate::adapters::{http_get_json, json_f64, pointer_secret};
use crate::credentials::{Credentials, json_field};
use crate::domain::{CreditUnit, ExtraCredits, ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use chrono::{DateTime, Utc};
use serde_json::Value;

pub const CLAUDE_USAGE_URL: &str = "https://api.anthropic.com/api/oauth/usage";
pub const CLAUDE_BETA: &str = "oauth-2025-04-20";

pub struct ClaudeAdapter {
    ident: AccountIdentity,
    token: Option<String>,
    recorded: Option<Value>,
}

impl ClaudeAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        let token = match pointer {
            Pointer::Env(_) => pointer_secret(c, pointer, &["accessToken", "access_token"]),
            Pointer::File(p) => c.read_to_string(p).and_then(|t| claude_token(&t)),
        };
        Self {
            ident,
            token,
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn from_credentials(c: &Credentials) -> Self {
        Self::from_account(
            c,
            AccountIdentity::vendor_default("claude", "Claude"),
            &Pointer::File(".claude/.credentials.json".into()),
        )
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("claude", "Claude"),
            token: Some("redacted".into()),
            recorded: Some(json),
        }
    }
}

fn claude_token(text: &str) -> Option<String> {
    json_field(text, &["accessToken", "access_token", "claudeAiOauth"]).or_else(|| {
        let v: Value = serde_json::from_str(text).ok()?;
        v.pointer("/claudeAiOauth/accessToken")
            .and_then(|x| x.as_str())
            .map(str::to_string)
    })
}

fn bucket(v: &Value, label: WindowLabel, mins: u32) -> Option<QuotaWindow> {
    let used = json_f64(v.get("utilization")?)? as f32;
    let resets = v
        .get("resets_at")
        .and_then(|x| x.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    Some(QuotaWindow::from_used_percent(
        label,
        used,
        resets,
        Some(mins),
    ))
}

pub fn map_claude_usage(v: &Value) -> ProviderStatus {
    let mut windows = Vec::new();
    if let Some(fh) = v.get("five_hour") {
        if let Some(w) = bucket(fh, WindowLabel::FiveHour, 300) {
            windows.push(w);
        }
    }
    if let Some(sd) = v.get("seven_day") {
        if let Some(w) = bucket(sd, WindowLabel::Weekly, 10080) {
            windows.push(w);
        }
    }
    if windows.is_empty() {
        return ProviderStatus::Error {
            message: "claude oauth/usage: no windows".into(),
            stale: None,
        };
    }
    let extra = v.get("extra_usage").and_then(|x| {
        let enabled = x
            .get("is_enabled")
            .and_then(|e| e.as_bool())
            .unwrap_or(false);
        if !enabled {
            return None;
        }
        let limit = json_f64(&x["monthly_limit"]);
        let used = json_f64(&x["used_credits"]).unwrap_or(0.0);
        let remaining = limit.map(|l| (l - used).max(0.0)).unwrap_or(0.0);
        Some(ExtraCredits {
            label: "extra usage".into(),
            remaining,
            unit: CreditUnit::Usd,
            limit,
        })
    });
    ProviderStatus::Available {
        plan: None,
        windows,
        extra,
    }
}

impl Provider for ClaudeAdapter {
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
        Some("https://docs.anthropic.com/")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if self.token.is_none() && self.recorded.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "run claude".into(),
                }
            });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_claude_usage(&v) });
        }
        let token = self.token.clone().unwrap();
        Box::pin(async move {
            match http_get_json(
                CLAUDE_USAGE_URL,
                &[
                    ("Authorization", format!("Bearer {token}")),
                    ("anthropic-beta", CLAUDE_BETA.to_string()),
                ],
            )
            .await
            {
                Ok(v) => map_claude_usage(&v),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}
