use crate::accounts::Pointer;
use crate::adapters::{http_get_json, json_f64, pointer_secret};
use crate::credentials::Credentials;
use crate::domain::{CreditUnit, ExtraCredits, LedgerKind, ProviderStatus};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use serde_json::Value;

pub const FAL_BILLING_URL: &str = "https://api.fal.ai/v1/account/billing?expand=credits";
const KEY_FIELDS: &[&str] = &["api_key", "key", "fal_key"];

pub struct FalAdapter {
    ident: AccountIdentity,
    key: Option<String>,
    recorded: Option<Value>,
}

impl FalAdapter {
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
            ident: AccountIdentity::vendor_default("fal", "fal"),
            key: Some("redacted".into()),
            recorded: Some(json),
        }
    }
}

pub fn map_fal_billing(v: &Value) -> ProviderStatus {
    if let Some(err) = v.get("error") {
        let msg = err
            .get("message")
            .and_then(|x| x.as_str())
            .unwrap_or("fal billing error");
        return ProviderStatus::Error {
            message: format!("fal: {msg}"),
            stale: None,
        };
    }
    let credits = v.get("credits").unwrap_or(&Value::Null);
    let Some(remaining) = json_f64(&credits["current_balance"]) else {
        return ProviderStatus::Error {
            message: "fal: no credits.current_balance".into(),
            stale: None,
        };
    };
    let currency = credits
        .get("currency")
        .and_then(|x| x.as_str())
        .unwrap_or("USD");
    let unit = if currency.eq_ignore_ascii_case("USD") {
        CreditUnit::Usd
    } else {
        CreditUnit::Unknown
    };
    let plan = v
        .get("username")
        .and_then(|x| x.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    ProviderStatus::Available {
        plan,
        windows: Vec::new(),
        extra: Some(ExtraCredits {
            label: currency.to_string(),
            remaining,
            unit,
            limit: None,
        }),
    }
}

fn auth_value(key: &str) -> String {
    let t = key.trim();
    if t.len() >= 4 && t[..4].eq_ignore_ascii_case("key ") {
        t.to_string()
    } else {
        format!("Key {t}")
    }
}

impl Provider for FalAdapter {
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
        Some("https://fal.ai/docs/platform-apis/v1/account/billing")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if self.key.is_none() && self.recorded.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "export FAL_KEY or point credentials at a fal key file".into(),
                }
            });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_fal_billing(&v) });
        }
        let key = self.key.clone().unwrap();
        Box::pin(async move {
            match http_get_json(FAL_BILLING_URL, &[("Authorization", auth_value(&key))]).await {
                Ok(v) => map_fal_billing(&v),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}

#[cfg(test)]
#[path = "fal_test.rs"]
mod tests;
