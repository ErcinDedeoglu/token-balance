use crate::accounts::Pointer;
use crate::adapters::{http_get_json, json_f64, pointer_secret};
use crate::credentials::Credentials;
use crate::domain::{ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use chrono::{DateTime, Utc};
use serde_json::Value;

pub const LLMGATEWAY_KEY_URL: &str = "https://api.llmgateway.io/v1/key";

pub struct LlmGatewayAdapter {
    ident: AccountIdentity,
    key: Option<String>,
    hint: String,
    recorded: Option<Value>,
}

impl LlmGatewayAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        let hint = match pointer {
            Pointer::Env(k) => format!("export {k}"),
            Pointer::File(p) => format!("missing {p}"),
        };
        Self {
            ident,
            key: pointer_secret(c, pointer, &["api_key", "access_token", "token"]),
            hint,
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("llmgateway", "LLM Gateway"),
            key: Some("redacted".into()),
            hint: "export LLM_GATEWAY_API_KEY".into(),
            recorded: Some(json),
        }
    }
}

fn iso8601(v: &Value) -> Option<DateTime<Utc>> {
    let s = v.as_str()?;
    DateTime::parse_from_rfc3339(s)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

fn title_case(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + &chars.as_str().to_lowercase(),
        None => String::new(),
    }
}

pub fn map_llmgateway_key(v: &Value) -> ProviderStatus {
    let data = v.get("data").unwrap_or(v);
    let plan = data.get("devPlan").and_then(|x| x.as_str()).unwrap_or("");
    if plan.is_empty() {
        return ProviderStatus::Error {
            message: "llmgateway: missing devPlan".into(),
            stale: None,
        };
    }
    if plan == "none" {
        // Pay-as-you-go / BYOK keys carry no DevPass plan window. The prepaid
        // /v1/credits balance is a wallet, not a plan, so it is not a window here.
        return ProviderStatus::Unsupported {
            reason: "llmgateway: PAYG/BYOK key has no DevPass plan remaining".into(),
        };
    }
    let mut windows = Vec::new();
    // Monthly plan-cycle credits: remaining / limit. The billing-cycle reset is
    // not exposed by /v1/key, so the window carries no resets_at.
    if let (Some(limit), Some(remaining)) = (
        json_f64(&data["devPlanCreditsLimit"]),
        json_f64(&data["devPlanCreditsRemaining"]),
    ) {
        if limit > 0.0 {
            windows.push(QuotaWindow::from_remaining_percent(
                WindowLabel::Other("mo".into()),
                (remaining / limit * 100.0) as f32,
                None,
                None,
            ));
        }
    }
    // Weekly premium-model allowance: used / limit, reset by devPlanPremiumWeekResetsAt.
    if let Some(limit) = json_f64(&data["devPlanPremiumWeeklyLimit"]) {
        if limit > 0.0 {
            let used = json_f64(&data["devPlanPremiumCreditsUsed"]).unwrap_or(0.0);
            windows.push(QuotaWindow::from_used_percent(
                WindowLabel::Weekly,
                (used / limit * 100.0) as f32,
                iso8601(&data["devPlanPremiumWeekResetsAt"]),
                Some(10080),
            ));
        }
    }
    if windows.is_empty() {
        return ProviderStatus::Error {
            message: "llmgateway: no plan windows".into(),
            stale: None,
        };
    }
    ProviderStatus::Available {
        plan: Some(format!("DevPass {}", title_case(plan))),
        windows,
        extra: None,
    }
}

impl Provider for LlmGatewayAdapter {
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
        Some("https://docs.llmgateway.io/developers/devpass-usage")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if self.key.is_none() && self.recorded.is_none() {
            let hint = self.hint.clone();
            return Box::pin(async move { ProviderStatus::NotConfigured { hint } });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_llmgateway_key(&v) });
        }
        let key = self.key.clone().unwrap();
        Box::pin(async move {
            match http_get_json(LLMGATEWAY_KEY_URL, &[("Authorization", format!("Bearer {key}"))])
                .await
            {
                Ok(v) => map_llmgateway_key(&v),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}

#[cfg(test)]
#[path = "llmgateway_test.rs"]
mod tests;
