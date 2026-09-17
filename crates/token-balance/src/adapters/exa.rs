use crate::accounts::Pointer;
use crate::adapters::{http_get_json, json_f64};
use crate::credentials::Credentials;
use crate::domain::{CreditUnit, ExtraCredits, LedgerKind, ProviderStatus};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use serde_json::Value;
use std::time::Duration;

#[path = "exa_chrome.rs"]
mod chrome;

pub const EXA_CREDITS_URL: &str = "https://dashboard.exa.ai/api/get-credits";
const CHROME_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/152.0.0.0 Safari/537.36";

/// Team Management `GET .../api-keys/{id}/usage` is spend (`total_cost_usd`), not remaining.
pub const EXA_USAGE_SPEND_NOTE: &str =
    "exa: total_cost_usd is Team Management usage spend, not remaining";

pub struct ExaAdapter {
    ident: AccountIdentity,
    unofficial_url: Option<String>,
    cookie: Option<String>,
    recorded: Option<Value>,
}

impl ExaAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        let unofficial_url = match pointer {
            Pointer::File(p) if p.starts_with("http://") || p.starts_with("https://") => {
                Some(p.clone())
            }
            _ => None,
        };
        let cookie = if unofficial_url.is_none() {
            load_cookie(c, pointer)
        } else {
            None
        };
        Self {
            ident,
            unofficial_url,
            cookie,
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("exa", "exa"),
            unofficial_url: None,
            cookie: Some("redacted".into()),
            recorded: Some(json),
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
        if let Some(s) = v.get("session_token").and_then(|x| x.as_str()) {
            let t = s.trim();
            if !t.is_empty() {
                return Some(format!("next-auth.session-token={t}"));
            }
        }
        return None;
    }
    let t = text.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
    }
}

fn tag_of(v: &Value) -> Option<&str> {
    v.get("tag")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("error").and_then(|e| e.get("tag")).and_then(|x| x.as_str()))
}

pub fn map_exa_json(v: &Value) -> ProviderStatus {
    if tag_of(v) == Some("NO_MORE_CREDITS") {
        return ProviderStatus::Error {
            message: "exa: credits exhausted (NO_MORE_CREDITS); remaining is on the Billing dashboard".into(),
            stale: None,
        };
    }
    if let Some(cents) = json_f64(&v["orbCreditsInCents"]) {
        let remaining = cents / 100.0;
        return ProviderStatus::Available {
            plan: Some("Pay as you go".into()),
            windows: Vec::new(),
            extra: Some(ExtraCredits {
                label: "USD".into(),
                remaining,
                unit: CreditUnit::Usd,
                limit: None,
            }),
        };
    }
    if v.get("total_cost_usd").is_some() {
        return ProviderStatus::Error {
            message: EXA_USAGE_SPEND_NOTE.into(),
            stale: None,
        };
    }
    if v.get("cost_breakdown").is_some() {
        return ProviderStatus::Error {
            message: EXA_USAGE_SPEND_NOTE.into(),
            stale: None,
        };
    }
    ProviderStatus::Error {
        message: "exa: no orbCreditsInCents; remaining is on https://dashboard.exa.ai/billing"
            .into(),
        stale: None,
    }
}

pub fn dashboard_http_error(status: u16) -> String {
    match status {
        429 => "exa: HTTP 429 Too Many Requests".into(),
        401 | 403 => "exa: dashboard session expired (refresh cookie in exa.json)".into(),
        n => format!("exa: HTTP {n}"),
    }
}

async fn get_credits(cookie: &str) -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .user_agent(CHROME_UA)
        .use_native_tls()
        .http1_only()
        .gzip(true)
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .get(EXA_CREDITS_URL)
        .header("Cookie", cookie)
        .header("Accept", "application/json, text/plain, */*")
        .header("Accept-Language", "en-US,en;q=0.9")
        .header("Origin", "https://dashboard.exa.ai")
        .header("Referer", "https://dashboard.exa.ai/billing")
        .header("Sec-Fetch-Dest", "empty")
        .header("Sec-Fetch-Mode", "cors")
        .header("Sec-Fetch-Site", "same-origin")
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        return Err(dashboard_http_error(status.as_u16()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

impl Provider for ExaAdapter {
    fn id(&self) -> &str {
        &self.ident.id
    }
    fn display_name(&self) -> &str {
        &self.ident.label
    }
    fn vendor(&self) -> &str {
        self.ident.vendor
    }
    fn ledger(&self) -> LedgerKind {
        LedgerKind::PrepaidWallet
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some("https://dashboard.exa.ai/billing")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Interval(Duration::from_secs(180))
    }
    fn fetch_timeout(&self) -> Duration {
        Duration::from_secs(18)
    }
    fn fetch(&self) -> FetchFuture {
        if self.recorded.is_none()
            && self.cookie.is_none()
            && self.unofficial_url.is_none()
        {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "point credentials at ~/.config/token-balance/exa.json with dashboard cookie".into(),
                }
            });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_exa_json(&v) });
        }
        if let Some(url) = self.unofficial_url.clone() {
            return Box::pin(async move {
                match http_get_json(&url, &[]).await {
                    Ok(v) => map_exa_json(&v),
                    Err(e) => ProviderStatus::Error {
                        message: e,
                        stale: None,
                    },
                }
            });
        }
        let cookie = self.cookie.clone().unwrap();
        Box::pin(async move {
            match chrome::get_credits_chrome().await {
                Ok(v) => map_exa_json(&v),
                Err(ce) => match get_credits(&cookie).await {
                    Ok(v) => map_exa_json(&v),
                    Err(he) => ProviderStatus::Error {
                        message: format!("{he} ({ce})"),
                        stale: None,
                    },
                },
            }
        })
    }
}

#[cfg(test)]
#[path = "exa_test.rs"]
mod tests;
