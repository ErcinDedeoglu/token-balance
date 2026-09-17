use super::{EXA_CREDITS_URL, dashboard_http_error};
use serde_json::Value;
use std::time::Duration;

fn chrome_mcp_bin() -> std::path::PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let p = std::path::PathBuf::from(home).join(".local/bin/chrome-mcp");
        if p.is_file() {
            return p;
        }
    }
    std::path::PathBuf::from("chrome-mcp")
}

async fn chrome_mcp(args: &[&str]) -> Result<Value, String> {
    let mut cmd = tokio::process::Command::new(chrome_mcp_bin());
    cmd.arg("--timeout").arg("12").args(args).kill_on_drop(true);
    let out = tokio::time::timeout(Duration::from_secs(14), cmd.output())
        .await
        .map_err(|_| "chrome-mcp timed out".to_string())?
        .map_err(|e| format!("chrome-mcp: {e}"))?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "chrome-mcp exit {}: {err}",
            out.status.code().unwrap_or(-1)
        ));
    }
    serde_json::from_slice(&out.stdout).map_err(|e| format!("chrome-mcp json: {e}"))
}

fn json_tab_id(v: &Value) -> Option<u64> {
    v.get("id")
        .and_then(|id| id.as_u64().or_else(|| id.as_i64().map(|i| i as u64)))
        .or_else(|| {
            v.get("tab").and_then(|t| {
                t.get("id")
                    .and_then(|id| id.as_u64().or_else(|| id.as_i64().map(|i| i as u64)))
            })
        })
}

async fn dashboard_tab_id() -> Result<u64, String> {
    let tabs = chrome_mcp(&["tabs_list"]).await?;
    let arr = tabs.as_array().ok_or("chrome-mcp tabs_list: not an array")?;
    for t in arr {
        let url = t.get("url").and_then(|u| u.as_str()).unwrap_or("");
        if url.contains("dashboard.exa.ai") {
            return json_tab_id(t).ok_or_else(|| "chrome-mcp tab missing id".into());
        }
    }
    let created = chrome_mcp(&[
        "tabs_create",
        "--args",
        r#"{"url":"https://dashboard.exa.ai/billing","active":false,"timeout":12000}"#,
    ])
    .await?;
    json_tab_id(&created).ok_or_else(|| "chrome-mcp tabs_create: no tab id".into())
}

pub async fn get_credits_chrome() -> Result<Value, String> {
    let tab_id = dashboard_tab_id().await?;
    let args = format!(
        r#"{{"url":"{EXA_CREDITS_URL}","method":"GET","responseType":"json","tabId":{tab_id}}}"#
    );
    let v = chrome_mcp(&["http_request", "--args", &args]).await?;
    let status = v.get("status").and_then(|s| s.as_u64()).unwrap_or(0);
    if status != 200 {
        return Err(dashboard_http_error(status as u16));
    }
    v.get("data")
        .cloned()
        .ok_or_else(|| "chrome-mcp get-credits: no data".into())
}

#[cfg(test)]
mod tests {
    use super::json_tab_id;
    use serde_json::json;

    #[test]
    fn json_tab_id_reads_id_or_nested_tab() {
        assert_eq!(json_tab_id(&json!({"id": 2134390653})), Some(2134390653));
        assert_eq!(
            json_tab_id(&json!({"tab": {"id": 99, "url": "https://dashboard.exa.ai/billing"}})),
            Some(99)
        );
        assert_eq!(json_tab_id(&json!({})), None);
    }
}
