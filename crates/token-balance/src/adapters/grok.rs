use crate::accounts::Pointer;
use crate::adapters::{json_f64, pointer_secret};
use crate::credentials::Credentials;
use crate::domain::{CreditUnit, ExtraCredits, ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use chrono::{DateTime, Utc};
use serde_json::Value;

pub const GROK_BILLING_URL: &str = "https://cli-chat-proxy.grok.com/v1/billing?format=credits";
pub const GROK_AUTH_KEYS: &[&str] = &["token", "access_token", "accessToken", "key"];

pub struct GrokAdapter {
    ident: AccountIdentity,
    token: Option<String>,
    recorded: Option<Value>,
}

impl GrokAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        Self {
            ident,
            token: pointer_secret(c, pointer, GROK_AUTH_KEYS),
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
            token: Some("redacted".into()),
            recorded: Some(json),
        }
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
    // rustls/default reqwest was 401 on this host; curl/urllib 200 with the same JWT.
    let client = reqwest::Client::builder()
        .user_agent("grok-shell")
        .use_native_tls()
        .http1_only()
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
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
        return Err(format!("HTTP {status}"));
    }
    serde_json::from_str(&text).map_err(|e| e.to_string())
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
        if self.token.is_none() && self.recorded.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "grok login".into(),
                }
            });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_grok_billing(&v) });
        }
        let token = self.token.clone().unwrap();
        Box::pin(async move {
            match get_billing(&token).await {
                Ok(v) => map_grok_billing(&v),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}
