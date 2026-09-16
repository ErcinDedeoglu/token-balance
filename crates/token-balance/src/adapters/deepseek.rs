use crate::accounts::Pointer;
use crate::adapters::{http_get_json, json_f64, pointer_secret};
use crate::credentials::Credentials;
use crate::domain::{CreditUnit, ExtraCredits, LedgerKind, ProviderStatus};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use serde_json::Value;

pub const DEEPSEEK_BALANCE_URL: &str = "https://api.deepseek.com/user/balance";

pub struct DeepseekAdapter {
    ident: AccountIdentity,
    key: Option<String>,
    recorded: Option<Value>,
}

impl DeepseekAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        Self {
            ident,
            key: pointer_secret(c, pointer, &["api_key", "access_token"]),
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("deepseek", "DeepSeek"),
            key: Some("redacted".into()),
            recorded: Some(json),
        }
    }
}

pub fn map_deepseek_balance(v: &Value) -> ProviderStatus {
    let infos = v
        .get("balance_infos")
        .and_then(|x| x.as_array())
        .cloned()
        .unwrap_or_default();
    let row = infos
        .iter()
        .find(|b| {
            b.get("currency")
                .and_then(|x| x.as_str())
                .is_some_and(|c| c.eq_ignore_ascii_case("USD"))
        })
        .or_else(|| infos.first());
    let Some(row) = row else {
        return ProviderStatus::Error {
            message: "deepseek: no balance_infos".into(),
            stale: None,
        };
    };
    let Some(total) = json_f64(&row["total_balance"]) else {
        return ProviderStatus::Error {
            message: "deepseek: missing total_balance".into(),
            stale: None,
        };
    };
    let currency = row
        .get("currency")
        .and_then(|x| x.as_str())
        .unwrap_or("USD");
    let unit = if currency.eq_ignore_ascii_case("USD") {
        CreditUnit::Usd
    } else {
        CreditUnit::Unknown
    };
    ProviderStatus::Available {
        plan: None,
        windows: Vec::new(),
        extra: Some(ExtraCredits {
            label: currency.to_string(),
            remaining: total,
            unit,
            limit: None,
        }),
    }
}

impl Provider for DeepseekAdapter {
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
        Some("https://api-docs.deepseek.com/api/get-user-balance")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if self.key.is_none() && self.recorded.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "export DEEPSEEK_API_KEY".into(),
                }
            });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_deepseek_balance(&v) });
        }
        let key = self.key.clone().unwrap();
        Box::pin(async move {
            match http_get_json(
                DEEPSEEK_BALANCE_URL,
                &[("Authorization", format!("Bearer {key}"))],
            )
            .await
            {
                Ok(v) => map_deepseek_balance(&v),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}

#[cfg(test)]
#[path = "deepseek_test.rs"]
mod tests;
