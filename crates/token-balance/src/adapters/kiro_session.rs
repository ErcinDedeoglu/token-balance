use crate::accounts::Pointer;
use crate::adapters::{http_post_amz_json, http_post_json};
use crate::credentials::Credentials;
use serde_json::{Value, json};
use std::path::Path;
use std::process::Command;

#[derive(Clone)]
pub(super) struct KiroSession {
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub client_id: Option<String>,
    pub client_secret: Option<String>,
    pub oidc_region: String,
    pub profile_arn: String,
    pub api_region: String,
}

pub(super) fn load_session(c: &Credentials, pointer: &Pointer) -> Option<KiroSession> {
    let Pointer::File(p) = pointer else {
        return env_session(c);
    };
    let path = c.resolve_path(p);
    if path.extension().and_then(|e| e.to_str()) == Some("json") {
        return json_session(&c.read_to_string(p)?);
    }
    sqlite_session(&path)
}

fn env_session(c: &Credentials) -> Option<KiroSession> {
    let token = c.env("KIRO_ACCESS_TOKEN")?.to_string();
    let arn = c.env("KIRO_PROFILE_ARN")?.to_string();
    Some(KiroSession {
        access_token: token,
        refresh_token: None,
        client_id: None,
        client_secret: None,
        oidc_region: c.env("KIRO_OIDC_REGION").unwrap_or("us-east-1").into(),
        api_region: arn_region(&arn),
        profile_arn: arn,
    })
}

fn json_session(text: &str) -> Option<KiroSession> {
    let v: Value = serde_json::from_str(text).ok()?;
    let token = v.get("token").unwrap_or(&v);
    let access = token
        .get("access_token")
        .or_else(|| token.get("accessToken"))
        .and_then(|x| x.as_str())?
        .to_string();
    let arn = v
        .pointer("/profile/arn")
        .or_else(|| v.get("profileArn"))
        .and_then(|x| x.as_str())?
        .to_string();
    let reg = v.get("registration").unwrap_or(&v);
    Some(KiroSession {
        access_token: access,
        refresh_token: token
            .get("refresh_token")
            .or_else(|| token.get("refreshToken"))
            .and_then(|x| x.as_str())
            .map(str::to_string),
        client_id: reg
            .get("client_id")
            .and_then(|x| x.as_str())
            .map(str::to_string),
        client_secret: reg
            .get("client_secret")
            .and_then(|x| x.as_str())
            .map(str::to_string),
        oidc_region: token
            .get("region")
            .or_else(|| v.get("idc_region"))
            .and_then(|x| x.as_str())
            .unwrap_or("us-east-1")
            .to_string(),
        api_region: arn_region(&arn),
        profile_arn: arn,
    })
}

fn sqlite_session(path: &Path) -> Option<KiroSession> {
    let token: Value =
        serde_json::from_str(&sqlite_kv(path, "auth_kv", "kirocli:odic:token")?).ok()?;
    let reg = sqlite_kv(path, "auth_kv", "kirocli:odic:device-registration")
        .and_then(|s| serde_json::from_str::<Value>(&s).ok());
    let prof_raw = sqlite_kv(path, "state", "api.codewhisperer.profile")?;
    let prof: Value = serde_json::from_str(&prof_raw)
        .ok()
        .and_then(|v: Value| {
            if let Some(s) = v.as_str() {
                serde_json::from_str(s).ok()
            } else {
                Some(v)
            }
        })?;
    let arn = prof.get("arn").and_then(|x| x.as_str())?.to_string();
    Some(KiroSession {
        access_token: token.get("access_token").and_then(|x| x.as_str())?.into(),
        refresh_token: token
            .get("refresh_token")
            .and_then(|x| x.as_str())
            .map(str::to_string),
        client_id: reg
            .as_ref()
            .and_then(|r| r.get("client_id"))
            .and_then(|x| x.as_str())
            .map(str::to_string),
        client_secret: reg
            .as_ref()
            .and_then(|r| r.get("client_secret"))
            .and_then(|x| x.as_str())
            .map(str::to_string),
        oidc_region: token
            .get("region")
            .and_then(|x| x.as_str())
            .unwrap_or("us-east-1")
            .to_string(),
        api_region: arn_region(&arn),
        profile_arn: arn,
    })
}

fn sqlite_kv(path: &Path, table: &str, key: &str) -> Option<String> {
    let out = Command::new("sqlite3")
        .args([
            "-readonly",
            path.to_str()?,
            &format!("SELECT value FROM {table} WHERE key='{key}';"),
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8(out.stdout).ok()?;
    let s = s.trim();
    if s.is_empty() {
        None
    } else {
        Some(s.to_string())
    }
}

fn arn_region(arn: &str) -> String {
    arn.split(':').nth(3).unwrap_or("us-east-1").to_string()
}

pub(super) async fn refresh_access(s: &KiroSession) -> Result<String, String> {
    let Some(rt) = s.refresh_token.as_ref() else {
        return Err("kiro-cli login".into());
    };
    let Some(cid) = s.client_id.as_ref() else {
        return Err("kiro-cli login".into());
    };
    let Some(sec) = s.client_secret.as_ref() else {
        return Err("kiro-cli login".into());
    };
    let url = format!("https://oidc.{}.amazonaws.com/token", s.oidc_region);
    let v = http_post_json(
        &url,
        &[("Content-Type", "application/json".into())],
        json!({
            "clientId": cid,
            "clientSecret": sec,
            "grantType": "refresh_token",
            "refreshToken": rt,
        }),
    )
    .await?;
    v.get("accessToken")
        .or_else(|| v.get("access_token"))
        .and_then(|x| x.as_str())
        .map(str::to_string)
        .ok_or_else(|| "kiro-cli login".into())
}

const KIRO_USAGE_TARGET: &str = "AmazonCodeWhispererService.GetUsageLimits";

pub(super) async fn get_usage(token: &str, s: &KiroSession) -> Result<Value, String> {
    let body = json!({
        "origin": "AI_EDITOR",
        "profileArn": s.profile_arn,
        "resourceType": "CREDIT",
        "isEmailRequired": true,
    });
    let hosts = [
        format!("https://q.{}.amazonaws.com/", s.api_region),
        format!("https://management.{}.kiro.dev/", s.api_region),
    ];
    let mut last = String::new();
    for host in hosts {
        match http_post_amz_json(&host, KIRO_USAGE_TARGET, token, body.clone()).await {
            Ok(v) => return Ok(v),
            Err(e) => last = e,
        }
    }
    Err(last)
}
