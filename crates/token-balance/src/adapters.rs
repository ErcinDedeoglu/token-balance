mod claude;
mod codex;
mod codex_rpc;
mod copilot;
mod deepseek;
mod exa;
mod fal;
mod firecrawl;
mod grok;
mod kiro;
mod kimi;
mod muse;
mod muse_web;
mod zai;

pub use claude::ClaudeAdapter;
pub use codex::CodexAdapter;
pub use copilot::CopilotAdapter;
pub use deepseek::DeepseekAdapter;
pub use exa::ExaAdapter;
pub use fal::FalAdapter;
pub use firecrawl::FirecrawlAdapter;
pub use grok::GrokAdapter;
pub use kiro::KiroAdapter;
pub use kimi::KimiAdapter;
pub use muse::MuseAdapter;
pub use muse_web::MuseWebAdapter;
pub use zai::ZaiAdapter;

#[cfg(test)]
pub use claude::{CLAUDE_BETA, CLAUDE_USAGE_URL, map_claude_usage};
#[cfg(test)]
pub use deepseek::map_deepseek_balance;
#[cfg(test)]
pub use codex::map_codex_rate_limits;
#[cfg(test)]
pub use grok::{GROK_AUTH_KEYS, GROK_BILLING_URL, map_grok_billing};
#[cfg(test)]
pub use kiro::map_kiro_usage;
#[cfg(test)]
pub use kimi::{KIMI_USAGES_URL, map_kimi_usages};
#[cfg(test)]
pub use muse::{MUSE_RESPONSES_URL, map_muse_sse, token_from_keychain_blob};
#[cfg(test)]
pub use zai::{ZAI_QUOTA_URL, map_zai_quota};

use crate::accounts::{AccountRow, Pointer, Vendor, load_rows};
use crate::credentials::{Credentials, json_field};
use crate::domain::Clock;
use crate::fixtures::{FixtureSet, fixture_registry};
use crate::providers::{AccountIdentity, Provider};
use serde_json::{Value, json};
use std::sync::Arc;

pub fn open_registry(
    fixture: Option<FixtureSet>,
    creds: &Credentials,
    muse_on_demand: bool,
    clock: Arc<dyn Clock>,
) -> Result<Vec<Arc<dyn Provider>>, String> {
    match fixture {
        Some(set) => Ok(fixture_registry(set, clock)),
        None => live_registry(creds, muse_on_demand),
    }
}

pub fn live_registry(
    creds: &Credentials,
    _muse_on_demand: bool,
) -> Result<Vec<Arc<dyn Provider>>, String> {
    let Some(rows) = load_rows(creds)? else {
        return Ok(Vec::new());
    };
    Ok(rows
        .into_iter()
        .map(|row| adapter_from_row(creds, row))
        .collect())
}

fn ident(row: &AccountRow) -> AccountIdentity {
    AccountIdentity {
        id: row.id.clone(),
        label: row.label.clone(),
        vendor: row.vendor.as_str(),
    }
}

fn adapter_from_row(creds: &Credentials, row: AccountRow) -> Arc<dyn Provider> {
    let id = ident(&row);
    match row.vendor {
        Vendor::Claude => Arc::new(ClaudeAdapter::from_account(creds, id, &row.pointer)),
        Vendor::Codex => Arc::new(CodexAdapter::from_account(creds, id, &row.pointer)),
        Vendor::Kimi => Arc::new(KimiAdapter::from_account(creds, id, &row.pointer)),
        Vendor::Grok => Arc::new(GrokAdapter::from_account(creds, id, &row.pointer)),
        Vendor::Zai => Arc::new(ZaiAdapter::from_account(creds, id, &row.pointer)),
        Vendor::Kiro => Arc::new(KiroAdapter::from_account(creds, id, &row.pointer)),
        Vendor::Deepseek => Arc::new(DeepseekAdapter::from_account(creds, id, &row.pointer)),
        Vendor::Muse => Arc::new(MuseAdapter::from_account(creds, id, &row.pointer, true)),
        Vendor::MuseWeb => Arc::new(MuseWebAdapter::from_account(creds, id, &row.pointer)),
        Vendor::Fal => Arc::new(FalAdapter::from_account(creds, id, &row.pointer)),
        Vendor::Copilot => Arc::new(CopilotAdapter::from_account(creds, id, &row.pointer)),
        Vendor::Exa => Arc::new(ExaAdapter::from_account(creds, id, &row.pointer)),
        Vendor::Firecrawl => Arc::new(FirecrawlAdapter::from_account(creds, id, &row.pointer)),
    }
}

pub fn pointer_secret(creds: &Credentials, pointer: &Pointer, json_keys: &[&str]) -> Option<String> {
    match pointer {
        Pointer::Env(k) => creds.env(k).map(str::to_string),
        Pointer::File(p) => {
            let text = creds.read_to_string(p)?;
            json_field(&text, json_keys).or_else(|| {
                let t = text.trim();
                if t.is_empty() || t.starts_with('{') {
                    None
                } else {
                    Some(t.to_string())
                }
            })
        }
    }
}

pub fn json_f64(v: &Value) -> Option<f64> {
    v.as_f64()
        .or_else(|| v.as_i64().map(|i| i as f64))
        .or_else(|| v.as_u64().map(|i| i as f64))
        .or_else(|| v.as_str()?.parse().ok())
}

pub async fn http_get_json(url: &str, headers: &[(&str, String)]) -> Result<Value, String> {
    let mut req = reqwest::Client::new().get(url);
    for (k, v) in headers {
        req = req.header(*k, v);
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    resp.json().await.map_err(|e| e.to_string())
}

pub async fn http_post_json(
    url: &str,
    headers: &[(&str, String)],
    body: Value,
) -> Result<Value, String> {
    let text = http_post_text(url, headers, body).await?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

pub async fn http_post_amz_json(
    url: &str,
    target: &str,
    bearer: &str,
    body: Value,
) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {bearer}"))
        .header("Content-Type", "application/x-amz-json-1.0")
        .header("x-amz-target", target)
        .body(body.to_string())
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    let v: Value = serde_json::from_str(&text).unwrap_or_else(|_| json!({"message": text}));
    if !status.is_success() {
        let msg = v
            .get("message")
            .and_then(|x| x.as_str())
            .unwrap_or(status.as_str());
        return Err(format!("HTTP {status} {msg}"));
    }
    Ok(v)
}

pub async fn http_post_text(
    url: &str,
    headers: &[(&str, String)],
    body: Value,
) -> Result<String, String> {
    let mut req = reqwest::Client::new().post(url).json(&body);
    for (k, v) in headers {
        req = req.header(*k, v);
    }
    let resp = req.send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    Ok(text)
}

#[cfg(test)]
#[path = "adapters_test.rs"]
mod tests;

#[cfg(test)]
#[path = "live_registry_test.rs"]
mod live_registry_tests;
