use super::read_rate_limits;
use crate::adapters::codex::{CodexAdapter, map_codex_rate_limits};
use crate::domain::effective_available;
use crate::providers::Provider;
use serde_json::{Value, json};
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const USED_PERCENT: f64 = 82.0;

fn limits_result() -> Value {
    json!({
        "rateLimits": {
            "limitId": "codex",
            "planType": "plus",
            "primary": {
                "usedPercent": USED_PERCENT,
                "windowDurationMins": 300,
                "resetsAt": 1757948640
            },
            "secondary": {
                "usedPercent": 37,
                "windowDurationMins": 10080,
                "resetsAt": 1758301920
            }
        },
        "rateLimitsByLimitId": {
            "codex": {
                "limitId": "codex",
                "planType": "plus",
                "primary": {
                    "usedPercent": USED_PERCENT,
                    "windowDurationMins": 300,
                    "resetsAt": 1757948640
                },
                "secondary": {
                    "usedPercent": 37,
                    "windowDurationMins": 10080,
                    "resetsAt": 1758301920
                }
            }
        }
    })
}

async fn require_init_peer<R, W>(mut reader: R, mut writer: W, methods: Arc<Mutex<Vec<String>>>)
where
    R: tokio::io::AsyncBufRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut saw_initialize = false;
    let mut saw_initialized = false;
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {}
            Err(_) => break,
        }
        let Ok(msg) = serde_json::from_str::<Value>(line.trim()) else {
            continue;
        };
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        methods.lock().unwrap().push(method.to_string());
        let id = msg.get("id").cloned();
        match method {
            "initialize" => {
                saw_initialize = true;
                let reply = json!({
                    "id": id,
                    "result": { "userAgent": "fake-codex" }
                });
                let _ = writer.write_all(format!("{reply}\n").as_bytes()).await;
                let _ = writer.flush().await;
            }
            "initialized" => {
                saw_initialized = true;
            }
            "account/rateLimits/read" => {
                let reply = if saw_initialize && saw_initialized {
                    let _ = writer
                        .write_all(
                            format!(
                                "{}\n",
                                json!({"method":"account/rateLimits/updated","params":{}})
                            )
                            .as_bytes(),
                        )
                        .await;
                    json!({ "id": id, "result": limits_result() })
                } else {
                    json!({
                        "id": id,
                        "error": { "message": "Not initialized" }
                    })
                };
                let _ = writer.write_all(format!("{reply}\n").as_bytes()).await;
                let _ = writer.flush().await;
                break;
            }
            _ => {
                if let Some(id) = id {
                    if !saw_initialize {
                        let reply = json!({
                            "id": id,
                            "error": { "message": "Not initialized" }
                        });
                        let _ = writer.write_all(format!("{reply}\n").as_bytes()).await;
                        let _ = writer.flush().await;
                    }
                }
            }
        }
    }
}

#[tokio::test]
async fn handshake_then_rate_limits_against_peer_that_requires_initialize() {
    let (client_write, server_read) = tokio::io::duplex(64 * 1024);
    let (server_write, client_read) = tokio::io::duplex(64 * 1024);
    let methods = Arc::new(Mutex::new(Vec::new()));
    let peer_methods = Arc::clone(&methods);
    let peer = tokio::spawn(async move {
        require_init_peer(BufReader::new(server_read), server_write, peer_methods).await;
    });
    let rpc = read_rate_limits(BufReader::new(client_read), client_write)
        .await
        .expect("handshake rpc");
    let _ = peer.await;
    assert_eq!(
        methods.lock().unwrap().as_slice(),
        ["initialize", "initialized", "account/rateLimits/read"]
    );
    let status = map_codex_rate_limits(&rpc);
    let av = effective_available(&status).expect("available after handshake");
    let session = av
        .windows
        .iter()
        .find(|w| w.duration_mins == Some(300))
        .expect("session window");
    assert_eq!(session.remaining_percent, (100.0 - USED_PERCENT) as f32);
}

#[tokio::test]
async fn fetch_spawns_stdio_peer_that_requires_initialize() {
    let py = r#"
import json, sys
init = False
inited = False
for raw in sys.stdin:
    line = raw.strip()
    if not line:
        continue
    msg = json.loads(line)
    method = msg.get("method")
    mid = msg.get("id")
    if method == "initialize":
        init = True
        sys.stdout.write(json.dumps({"id": mid, "result": {"userAgent": "fake"}}) + "\n")
        sys.stdout.flush()
    elif method == "initialized":
        inited = True
    elif method == "account/rateLimits/read":
        if not (init and inited):
            sys.stdout.write(json.dumps({"id": mid, "error": {"message": "Not initialized"}}) + "\n")
            sys.stdout.flush()
            continue
        sys.stdout.write(json.dumps({"method": "account/rateLimits/updated", "params": {}}) + "\n")
        result = {
            "rateLimits": {
                "primary": {"usedPercent": 82.0, "windowDurationMins": 300, "resetsAt": 1757948640},
                "secondary": {"usedPercent": 37, "windowDurationMins": 10080, "resetsAt": 1758301920}
            }
        }
        sys.stdout.write(json.dumps({"id": mid, "result": result}) + "\n")
        sys.stdout.flush()
        break
    elif mid is not None and not init:
        sys.stdout.write(json.dumps({"id": mid, "error": {"message": "Not initialized"}}) + "\n")
        sys.stdout.flush()
"#;
    let program = if std::path::Path::new("/usr/bin/python3").is_file() {
        "/usr/bin/python3"
    } else {
        "python3"
    };
    let adapter = CodexAdapter::with_command(program.into(), vec!["-c".into(), py.into()]);
    let status = adapter.fetch().await;
    let av = effective_available(&status).unwrap_or_else(|| panic!("{status:?}"));
    let session = av
        .windows
        .iter()
        .find(|w| w.duration_mins == Some(300))
        .expect("session");
    assert_eq!(session.remaining_percent, 18.0);
}

#[tokio::test]
async fn pre_init_rate_limits_is_not_initialized() {
    let (mut client_write, server_read) = tokio::io::duplex(64 * 1024);
    let (server_write, client_read) = tokio::io::duplex(64 * 1024);
    let methods = Arc::new(Mutex::new(Vec::new()));
    let peer_methods = Arc::clone(&methods);
    let peer = tokio::spawn(async move {
        require_init_peer(BufReader::new(server_read), server_write, peer_methods).await;
    });
    client_write
        .write_all(b"{\"id\":2,\"method\":\"account/rateLimits/read\"}\n")
        .await
        .unwrap();
    let _ = client_write.flush().await;
    let mut reader = BufReader::new(client_read);
    let mut line = String::new();
    reader.read_line(&mut line).await.unwrap();
    let v: Value = serde_json::from_str(line.trim()).unwrap();
    assert_eq!(v["error"]["message"].as_str(), Some("Not initialized"));
    drop(client_write);
    let _ = peer.await;
}
