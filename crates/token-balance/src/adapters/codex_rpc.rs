use serde_json::{Value, json};
use tokio::io::{AsyncBufRead, AsyncBufReadExt, AsyncWrite, AsyncWriteExt};

const INIT_ID: u64 = 1;
const LIMITS_ID: u64 = 2;

/// Line-oriented app-server JSON-RPC (no `"jsonrpc"` field). Handshake is
/// `initialize` → wait for that id → `initialized` notify → `account/rateLimits/read`
/// and wait for that id, skipping notifications.
pub async fn read_rate_limits<R, W>(mut stdout: R, mut stdin: W) -> Result<Value, String>
where
    R: AsyncBufRead + Unpin,
    W: AsyncWrite + Unpin,
{
    write_msg(
        &mut stdin,
        json!({
            "method": "initialize",
            "id": INIT_ID,
            "params": {
                "clientInfo": {
                    "name": "token-balance",
                    "title": "token-balance",
                    "version": env!("CARGO_PKG_VERSION"),
                }
            }
        }),
    )
    .await?;
    let _ = wait_id(&mut stdout, INIT_ID).await?;
    write_msg(
        &mut stdin,
        json!({
            "method": "initialized",
            "params": {}
        }),
    )
    .await?;
    write_msg(
        &mut stdin,
        json!({
            "method": "account/rateLimits/read",
            "id": LIMITS_ID
        }),
    )
    .await?;
    wait_id(&mut stdout, LIMITS_ID).await
}

async fn write_msg<W: AsyncWrite + Unpin>(w: &mut W, v: Value) -> Result<(), String> {
    let mut line = serde_json::to_string(&v).map_err(|e| e.to_string())?;
    line.push('\n');
    w.write_all(line.as_bytes())
        .await
        .map_err(|e| e.to_string())?;
    w.flush().await.map_err(|e| e.to_string())
}

fn msg_id(v: &Value) -> Option<u64> {
    let id = v.get("id")?;
    id.as_u64()
        .or_else(|| id.as_i64().and_then(|i| u64::try_from(i).ok()))
}

async fn wait_id<R: AsyncBufRead + Unpin>(r: &mut R, want: u64) -> Result<Value, String> {
    loop {
        let mut line = String::new();
        let n = r.read_line(&mut line).await.map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("app-server closed".into());
        }
        let v: Value = serde_json::from_str(line.trim()).map_err(|e| e.to_string())?;
        if msg_id(&v) != Some(want) {
            continue;
        }
        if let Some(err) = v.get("error") {
            let msg = err
                .get("message")
                .and_then(|m| m.as_str())
                .unwrap_or("rpc error");
            return Err(msg.to_string());
        }
        return Ok(v);
    }
}

#[cfg(test)]
#[path = "codex_rpc_test.rs"]
mod tests;
