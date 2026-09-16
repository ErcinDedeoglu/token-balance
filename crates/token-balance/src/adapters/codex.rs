use crate::accounts::Pointer;
use crate::adapters::codex_rpc::read_rate_limits;
use crate::adapters::json_f64;
use crate::credentials::Credentials;
use crate::domain::{
    CreditUnit, ExtraCredits, ProviderStatus, QuotaWindow, SESSION_MAX_MINS, WindowLabel,
};
use crate::providers::{AccountIdentity, FetchFuture, Provider, RefreshPolicy};
use chrono::{DateTime, TimeZone, Utc};
use serde_json::Value;
use std::path::PathBuf;
use tokio::io::BufReader;
use tokio::process::Command;

pub struct CodexAdapter {
    ident: AccountIdentity,
    configured: bool,
    recorded: Option<Value>,
    program: String,
    args: Vec<String>,
}

impl CodexAdapter {
    pub fn from_account(c: &Credentials, ident: AccountIdentity, pointer: &Pointer) -> Self {
        let configured = match pointer {
            Pointer::Env(k) => c.env(k).is_some(),
            Pointer::File(p) => {
                let path = c.resolve_path(p);
                path.is_file() || path.join("auth.json").is_file()
            }
        };
        let program = c
            .env("CODEX_BIN")
            .map(str::to_string)
            .or_else(|| std::env::var("CODEX_BIN").ok())
            .unwrap_or_else(|| "codex".into());
        Self {
            ident,
            configured,
            recorded: None,
            program,
            args: vec!["app-server".into()],
        }
    }

    #[cfg(test)]
    pub fn from_credentials(c: &Credentials) -> Self {
        Self::from_account(
            c,
            AccountIdentity::vendor_default("codex", "Codex"),
            &Pointer::File(".codex/auth.json".into()),
        )
    }

    #[cfg(test)]
    pub fn with_recorded(json: Value) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("codex", "Codex"),
            configured: true,
            recorded: Some(json),
            program: "codex".into(),
            args: vec!["app-server".into()],
        }
    }

    #[cfg(test)]
    pub fn with_command(program: String, args: Vec<String>) -> Self {
        Self {
            ident: AccountIdentity::vendor_default("codex", "Codex"),
            configured: true,
            recorded: None,
            program,
            args,
        }
    }
}

fn unix_secs(v: &Value) -> Option<DateTime<Utc>> {
    Utc.timestamp_opt(json_f64(v)? as i64, 0).single()
}

fn map_window(w: &Value) -> Option<QuotaWindow> {
    let used = json_f64(w.get("usedPercent")?)? as f32;
    let mins = json_f64(w.get("windowDurationMins")?)? as u32;
    let resets = w.get("resetsAt").and_then(unix_secs);
    let label = if mins <= SESSION_MAX_MINS {
        if mins == 300 {
            WindowLabel::FiveHour
        } else {
            WindowLabel::Other(format!("{mins}m"))
        }
    } else {
        WindowLabel::Weekly
    };
    Some(QuotaWindow::from_used_percent(
        label,
        used,
        resets,
        Some(mins),
    ))
}

fn snapshot_windows(snap: &Value) -> Vec<QuotaWindow> {
    let mut out = Vec::new();
    if let Some(p) = snap.get("primary") {
        if let Some(w) = map_window(p) {
            out.push(w);
        }
    }
    if let Some(s) = snap.get("secondary") {
        if let Some(w) = map_window(s) {
            out.push(w);
        }
    }
    out
}

pub fn map_codex_rate_limits(v: &Value) -> ProviderStatus {
    let result = v.get("result").unwrap_or(v);
    let snap = result
        .pointer("/rateLimitsByLimitId/codex")
        .or_else(|| result.get("rateLimits"))
        .cloned()
        .unwrap_or(Value::Null);
    let windows = snapshot_windows(&snap);
    if windows.is_empty() {
        return ProviderStatus::Error {
            message: "codex rateLimits: no windows".into(),
            stale: None,
        };
    }
    let extra = snap
        .get("rateLimitResetCredits")
        .and_then(json_f64)
        .map(|n| ExtraCredits {
            label: "reset credits".into(),
            remaining: n,
            unit: CreditUnit::Credits,
            limit: None,
        });
    let plan = snap
        .get("planType")
        .and_then(|x| x.as_str())
        .map(str::to_string);
    ProviderStatus::Available {
        plan,
        windows,
        extra,
    }
}

async fn invoke_app_server(program: &str, args: &[String]) -> Result<Value, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|_| {
            format!(
                "{} not found; run codex login",
                PathBuf::from(program).display()
            )
        })?;
    let stdin = child.stdin.take().ok_or("no stdin")?;
    let stdout = child.stdout.take().ok_or("no stdout")?;
    let out = read_rate_limits(BufReader::new(stdout), stdin).await;
    let _ = child.kill().await;
    out
}

impl Provider for CodexAdapter {
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
        Some("https://developers.openai.com/codex/")
    }
    fn refresh_policy(&self) -> RefreshPolicy {
        RefreshPolicy::Default
    }
    fn fetch(&self) -> FetchFuture {
        if !self.configured && self.recorded.is_none() {
            return Box::pin(async {
                ProviderStatus::NotConfigured {
                    hint: "codex login".into(),
                }
            });
        }
        if let Some(v) = self.recorded.clone() {
            return Box::pin(async move { map_codex_rate_limits(&v) });
        }
        let program = self.program.clone();
        let args = self.args.clone();
        Box::pin(async move {
            match invoke_app_server(&program, &args).await {
                Ok(v) => map_codex_rate_limits(&v),
                Err(e) => ProviderStatus::Error {
                    message: e,
                    stale: None,
                },
            }
        })
    }
}
