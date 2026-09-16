use crate::adapters::{http_post_text, json_f64};
use crate::credentials::{Credentials, json_field};
use crate::domain::{ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{FetchFuture, Provider, RefreshPolicy, glyph_ascii};
use chrono::{DateTime, Utc};
use serde_json::{Value, json};

pub const MUSE_RESPONSES_URL: &str = "https://api.meta.ai/v1/responses";

pub struct MuseAdapter {
    on_demand: bool,
    token: Option<String>,
    payg_only: bool,
    recorded: Option<String>,
}

impl MuseAdapter {
    pub fn from_credentials(c: &Credentials, on_demand: bool) -> Self {
        let file = c
            .read_to_string(".config/muse/auth.json")
            .and_then(|t| json_field(&t, &["access_token", "token", "api_key"]));
        let payg = c.env("META_API_KEY").map(str::to_string);
        let payg_only = file.is_none() && payg.is_some();
        Self {
            on_demand,
            token: file,
            payg_only,
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn with_recorded(sse: String) -> Self {
        Self {
            on_demand: true,
            token: Some("redacted".into()),
            payg_only: false,
            recorded: Some(sse),
        }
    }

    #[cfg(test)]
    pub fn unsupported() -> Self {
        Self {
            on_demand: false,
            token: None,
            payg_only: false,
            recorded: None,
        }
    }
}

pub fn map_muse_sse(text: &str) -> ProviderStatus {
    let mut payload: Option<Value> = None;
    for line in text.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("data:") {
            if let Ok(v) = serde_json::from_str::<Value>(rest.trim()) {
                let ty = v.get("type").and_then(|x| x.as_str()).unwrap_or("");
                if ty == "response.subscription_usage" || v.get("subscription_usage").is_some() {
                    payload = Some(v);
                }
            }
        }
    }
    let Some(v) = payload else {
        return ProviderStatus::Error {
            message: "muse: no subscription_usage".into(),
            stale: None,
        };
    };
    let usage = v.get("subscription_usage").unwrap_or(&v);
    let used = json_f64(&usage["used_percent"])
        .or_else(|| json_f64(&usage["usage_percent"]))
        .or_else(|| json_f64(&usage["remaining_percent"]).map(|r| 100.0 - r));
    let Some(used) = used else {
        return ProviderStatus::Error {
            message: "muse: missing percent".into(),
            stale: None,
        };
    };
    let mins = json_f64(&usage["window_duration_mins"]).unwrap_or(300.0) as u32;
    let resets = usage
        .get("resets_at")
        .and_then(|x| x.as_str())
        .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
        .map(|d| d.with_timezone(&Utc));
    let label = if mins <= 360 {
        WindowLabel::FiveHour
    } else {
        WindowLabel::Weekly
    };
    ProviderStatus::Available {
        plan: Some("Everyday".into()),
        windows: vec![QuotaWindow::from_used_percent(
            label,
            used as f32,
            resets,
            Some(mins),
        )],
        extra: None,
    }
}

impl Provider for MuseAdapter {
    fn id(&self) -> &'static str {
        "muse"
    }
    fn display_name(&self) -> &'static str {
        "Muse"
    }
    fn glyph_ascii(&self) -> &'static str {
        glyph_ascii("muse")
    }
    fn docs_url(&self) -> Option<&'static str> {
        Some("https://ai.developer.meta.com/docs/muse-code/subscriptions/")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::OnDemand
    }
    fn fetch(&self) -> FetchFuture {
        if !self.on_demand {
            return Box::pin(async {
                ProviderStatus::Unsupported {
                    reason: "no remaining API; polling burns Everyday requests".into(),
                }
            });
        }
        if self.payg_only {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "META_API_KEY is PAYG, not subscription remaining".into(),
                }
            });
        }
        if let Some(sse) = self.recorded.clone() {
            return Box::pin(async move { map_muse_sse(&sse) });
        }
        if self.token.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "muse login".into(),
                }
            });
        }
        let token = self.token.clone().unwrap();
        Box::pin(async move {
            match http_post_text(
                MUSE_RESPONSES_URL,
                &[
                    ("Authorization", format!("Bearer {token}")),
                    ("Content-Type", "application/json".into()),
                ],
                json!({
                    "model": "muse-spark-1.3",
                    "input": ".",
                    "stream": true,
                }),
            )
            .await
            {
                Ok(text) => map_muse_sse(&text),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}
