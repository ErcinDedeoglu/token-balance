use crate::accounts::Pointer;
use crate::adapters::{http_get_json, json_f64, pointer_secret};
use crate::credentials::Credentials;
use crate::domain::{CreditUnit, ExtraCredits, LedgerKind, ProviderStatus};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use serde_json::Value;

pub const FIRECRAWL_CREDITS_URL: &str = "https://api.firecrawl.dev/v2/team/credit-usage";
const KEY_FIELDS: &[&str] = &["api_key", "key", "firecrawl_api_key"];

pub struct FirecrawlAdapter {
    ident: AccountIdentity,
    key: Option<String>,
    recorded: Option<Value>,
}

impl FirecrawlAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        Self {
            ident,
            key: pointer_secret(c, pointer, KEY_FIELDS),
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("firecrawl", "firecrawl"),
            key: Some("redacted".into()),
            recorded: Some(json),
        }
    }
}

fn remaining_field(data: &Value) -> Option<f64> {
    json_f64(&data["remainingCredits"]).or_else(|| json_f64(&data["remaining_credits"]))
}

fn plan_credits_field(data: &Value) -> Option<f64> {
    json_f64(&data["planCredits"]).or_else(|| json_f64(&data["plan_credits"]))
}

pub fn map_firecrawl_credits(v: &Value) -> ProviderStatus {
    if v.get("success") == Some(&Value::Bool(false)) {
        let msg = v
            .get("error")
            .and_then(|e| e.as_str())
            .unwrap_or("credit usage failed");
        return ProviderStatus::Error {
            message: format!("firecrawl: {msg}"),
            stale: None,
        };
    }
    let data = v.get("data").unwrap_or(v);
    let Some(remaining) = remaining_field(data) else {
        return ProviderStatus::Error {
            message: "firecrawl: no remainingCredits".into(),
            stale: None,
        };
    };
    let limit = plan_credits_field(data).filter(|&p| remaining <= p);
    ProviderStatus::Available {
        plan: None,
        windows: Vec::new(),
        extra: Some(ExtraCredits {
            label: "credits".into(),
            remaining,
            unit: CreditUnit::Credits,
            limit,
        }),
    }
}

fn auth_value(key: &str) -> String {
    let t = key.trim();
    if t.len() >= 7 && t[..7].eq_ignore_ascii_case("bearer ") {
        t.to_string()
    } else {
        format!("Bearer {t}")
    }
}

impl Provider for FirecrawlAdapter {
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
        Some("https://docs.firecrawl.dev/api-reference/endpoint/credit-usage")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if self.key.is_none() && self.recorded.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "export FIRECRAWL_API_KEY or point credentials at a firecrawl key file"
                        .into(),
                }
            });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_firecrawl_credits(&v) });
        }
        let key = self.key.clone().unwrap();
        Box::pin(async move {
            match http_get_json(FIRECRAWL_CREDITS_URL, &[("Authorization", auth_value(&key))])
                .await
            {
                Ok(v) => map_firecrawl_credits(&v),
                Err(e) => ProviderStatus::Error {
                    message: format!("firecrawl: {e}"),
                    stale: None,
                },
            }
        })
    }
}

#[cfg(test)]
#[path = "firecrawl_test.rs"]
mod tests;
