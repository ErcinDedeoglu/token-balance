use crate::accounts::Pointer;
use crate::adapters::pointer_secret;
use crate::credentials::Credentials;
use crate::domain::{LedgerKind, ProviderStatus};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use serde_json::Value;

const KEY_FIELDS: &[&str] = &["api_key", "key", "exa_api_key"];

/// Team Management `GET .../api-keys/{id}/usage` is spend (`total_cost_usd`), not remaining.
/// Remaining is dashboard-only: https://dashboard.exa.ai/billing
pub const EXA_USAGE_SPEND_NOTE: &str =
    "exa: total_cost_usd is Team Management usage spend, not remaining";

pub struct ExaAdapter {
    ident: AccountIdentity,
    key: Option<String>,
    recorded: Option<Value>,
}

impl ExaAdapter {
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
            ident: AccountIdentity::vendor_default("exa", "exa"),
            key: Some("redacted".into()),
            recorded: Some(json),
        }
    }
}

fn tag_of(v: &Value) -> Option<&str> {
    v.get("tag")
        .and_then(|x| x.as_str())
        .or_else(|| v.get("error").and_then(|e| e.get("tag")).and_then(|x| x.as_str()))
}

fn has_remaining_balance(v: &Value) -> bool {
    v.get("remaining").is_some()
        || v.get("remaining_balance").is_some()
        || v.get("current_balance").is_some()
        || v
            .get("credits")
            .and_then(|c| c.get("current_balance"))
            .is_some()
}

pub fn map_exa_json(v: &Value) -> ProviderStatus {
    if tag_of(v) == Some("NO_MORE_CREDITS") {
        return ProviderStatus::Error {
            message: "exa: credits exhausted (NO_MORE_CREDITS); remaining is on the Billing dashboard".into(),
            stale: None,
        };
    }
    if v.get("total_cost_usd").is_some() && !has_remaining_balance(v) {
        return ProviderStatus::Error {
            message: EXA_USAGE_SPEND_NOTE.into(),
            stale: None,
        };
    }
    if v.get("cost_breakdown").is_some() && !has_remaining_balance(v) {
        return ProviderStatus::Error {
            message: EXA_USAGE_SPEND_NOTE.into(),
            stale: None,
        };
    }
    ProviderStatus::Error {
        message: "exa: no remaining-balance field; remaining is on https://dashboard.exa.ai/billing"
            .into(),
        stale: None,
    }
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
        Some("https://exa.ai/docs/reference/billing")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if self.key.is_none() && self.recorded.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "export EXA_API_KEY or point credentials at an Exa key file".into(),
                }
            });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_exa_json(&v) });
        }
        Box::pin(async {
            ProviderStatus::Unsupported {
                reason: "exa remaining is on https://dashboard.exa.ai/billing; no remaining-balance GET"
                    .into(),
            }
        })
    }
}

#[cfg(test)]
#[path = "exa_test.rs"]
mod tests;
