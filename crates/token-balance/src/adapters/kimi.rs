use crate::adapters::{http_get_json, json_f64};
use crate::credentials::{Credentials, json_field};
use crate::domain::{ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{FetchFuture, Provider, RefreshPolicy, glyph_ascii};
use chrono::{DateTime, Utc};
use serde_json::Value;

pub const KIMI_USAGES_URL: &str = "https://api.kimi.com/coding/v1/usages";

pub struct KimiAdapter {
    key: Option<String>,
    recorded: Option<Value>,
}

impl KimiAdapter {
    pub fn from_credentials(c: &Credentials) -> Self {
        let key = c
            .env("KIMI_CODE_API_KEY")
            .or_else(|| c.env("KIMI_API_KEY"))
            .map(str::to_string)
            .or_else(|| {
                c.read_to_string(".kimi-code/credentials/kimi-code.json")
                    .and_then(|t| json_field(&t, &["api_key", "access_token"]))
            })
            .or_else(|| {
                c.read_to_string(".kimi/credentials/kimi-code.json")
                    .and_then(|t| json_field(&t, &["api_key", "access_token"]))
            });
        Self {
            key,
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            key: Some("sk-kimi-redacted".into()),
            recorded: Some(json),
        }
    }
}

fn parse_ts(v: &Value) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(v.as_str()?)
        .ok()
        .map(|d| d.with_timezone(&Utc))
}

fn window_from_detail(
    detail: &Value,
    label: WindowLabel,
    mins: Option<u32>,
) -> Option<QuotaWindow> {
    let limit = json_f64(detail.get("limit")?)?;
    let remaining = json_f64(detail.get("remaining")?)?;
    if limit <= 0.0 {
        return None;
    }
    let pct = (remaining / limit) * 100.0;
    let mut w = QuotaWindow::from_remaining_percent(
        label,
        pct as f32,
        parse_ts(&detail["resetTime"]),
        mins,
    );
    w.limit = Some(limit);
    w.remaining = Some(remaining);
    Some(w)
}

pub fn map_kimi_usages(v: &Value) -> ProviderStatus {
    let mut windows = Vec::new();
    if let Some(usage) = v.get("usage") {
        if let Some(w) = window_from_detail(usage, WindowLabel::Weekly, Some(10080)) {
            windows.push(w);
        }
    }
    if let Some(limits) = v.get("limits").and_then(|x| x.as_array()) {
        for item in limits {
            let window = &item["window"];
            let duration = json_f64(&window["duration"]).unwrap_or(0.0) as u32;
            let unit = window["timeUnit"].as_str().unwrap_or("");
            if duration == 300 && unit == "TIME_UNIT_MINUTE" {
                if let Some(detail) = item.get("detail") {
                    if let Some(w) = window_from_detail(detail, WindowLabel::FiveHour, Some(300)) {
                        windows.push(w);
                    }
                }
            }
        }
    }
    if windows.is_empty() {
        return ProviderStatus::Error {
            message: "kimi usages: no windows".into(),
            stale: None,
        };
    }
    let plan = v
        .pointer("/user/membership/level")
        .and_then(|x| x.as_str())
        .map(|s| s.trim_start_matches("LEVEL_").to_string());
    ProviderStatus::Available {
        plan,
        windows,
        extra: None,
    }
}

impl Provider for KimiAdapter {
    fn id(&self) -> &'static str {
        "kimi"
    }
    fn display_name(&self) -> &'static str {
        "Kimi"
    }
    fn glyph_ascii(&self) -> &'static str {
        glyph_ascii("kimi")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some("https://www.kimi.com/en/help/kimi-code/benefits")
    }
    fn fetch(&self) -> FetchFuture {
        if self.key.is_none() && self.recorded.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "export KIMI_API_KEY".into(),
                }
            });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_kimi_usages(&v) });
        }
        let key = self.key.clone().unwrap();
        Box::pin(async move {
            match http_get_json(
                KIMI_USAGES_URL,
                &[("Authorization", format!("Bearer {key}"))],
            )
            .await
            {
                Ok(v) => map_kimi_usages(&v),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}
