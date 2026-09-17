use crate::accounts::Pointer;
use crate::adapters::json_f64;
use crate::credentials::Credentials;
use crate::domain::{ProviderStatus, QuotaWindow, WindowLabel};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use std::time::Duration;
use chrono::{TimeZone, Utc};
use serde_json::{Value, json};

pub const MUSE_WEB_GRAPHQL: &str = "https://dev.meta.ai/api/graphql/";
pub const MUSE_WEB_DOC_ID: &str = "28117303444603430";
pub const MUSE_WEB_OP: &str = "LLMDCUsageQuery";

pub struct MuseWebAdapter {
    ident: AccountIdentity,
    unofficial_url: Option<String>,
    session: Option<WebSession>,
    recorded: Option<Value>,
}

struct WebSession {
    team_id: String,
    cookie: String,
    fb_dtsg: String,
    lsd: Option<String>,
    doc_id: String,
}

impl MuseWebAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        let unofficial_url = match pointer {
            Pointer::File(p) if p.starts_with("http://") || p.starts_with("https://") => {
                Some(p.clone())
            }
            _ => None,
        };
        let session = if unofficial_url.is_none() {
            load_session(c, pointer)
        } else {
            None
        };
        Self {
            ident,
            unofficial_url,
            session,
            recorded: None,
        }
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("muse-web", "muse web"),
            unofficial_url: None,
            session: None,
            recorded: Some(json),
        }
    }
}

pub fn map_muse_web_quota(v: &Value) -> ProviderStatus {
    let usage = v
        .pointer("/data/team/subscription_quota_usage")
        .or_else(|| v.get("subscription_quota_usage"))
        .unwrap_or(v);
    let win_used = json_f64(&usage["window_weighted_used"]);
    let win_lim = json_f64(&usage["window_weighted_limit"]);
    let wk_used = json_f64(&usage["weekly_weighted_used"]);
    let wk_lim = json_f64(&usage["weekly_weighted_limit"]);
    let mut windows = Vec::new();
    if let (Some(u), Some(l)) = (win_used, win_lim) {
        if l > 0.0 {
            windows.push(QuotaWindow::from_used_percent(
                WindowLabel::FiveHour,
                ((u / l) * 100.0) as f32,
                unix_secs(&usage["window_resets_at"]),
                Some(300),
            ));
        }
    }
    if let (Some(u), Some(l)) = (wk_used, wk_lim) {
        if l > 0.0 {
            windows.push(QuotaWindow::from_used_percent(
                WindowLabel::Weekly,
                ((u / l) * 100.0) as f32,
                unix_secs(&usage["weekly_resets_at"]),
                Some(10080),
            ));
        }
    }
    if windows.is_empty() {
        return ProviderStatus::Error {
            message: "muse-web: no subscription_quota_usage".into(),
            stale: None,
        };
    }
    let plan = usage
        .get("tier")
        .and_then(|x| x.as_str())
        .map(|s| s.trim_start_matches("Muse Code ").trim().to_string())
        .filter(|s| !s.is_empty());
    ProviderStatus::Available {
        plan,
        windows,
        extra: None,
    }
}

fn unix_secs(v: &Value) -> Option<chrono::DateTime<Utc>> {
    let n = json_f64(v)?;
    let secs = if n > 1.0e12 { n / 1000.0 } else { n };
    Utc.timestamp_opt(secs as i64, 0).single()
}

fn load_session(c: &Credentials, pointer: &Pointer) -> Option<WebSession> {
    let Pointer::File(p) = pointer else {
        return None;
    };
    let v: Value = serde_json::from_str(&c.read_to_string(p)?).ok()?;
    let team_id = v.get("team_id").and_then(|x| x.as_str())?.trim().to_string();
    let cookie = v.get("cookie").and_then(|x| x.as_str()).unwrap_or("").trim();
    let fb_dtsg = v
        .get("fb_dtsg")
        .and_then(|x| x.as_str())
        .unwrap_or("")
        .trim();
    if team_id.is_empty() || cookie.is_empty() || fb_dtsg.is_empty() {
        return None;
    }
    Some(WebSession {
        team_id,
        cookie: cookie.to_string(),
        fb_dtsg: fb_dtsg.to_string(),
        lsd: v
            .get("lsd")
            .and_then(|x| x.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        doc_id: v
            .get("doc_id")
            .and_then(|x| x.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or(MUSE_WEB_DOC_ID)
            .to_string(),
    })
}

async fn fetch_graphql(s: &WebSession) -> Result<Value, String> {
    let today = Utc::now().format("%Y-%m-%d").to_string();
    let start = (Utc::now() - chrono::Duration::days(6))
        .format("%Y-%m-%d")
        .to_string();
    let variables = json!({
        "api_key_id": null,
        "end_date": today,
        "model_id": null,
        "start_date": start,
        "team_id": s.team_id,
        "timezone": "UTC",
        "__relay_internal__pv__Usage_ShouldIncludeSubscriptionQuotarelayprovider": true,
        "__relay_internal__pv__Usage_ShouldIncludeBatchMetricsrelayprovider": false,
        "__relay_internal__pv__Usage_ShouldIncludeCostMetricsrelayprovider": true,
        "__relay_internal__pv__Usage_ShouldIncludeImageMetricsrelayprovider": true,
    });
    let mut form: Vec<(String, String)> = vec![
        ("fb_dtsg".into(), s.fb_dtsg.clone()),
        ("fb_api_caller_class".into(), "RelayModern".into()),
        ("fb_api_req_friendly_name".into(), MUSE_WEB_OP.into()),
        ("server_timestamps".into(), "true".into()),
        ("doc_id".into(), s.doc_id.clone()),
        ("variables".into(), variables.to_string()),
    ];
    if let Some(lsd) = &s.lsd {
        form.push(("lsd".into(), lsd.clone()));
    }
    let mut req = reqwest::Client::new()
        .post(MUSE_WEB_GRAPHQL)
        .header("Cookie", &s.cookie)
        .header("Origin", "https://dev.meta.ai")
        .header("Referer", "https://dev.meta.ai/usage/")
        .header("x-fb-friendly-name", MUSE_WEB_OP)
        .header(
            "User-Agent",
            "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36",
        );
    if let Some(lsd) = &s.lsd {
        req = req.header("x-fb-lsd", lsd);
    }
    let resp = req.form(&form).send().await.map_err(|e| e.to_string())?;
    let status = resp.status();
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    if v.get("errors").is_some() && v.pointer("/data/team/subscription_quota_usage").is_none() {
        let msg = v
            .get("errors")
            .and_then(|e| e.as_array())
            .and_then(|a| a.first())
            .and_then(|e| e.get("message"))
            .and_then(|m| m.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("GraphQL error");
        let msg: String = msg.chars().take(80).collect();
        return Err(format!("muse-web: {msg}"));
    }
    Ok(v)
}

async fn fetch_unofficial(url: &str) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let v: Value = resp.json().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(format!("HTTP {status}"));
    }
    Ok(v)
}

impl Provider for MuseWebAdapter {
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
        Some("https://dev.meta.ai/usage/")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Interval(Duration::from_secs(180))
    }
    fn fetch(&self) -> FetchFuture {
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_muse_web_quota(&v) });
        }
        if let Some(url) = self.unofficial_url.clone() {
            return Box::pin(async move {
                match fetch_unofficial(&url).await {
                    Ok(v) => map_muse_web_quota(&v),
                    Err(e) => ProviderStatus::Error {
                        message: e,
                        stale: None,
                    },
                }
            });
        }
        let Some(session) = self.session.as_ref() else {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "muse-web.json needs team_id, cookie, fb_dtsg".into(),
                }
            });
        };
        let session = WebSession {
            team_id: session.team_id.clone(),
            cookie: session.cookie.clone(),
            fb_dtsg: session.fb_dtsg.clone(),
            lsd: session.lsd.clone(),
            doc_id: session.doc_id.clone(),
        };
        Box::pin(async move {
            match fetch_graphql(&session).await {
                Ok(v) => map_muse_web_quota(&v),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}

#[cfg(test)]
#[path = "muse_web_test.rs"]
mod tests;
