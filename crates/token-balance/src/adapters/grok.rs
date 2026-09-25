use crate::accounts::Pointer;
use crate::adapters::{json_f64, pointer_secret};
use crate::credentials::Credentials;
use crate::domain::{CreditUnit, ExtraCredits, ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use chrono::{DateTime, Utc};
use serde_json::Value;

#[path = "grok_oidc.rs"]
mod oidc;

use oidc::{
    access_expired, grok_http, parse_grok_auth, refresh_and_store,
};

pub const GROK_BILLING_URL: &str = "https://cli-chat-proxy.grok.com/v1/billing?format=credits";
pub const GROK_AUTH_KEYS: &[&str] = &["token", "access_token", "accessToken", "key"];

pub struct GrokAdapter {
    ident: AccountIdentity,
    live: Option<(Credentials, Pointer)>,
    recorded: Option<Value>,
}

impl GrokAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        Self {
            ident,
            live: Some((c.clone(), pointer.clone())),
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn from_credentials(c: &Credentials) -> Self {
        Self::from_account(
            c,
            AccountIdentity::vendor_default("grok", "Grok"),
            &Pointer::File(".grok/auth.json".into()),
        )
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("grok", "Grok"),
            live: None,
            recorded: Some(json),
        }
    }

    #[cfg(test)]
    fn live_token(&self) -> Option<String> {
        let (c, pointer) = self.live.as_ref()?;
        pointer_secret(c, pointer, GROK_AUTH_KEYS)
    }
}

fn ts(s: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

pub fn map_grok_billing(v: &Value) -> ProviderStatus {
    let config = v.get("config").unwrap_or(v);
    let used_pct = json_f64(&config["creditUsagePercent"]).or_else(|| {
        let used = config.pointer("/onDemandUsed/val").and_then(json_f64);
        let cap = config.pointer("/onDemandCap/val").and_then(json_f64);
        match (used, cap) {
            (Some(u), Some(c)) if c > 0.0 => Some(u / c * 100.0),
            _ => None,
        }
    });
    let Some(used_pct) = used_pct else {
        return ProviderStatus::Error {
            message: "grok billing: missing creditUsagePercent".into(),
            stale: None,
        };
    };
    let end = config
        .pointer("/currentPeriod/end")
        .and_then(|x| x.as_str())
        .or_else(|| config.get("billingPeriodEnd").and_then(|x| x.as_str()))
        .and_then(ts);
    let weekly =
        QuotaWindow::from_used_percent(WindowLabel::Weekly, used_pct as f32, end, Some(10080));
    let extra = match (
        config.pointer("/onDemandCap/val").and_then(json_f64),
        config.pointer("/onDemandUsed/val").and_then(json_f64),
    ) {
        (Some(cap), used) if cap > 0.0 => {
            let used = used.unwrap_or(0.0);
            Some(ExtraCredits {
                label: "extra usage".into(),
                remaining: (cap - used).max(0.0),
                unit: CreditUnit::Credits,
                limit: Some(cap),
            })
        }
        _ => None,
    };
    ProviderStatus::Available {
        plan: Some("SuperGrok".into()),
        windows: vec![weekly],
        extra,
    }
}

async fn get_billing(token: &str) -> Result<Value, String> {
    let resp = grok_http()?
        .get(GROK_BILLING_URL)
        .header("Authorization", format!("Bearer {token}"))
        .header("X-XAI-Token-Auth", "xai-grok-cli")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(billing_http_error(status.as_u16()));
    }
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub fn billing_http_error(status: u16) -> String {
    match status {
        401 => "HTTP 401 Unauthorized — grok login".into(),
        n => format!("HTTP {n}"),
    }
}

async fn fetch_live(creds: &Credentials, pointer: &Pointer) -> ProviderStatus {
    let mut auth = match pointer {
        Pointer::File(rel) => creds
            .read_to_string(rel)
            .as_deref()
            .and_then(parse_grok_auth),
        Pointer::Env(_) => None,
    };
    if let Some(a) = auth.as_mut() {
        if access_expired(a) {
            let _ = refresh_and_store(creds, pointer, a).await;
        }
    }
    let token = match auth.as_ref().map(|a| a.access.clone()) {
        Some(t) => t,
        None => match pointer_secret(creds, pointer, GROK_AUTH_KEYS) {
            Some(t) => t,
            None => {
                return ProviderStatus::NotConfigured {
                    hint: "grok login".into(),
                };
            }
        },
    };
    match get_billing(&token).await {
        Ok(v) => map_grok_billing(&v),
        Err(e) if e.contains("401") => {
            if let Some(a) = auth.as_mut() {
                if refresh_and_store(creds, pointer, a).await.is_ok() {
                    return match get_billing(&a.access).await {
                        Ok(v) => map_grok_billing(&v),
                        Err(e2) => ProviderStatus::Error {
                            message: e2,
                            stale: None,
                        },
                    };
                }
            }
            ProviderStatus::Error {
                message: e,
                stale: None,
            }
        }
        Err(e) => ProviderStatus::Error {
            message: e,
            stale: None,
        },
    }
}

impl Provider for GrokAdapter {
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
        Some("https://grok.com")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_grok_billing(&v) });
        }
        let Some((creds, pointer)) = self.live.clone() else {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "grok login".into(),
                }
            });
        };
        Box::pin(async move { fetch_live(&creds, &pointer).await })
    }
}

#[cfg(test)]
#[path = "grok_test.rs"]
mod tests;
