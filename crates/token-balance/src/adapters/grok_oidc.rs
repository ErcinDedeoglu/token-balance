use crate::accounts::Pointer;
use crate::credentials::Credentials;
use chrono::{DateTime, Duration, SecondsFormat, Utc};
use serde_json::{Value, json};
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;

use super::GROK_AUTH_KEYS;

pub struct GrokCliAuth {
    pub access: String,
    pub refresh: Option<String>,
    pub client_id: Option<String>,
    pub issuer: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

pub fn grok_http() -> Result<reqwest::Client, String> {
    reqwest::Client::builder()
        .user_agent("grok-shell")
        .use_native_tls()
        .http1_only()
        .build()
        .map_err(|e| e.to_string())
}

pub fn oidc_token_url(issuer: &str) -> String {
    format!("{}/oauth2/token", issuer.trim_end_matches('/'))
}

pub fn access_expired(auth: &GrokCliAuth) -> bool {
    match auth.expires_at {
        Some(t) => Utc::now() + Duration::seconds(60) >= t,
        None => false,
    }
}

fn nested_auth_obj(v: &Value) -> Option<&serde_json::Map<String, Value>> {
    let obj = v.as_object()?;
    if obj.contains_key("key") || obj.contains_key("refresh_token") {
        return Some(obj);
    }
    obj.values().find_map(|inner| {
        let m = inner.as_object()?;
        (m.contains_key("key") || m.contains_key("refresh_token")).then_some(m)
    })
}

fn nested_auth_obj_mut(v: &mut Value) -> Option<&mut serde_json::Map<String, Value>> {
    let obj = v.as_object_mut()?;
    if obj.contains_key("key") || obj.contains_key("refresh_token") {
        return Some(obj);
    }
    obj.values_mut().find_map(|inner| {
        let m = inner.as_object_mut()?;
        (m.contains_key("key") || m.contains_key("refresh_token")).then_some(m)
    })
}

pub fn parse_grok_auth(text: &str) -> Option<GrokCliAuth> {
    let v: Value = serde_json::from_str(text).ok()?;
    let e = nested_auth_obj(&v)?;
    let access = GROK_AUTH_KEYS
        .iter()
        .find_map(|k| e.get(*k).and_then(|x| x.as_str()))
        .filter(|s| !s.is_empty())?
        .to_string();
    Some(GrokCliAuth {
        access,
        refresh: e
            .get("refresh_token")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        client_id: e
            .get("oidc_client_id")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        issuer: e
            .get("oidc_issuer")
            .and_then(|x| x.as_str())
            .filter(|s| !s.is_empty())
            .map(str::to_string),
        expires_at: e.get("expires_at").and_then(|x| x.as_str()).and_then(|s| {
            DateTime::parse_from_rfc3339(s)
                .ok()
                .map(|d| d.with_timezone(&Utc))
        }),
    })
}

pub fn map_oidc_refresh(v: &Value) -> Result<(String, Option<String>, u64), String> {
    let access = v
        .get("access_token")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .ok_or_else(|| "oidc refresh: missing access_token".to_string())?
        .to_string();
    let refresh = v
        .get("refresh_token")
        .and_then(|x| x.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    let expires_in = v.get("expires_in").and_then(|x| x.as_u64()).unwrap_or(21600);
    Ok((access, refresh, expires_in))
}

fn patch_auth_json(root: &mut Value, access: &str, refresh: Option<&str>, expires_in: u64) {
    let Some(e) = nested_auth_obj_mut(root) else {
        return;
    };
    e.insert("key".into(), json!(access));
    if let Some(r) = refresh {
        e.insert("refresh_token".into(), json!(r));
    }
    let exp = (Utc::now() + Duration::seconds(expires_in as i64))
        .to_rfc3339_opts(SecondsFormat::Micros, true);
    e.insert("expires_at".into(), json!(exp));
}

fn write_auth_json(path: &Path, v: &Value) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    let data = serde_json::to_vec(v).map_err(|e| e.to_string())?;
    {
        let mut opts = OpenOptions::new();
        opts.write(true).create(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            opts.mode(0o600);
        }
        let mut f = opts.open(&tmp).map_err(|e| e.to_string())?;
        f.write_all(&data).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&tmp, path).map_err(|e| e.to_string())
}

pub fn persist_refreshed(
    creds: &Credentials,
    pointer: &Pointer,
    access: &str,
    refresh: Option<&str>,
    expires_in: u64,
) {
    let Pointer::File(rel) = pointer else {
        return;
    };
    let path = creds.resolve_path(rel);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(mut root) = serde_json::from_str::<Value>(&text) else {
        return;
    };
    patch_auth_json(&mut root, access, refresh, expires_in);
    let _ = write_auth_json(&path, &root);
}

pub async fn oidc_refresh(auth: &GrokCliAuth) -> Result<(String, Option<String>, u64), String> {
    let issuer = auth.issuer.as_deref().ok_or("oidc refresh: missing issuer")?;
    let client_id = auth
        .client_id
        .as_deref()
        .ok_or("oidc refresh: missing client_id")?;
    let refresh = auth
        .refresh
        .as_deref()
        .ok_or("oidc refresh: missing refresh_token")?;
    let resp = grok_http()?
        .post(oidc_token_url(issuer))
        .header("Accept", "application/json")
        .header("x-grok-client-surface", "cli")
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh),
            ("client_id", client_id),
        ])
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| e.to_string())?;
    if !status.is_success() {
        return Err(super::billing_http_error(status.as_u16()));
    }
    let v: Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    map_oidc_refresh(&v)
}

pub async fn refresh_and_store(
    creds: &Credentials,
    pointer: &Pointer,
    auth: &mut GrokCliAuth,
) -> Result<(), String> {
    let (access, refresh, expires_in) = oidc_refresh(auth).await?;
    persist_refreshed(creds, pointer, &access, refresh.as_deref(), expires_in);
    auth.access = access;
    if let Some(r) = refresh {
        auth.refresh = Some(r);
    }
    auth.expires_at = Some(Utc::now() + Duration::seconds(expires_in as i64));
    Ok(())
}
