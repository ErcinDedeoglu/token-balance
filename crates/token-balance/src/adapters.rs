mod claude;
mod codex;
mod codex_rpc;
mod grok;
mod kimi;
mod muse;
mod zai;

pub use claude::ClaudeAdapter;
pub use codex::CodexAdapter;
pub use grok::GrokAdapter;
pub use kimi::KimiAdapter;
pub use muse::MuseAdapter;
pub use zai::ZaiAdapter;

#[cfg(test)]
pub use claude::{CLAUDE_BETA, CLAUDE_USAGE_URL, map_claude_usage};
#[cfg(test)]
pub use codex::map_codex_rate_limits;
#[cfg(test)]
pub use grok::{GROK_BILLING_URL, map_grok_billing};
#[cfg(test)]
pub use kimi::{KIMI_USAGES_URL, map_kimi_usages};
#[cfg(test)]
pub use muse::{MUSE_RESPONSES_URL, map_muse_sse};
#[cfg(test)]
pub use zai::{ZAI_QUOTA_URL, map_zai_quota};

use crate::credentials::Credentials;
use crate::providers::Provider;
use serde_json::Value;
use std::sync::Arc;

pub fn live_registry(creds: &Credentials, muse_on_demand: bool) -> Vec<Arc<dyn Provider>> {
    vec![
        Arc::new(CodexAdapter::from_credentials(creds)),
        Arc::new(GrokAdapter::from_credentials(creds)),
        Arc::new(ClaudeAdapter::from_credentials(creds)),
        Arc::new(KimiAdapter::from_credentials(creds)),
        Arc::new(ZaiAdapter::from_credentials(creds)),
        Arc::new(MuseAdapter::from_credentials(creds, muse_on_demand)),
    ]
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
