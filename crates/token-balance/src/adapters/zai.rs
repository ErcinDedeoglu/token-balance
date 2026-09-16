use crate::adapters::{http_get_json, json_f64};
use crate::credentials::Credentials;
use crate::domain::{ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{FetchFuture, Provider, RefreshPolicy, glyph_ascii};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;

pub const ZAI_QUOTA_URL: &str = "https://api.z.ai/api/monitor/usage/quota/limit";

pub struct ZaiAdapter {
    key: Option<String>,
    recorded: Option<Value>,
}

impl ZaiAdapter {
    pub fn from_credentials(c: &Credentials) -> Self {
        Self {
            key: c.env("ZAI_API_KEY").map(str::to_string),
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            key: Some("redacted".into()),
            recorded: Some(json),
        }
    }
}

fn ms(v: &Value) -> Option<DateTime<Utc>> {
    let n = json_f64(v)? as i64;
    Utc.timestamp_millis_opt(n).single()
}

pub fn map_zai_quota(v: &Value) -> ProviderStatus {
    let data = v.get("data").unwrap_or(v);
    let Some(limits) = data.get("limits").and_then(|x| x.as_array()) else {
        return ProviderStatus::Error {
            message: "z.ai quota: missing limits".into(),
            stale: None,
        };
    };
    let mut windows = Vec::new();
    for item in limits {
        let kind = item.get("type").and_then(|x| x.as_str()).unwrap_or("");
        if kind != "TOKENS_LIMIT" {
            continue;
        }
        let unit = json_f64(&item["unit"]).unwrap_or(0.0) as u32;
        let number = json_f64(&item["number"]).unwrap_or(0.0) as u32;
        let Some(pct) = json_f64(&item["percentage"]) else {
            continue;
        };
        let resets = item.get("nextResetTime").and_then(ms);
        let (label, mins) = match (unit, number) {
            (3, 5) => (WindowLabel::FiveHour, Some(300)),
            (6, 7) => (WindowLabel::Weekly, Some(10080)),
            _ => continue,
        };
        windows.push(QuotaWindow::from_used_percent(
            label, pct as f32, resets, mins,
        ));
    }
    if windows.is_empty() {
        return ProviderStatus::Error {
            message: "z.ai quota: no session/weekly windows".into(),
            stale: None,
        };
    }
    ProviderStatus::Available {
        plan: data
            .get("level")
            .and_then(|x| x.as_str())
            .map(str::to_string),
        windows,
        extra: None,
    }
}

impl Provider for ZaiAdapter {
    fn id(&self) -> &'static str {
        "zai"
    }
    fn display_name(&self) -> &'static str {
        "z.ai"
    }
    fn glyph_ascii(&self) -> &'static str {
        glyph_ascii("zai")
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some("https://docs.z.ai/devpack/usage-policy")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if self.key.is_none() && self.recorded.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "export ZAI_API_KEY".into(),
                }
            });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_zai_quota(&v) });
        }
        let key = self.key.clone().unwrap();
        Box::pin(async move {
            match http_get_json(ZAI_QUOTA_URL, &[("Authorization", format!("Bearer {key}"))]).await
            {
                Ok(v) => map_zai_quota(&v),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}
